use crate::model::{CommentInput, Error, Result, VisualInput};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};

pub struct Store {
    pub connection: Mutex<Connection>,
    pub admin: String,
    pub changed: tokio::sync::watch::Sender<()>,
    viewers: Mutex<HashMap<String, ViewerPresence>>,
}

struct ViewerPresence {
    seen: Instant,
    desktop: bool,
}

impl Store {
    pub fn open(directory: &Path) -> anyhow::Result<Self> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(directory)?;
        let credential = directory.join("admin.token");
        let admin = match fs::read_to_string(&credential) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let token = uuid::Uuid::new_v4().to_string();
                let temporary = directory.join(format!(".admin-{}", uuid::Uuid::new_v4()));
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&temporary)?;
                file.write_all(token.as_bytes())?;
                file.sync_all()?;
                let published = fs::hard_link(&temporary, &credential);
                fs::remove_file(&temporary)?;
                published?;
                token
            }
            Err(error) => return Err(error.into()),
        };
        anyhow::ensure!(
            uuid::Uuid::parse_str(&admin).is_ok(),
            "Owner credential is incomplete or invalid; restore admin.token before starting the companion"
        );
        let connection = Connection::open(directory.join("companion.sqlite3"))?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS workspaces(id TEXT PRIMARY KEY, title TEXT NOT NULL, token TEXT NOT NULL UNIQUE);
            CREATE TABLE IF NOT EXISTS visuals(id TEXT PRIMARY KEY, workspace TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE, title TEXT NOT NULL, latest INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS revisions(sequence INTEGER PRIMARY KEY AUTOINCREMENT, visual TEXT NOT NULL REFERENCES visuals(id) ON DELETE CASCADE, revision INTEGER NOT NULL, title TEXT NOT NULL, format TEXT NOT NULL, content TEXT NOT NULL, description TEXT NOT NULL, render_status TEXT NOT NULL DEFAULT 'accepted', render_error TEXT, UNIQUE(visual,revision));
            CREATE TABLE IF NOT EXISTS comments(id INTEGER PRIMARY KEY AUTOINCREMENT, visual TEXT NOT NULL, revision INTEGER NOT NULL, text TEXT NOT NULL, FOREIGN KEY(visual,revision) REFERENCES revisions(visual,revision) ON DELETE CASCADE);")?;
        Ok(Self {
            connection: Mutex::new(connection),
            admin,
            changed: tokio::sync::watch::channel(()).0,
            viewers: Mutex::new(HashMap::new()),
        })
    }

    pub fn scope(&self, token: &str) -> Result<Option<String>> {
        if token.is_empty() {
            return Err(Error(
                axum::http::StatusCode::UNAUTHORIZED,
                "A companion connection is required".into(),
            ));
        }
        if token == self.admin {
            return Ok(None);
        }
        self.connection
            .lock()
            .unwrap()
            .query_row("SELECT id FROM workspaces WHERE token=?", [token], |row| {
                row.get(0)
            })
            .optional()?
            .map(Some)
            .ok_or_else(|| {
                Error(
                    axum::http::StatusCode::UNAUTHORIZED,
                    "A valid companion connection is required".into(),
                )
            })
    }

    pub fn create_workspace(&self, title: &str) -> Result<Value> {
        if title.trim().is_empty() || title.len() > 200 {
            return Err(Error::bad("Workspace title must contain 1–200 bytes"));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let token = uuid::Uuid::new_v4().to_string();
        self.connection.lock().unwrap().execute(
            "INSERT INTO workspaces VALUES(?,?,?)",
            params![id, title, token],
        )?;
        Ok(json!({"workspace":id,"title":title,"token":token}))
    }

    pub fn publish(&self, workspace: &str, input: VisualInput) -> Result<Value> {
        if input.title.trim().is_empty() || input.title.len() > 200 {
            return Err(Error::bad("Visual title must contain 1–200 bytes"));
        }
        if input.content.is_empty() || input.content.len() > 12 * 1024 * 1024 {
            return Err(Error::bad("Content must contain 1 byte to 12 MiB"));
        }
        if input.id.is_some() != input.expected_revision.is_some() {
            return Err(Error::bad(
                "Replacement requires both id and expected_revision",
            ));
        }
        let mut connection = self.connection.lock().unwrap();
        let transaction = connection.transaction()?;
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id=?)",
            [workspace],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(Error::missing());
        }
        let (id, revision) = if let Some(id) = input.id {
            let current: i64 = transaction
                .query_row(
                    "SELECT latest FROM visuals WHERE id=? AND workspace=?",
                    params![id, workspace],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or_else(Error::missing)?;
            if input.expected_revision != Some(current) {
                return Err(Error(
                    axum::http::StatusCode::CONFLICT,
                    format!(
                        "Revision conflict: current revision is {current}; inspect it before replacing"
                    ),
                ));
            }
            transaction.execute(
                "UPDATE visuals SET title=?,latest=? WHERE id=?",
                params![input.title, current + 1, id],
            )?;
            (id, current + 1)
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            transaction.execute(
                "INSERT INTO visuals VALUES(?,?,?,1)",
                params![id, workspace, input.title],
            )?;
            (id, 1)
        };
        transaction.execute("INSERT INTO revisions(visual,revision,title,format,content,description) VALUES(?,?,?,?,?,?)", params![id,revision,input.title,input.format.name(),input.content,input.description])?;
        transaction.commit()?;
        Ok(
            json!({"id":id,"revision":revision,"presentation":{"viewer_connected":false,"render_status":"accepted","error":null}}),
        )
    }

    pub fn revision(&self, workspace: &str, id: &str, revision: i64) -> Result<Value> {
        self.connection.lock().unwrap().query_row("SELECT r.title,r.format,r.content,r.description,r.render_status,r.render_error FROM revisions r JOIN visuals v ON v.id=r.visual WHERE v.workspace=? AND v.id=? AND r.revision=?", params![workspace,id,revision], |row| {
            Ok(json!({"id":id,"revision":revision,"title":row.get::<_,String>(0)?,"format":row.get::<_,String>(1)?,"content":row.get::<_,String>(2)?,"description":row.get::<_,String>(3)?,"render_status":row.get::<_,String>(4)?,"error":row.get::<_,Option<String>>(5)?}))
        }).optional()?.ok_or_else(Error::missing)
    }

    pub fn comment(&self, scope: Option<&str>, input: CommentInput) -> Result<Value> {
        if input.text.trim().is_empty() || input.text.len() > 16_384 {
            return Err(Error::bad("Comment must contain 1–16384 bytes"));
        }
        let connection = self.connection.lock().unwrap();
        let workspace: String = connection.query_row("SELECT v.workspace FROM visuals v JOIN revisions r ON v.id=r.visual WHERE v.id=? AND r.revision=?",params![input.visual,input.revision],|row|row.get(0)).optional()?.ok_or_else(Error::missing)?;
        if scope.is_some_and(|id| id != workspace) {
            return Err(Error::missing());
        }
        connection.execute(
            "INSERT INTO comments(visual,revision,text) VALUES(?,?,?)",
            params![input.visual, input.revision, input.text],
        )?;
        let id = connection.last_insert_rowid();
        self.changed.send_replace(());
        Ok(json!({"id":id,"visual":input.visual,"revision":input.revision,"text":input.text}))
    }

    pub fn feedback(&self, workspace: &str, after: i64) -> Result<Value> {
        if after < 0 {
            return Err(Error::bad("Feedback cursor cannot be negative"));
        }
        let connection = self.connection.lock().unwrap();
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id=?)",
            [workspace],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(Error::missing());
        }
        let mut statement = connection.prepare("SELECT c.id,c.visual,c.revision,c.text FROM comments c JOIN visuals v ON v.id=c.visual WHERE v.workspace=? AND c.id>? ORDER BY c.id LIMIT 100")?;
        let comments = statement.query_map(params![workspace,after],|row|Ok(json!({"id":row.get::<_,i64>(0)?,"visual":row.get::<_,String>(1)?,"revision":row.get::<_,i64>(2)?,"text":row.get::<_,String>(3)?})))?.collect::<std::result::Result<Vec<Value>,_>>()?;
        let cursor = comments
            .last()
            .and_then(|v| v["id"].as_i64())
            .unwrap_or(after);
        Ok(json!({"comments":comments,"cursor":cursor}))
    }

    pub fn workspaces(&self, scope: Option<&str>) -> Result<Value> {
        let connection = self.connection.lock().unwrap();
        let mut query = connection.prepare("SELECT w.id,w.title,COALESCE(MAX(r.sequence),0) FROM workspaces w LEFT JOIN visuals v ON v.workspace=w.id LEFT JOIN revisions r ON r.visual=v.id WHERE ? IS NULL OR w.id=? GROUP BY w.id ORDER BY w.rowid")?;
        let values = query.query_map(params![scope,scope],|row|Ok(json!({"id":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,"sequence":row.get::<_,i64>(2)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(json!(values))
    }

    pub fn visuals(&self, workspace: &str) -> Result<Value> {
        let connection = self.connection.lock().unwrap();
        let mut query = connection.prepare("SELECT v.id,v.title,v.latest,MAX(r.sequence) FROM visuals v JOIN revisions r ON r.visual=v.id WHERE v.workspace=? GROUP BY v.id ORDER BY MAX(r.sequence) DESC")?;
        let values = query.query_map([workspace],|row|Ok(json!({"id":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,"revision":row.get::<_,i64>(2)?,"sequence":row.get::<_,i64>(3)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(json!(values))
    }

    pub fn history(&self, workspace: &str, id: &str) -> Result<Value> {
        let connection = self.connection.lock().unwrap();
        let mut query = connection.prepare("SELECT r.revision,r.title,r.format FROM revisions r JOIN visuals v ON v.id=r.visual WHERE v.workspace=? AND v.id=? ORDER BY r.revision DESC")?;
        let values = query.query_map(params![workspace,id],|row|Ok(json!({"revision":row.get::<_,i64>(0)?,"title":row.get::<_,String>(1)?,"format":row.get::<_,String>(2)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        if values.is_empty() {
            return Err(Error::missing());
        }
        Ok(json!(values))
    }

    pub fn delete_visual(&self, workspace: &str, id: &str) -> Result<()> {
        if self.connection.lock().unwrap().execute(
            "DELETE FROM visuals WHERE workspace=? AND id=?",
            params![workspace, id],
        )? == 0
        {
            return Err(Error::missing());
        }
        self.changed.send_replace(());
        Ok(())
    }

    pub fn delete_workspace(&self, workspace: &str) -> Result<()> {
        if self
            .connection
            .lock()
            .unwrap()
            .execute("DELETE FROM workspaces WHERE id=?", [workspace])?
            == 0
        {
            return Err(Error::missing());
        }
        self.changed.send_replace(());
        Ok(())
    }

    pub fn viewer(&self, id: &str, connected: bool, desktop: bool) -> Result<()> {
        if id.is_empty() || id.len() > 100 {
            return Err(Error::bad("Viewer ID must contain 1–100 bytes"));
        }
        let mut viewers = self.viewers.lock().unwrap();
        viewers.retain(|_, presence| presence.seen.elapsed() < Duration::from_secs(5));
        if connected {
            viewers.insert(
                id.into(),
                ViewerPresence {
                    seen: Instant::now(),
                    desktop,
                },
            );
        } else {
            viewers.remove(id);
        }
        Ok(())
    }

    pub fn viewer_connected(&self) -> bool {
        self.viewers
            .lock()
            .unwrap()
            .values()
            .any(|presence| presence.seen.elapsed() < Duration::from_secs(5))
    }

    pub fn desktop_connected(&self) -> bool {
        self.viewers
            .lock()
            .unwrap()
            .values()
            .any(|presence| presence.desktop && presence.seen.elapsed() < Duration::from_secs(5))
    }

    pub fn presentation(&self, workspace: &str, id: &str, revision: i64) -> Result<Value> {
        let value = self.revision(workspace, id, revision)?;
        Ok(
            json!({"viewer_connected":self.viewer_connected(),"render_status":value["render_status"],"error":value["error"]}),
        )
    }

    pub fn render(&self, id: &str, revision: i64, status: &str, error: Option<&str>) -> Result<()> {
        if !matches!(status, "rendered" | "failed") || error.is_some_and(|s| s.len() > 4096) {
            return Err(Error::bad("Invalid rendering acknowledgment"));
        }
        if self.connection.lock().unwrap().execute(
            "UPDATE revisions SET render_status=?,render_error=? WHERE visual=? AND revision=?",
            params![status, error, id, revision],
        )? == 0
        {
            return Err(Error::missing());
        }
        Ok(())
    }
}
