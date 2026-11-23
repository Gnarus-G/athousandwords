use reqwest::{Client, StatusCode};
use serde_json::{Value, json};

struct Companion {
    directory: tempfile::TempDir,
    url: String,
    admin: String,
    client: Client,
    task: tokio::task::JoinHandle<()>,
}
impl Companion {
    async fn start() -> Self {
        Self::start_with_launcher(None).await
    }
    async fn start_with_launcher(launcher: Option<athousandwords::viewer::ViewerLauncher>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let app =
            athousandwords::server::app_with_viewer_launcher(directory.path(), launcher).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let admin = std::fs::read_to_string(directory.path().join("admin.token")).unwrap();
        Self {
            directory,
            url,
            admin,
            client: Client::new(),
            task,
        }
    }
    async fn workspace(&self, title: &str) -> Value {
        let response = self
            .client
            .post(format!("{}/api/workspaces", self.url))
            .bearer_auth(&self.admin)
            .json(&json!({"title":title}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        response.json().await.unwrap()
    }
    async fn publish(&self, token: &str, input: Value) -> reqwest::Response {
        self.client
            .post(format!("{}/api/visuals", self.url))
            .bearer_auth(token)
            .json(&input)
            .send()
            .await
            .unwrap()
    }
    async fn restart(&mut self) {
        self.task.abort();
        let app = athousandwords::server::app(self.directory.path()).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        self.url = format!("http://{}", listener.local_addr().unwrap());
        self.task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    }
}

#[tokio::test]
async fn publishing_launches_the_desktop_once_then_reuses_it_and_reopens_after_close() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let launches = Arc::new(AtomicUsize::new(0));
    let count = launches.clone();
    let launcher = athousandwords::viewer::ViewerLauncher::new(
        "http://127.0.0.1:7331".into(),
        move |url, workspace| {
            assert_eq!(url, "http://127.0.0.1:7331");
            assert!(uuid::Uuid::parse_str(workspace).is_ok());
            count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
    );
    let c = Companion::start_with_launcher(Some(launcher)).await;
    let w = c.workspace("Automatically visible").await;
    let token = w["token"].as_str().unwrap();
    let input = json!({"title":"Show this","format":"svg","content":"<svg/>"});
    let first: Value = c.publish(token, input.clone()).await.json().await.unwrap();
    assert_eq!(
        launches.load(Ordering::SeqCst),
        1,
        "The first publication must launch the app on the companion's desktop"
    );
    assert_eq!(first["presentation"]["opening"]["status"], "requested");
    c.publish(token, input.clone()).await;
    assert_eq!(
        launches.load(Ordering::SeqCst),
        1,
        "A pending app launch must not create duplicate windows"
    );
    c.client
        .post(format!("{}/api/viewer", c.url))
        .bearer_auth(&c.admin)
        .json(&json!({"viewer":"desktop","desktop":true}))
        .send()
        .await
        .unwrap();
    c.publish(token, input.clone()).await;
    assert_eq!(
        launches.load(Ordering::SeqCst),
        1,
        "Publishing while the desktop is connected must reuse it"
    );
    c.client
        .delete(format!("{}/api/viewer", c.url))
        .bearer_auth(&c.admin)
        .json(&json!({"viewer":"desktop","desktop":true}))
        .send()
        .await
        .unwrap();
    c.publish(token, input).await;
    assert_eq!(
        launches.load(Ordering::SeqCst),
        2,
        "Publishing after closing the app must reopen it"
    );
}

#[tokio::test]
async fn a_legacy_browser_tab_cannot_suppress_the_app_or_override_its_render_evidence() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let launches = Arc::new(AtomicUsize::new(0));
    let count = launches.clone();
    let launcher =
        athousandwords::viewer::ViewerLauncher::new("http://127.0.0.1:7331".into(), move |_, _| {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
    let c = Companion::start_with_launcher(Some(launcher)).await;
    let w = c.workspace("Desktop migration").await;
    let token = w["token"].as_str().unwrap();
    c.client
        .post(format!("{}/api/viewer", c.url))
        .bearer_auth(&c.admin)
        .json(&json!({"viewer":"legacy-browser"}))
        .send()
        .await
        .unwrap();
    let visual: Value = c
        .publish(
            token,
            json!({"title":"Move to the app","format":"svg","content":"<svg/>"}),
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(
        launches.load(Ordering::SeqCst),
        1,
        "An old browser tab is not the standalone app"
    );
    assert_eq!(
        visual["presentation"]["viewer_connected"], false,
        "Desktop mode must not report an old browser as its app"
    );
    let id = visual["id"].as_str().unwrap();
    let ack = |desktop: bool, status: &str| json!({"visual":id,"revision":1,"status":status,"desktop":desktop,"error":null});
    c.client
        .post(format!("{}/api/render", c.url))
        .bearer_auth(&c.admin)
        .json(&ack(false, "failed"))
        .send()
        .await
        .unwrap();
    let status_url = format!("{}/api/status/{id}/1", c.url);
    let status: Value = c
        .client
        .get(&status_url)
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status["render_status"], "accepted");
    c.client
        .post(format!("{}/api/render", c.url))
        .bearer_auth(&c.admin)
        .json(&ack(true, "rendered"))
        .send()
        .await
        .unwrap();
    c.client
        .post(format!("{}/api/render", c.url))
        .bearer_auth(&c.admin)
        .json(&ack(false, "failed"))
        .send()
        .await
        .unwrap();
    let status: Value = c
        .client
        .get(&status_url)
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status["render_status"], "rendered");
}

#[tokio::test]
async fn a_failed_desktop_launch_preserves_the_visual_and_reports_a_safe_error() {
    let launcher =
        athousandwords::viewer::ViewerLauncher::new("http://127.0.0.1:7331".into(), |_, _| {
            Err(std::io::Error::other("private launcher diagnostics"))
        });
    let c = Companion::start_with_launcher(Some(launcher)).await;
    let w = c.workspace("Launch failure").await;
    let token = w["token"].as_str().unwrap();
    let response = c
        .publish(
            token,
            json!({"title":"Keep this","format":"svg","content":"<svg/>"}),
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let visual: Value = response.json().await.unwrap();
    assert_eq!(visual["presentation"]["opening"]["status"], "failed");
    assert!(!visual.to_string().contains("private launcher diagnostics"));
    let response = c
        .client
        .get(format!(
            "{}/api/visuals/{}/revisions/1",
            c.url,
            visual["id"].as_str().unwrap()
        ))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
impl Drop for Companion {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[test]
fn an_incomplete_owner_credential_fails_closed_on_restart() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("admin.token"), "").unwrap();
    assert!(
        athousandwords::server::app(directory.path()).is_err(),
        "An incomplete credential must not authorize unauthenticated requests"
    );
}

#[tokio::test]
async fn an_unwritable_connection_destination_does_not_create_an_unpaired_workspace() {
    let c = Companion::start().await;
    let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_atw"))
        .arg("--data-dir")
        .arg(c.directory.path())
        .args([
            "--url",
            &c.url,
            "workspace",
            "create",
            "--title",
            "No orphan",
            "--out",
        ])
        .arg(c.directory.path().join("missing-parent/connection.json"))
        .output()
        .await
        .unwrap();
    assert!(!output.status.success());
    let workspaces: Value = c
        .client
        .get(format!("{}/api/workspaces", c.url))
        .bearer_auth(&c.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(workspaces, json!([]));
}

#[tokio::test]
async fn stale_owner_requests_report_missing_workspaces_and_cross_origin_requests_are_refused() {
    let c = Companion::start().await;
    let w = c.workspace("Deleted selection").await;
    c.client
        .delete(format!("{}/api/workspace", c.url))
        .bearer_auth(w["token"].as_str().unwrap())
        .send()
        .await
        .unwrap();
    let response = c
        .client
        .post(format!("{}/api/visuals", c.url))
        .bearer_auth(&c.admin)
        .header("x-atw-workspace", w["workspace"].as_str().unwrap())
        .json(&json!({"title":"Stale","format":"svg","content":"<svg/>"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        c.client
            .get(format!("{}/api/workspaces", c.url))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        c.client
            .get(format!("{}/api/workspaces", c.url))
            .bearer_auth(&c.admin)
            .header("Origin", "null")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        c.client
            .get(format!("{}/api/workspaces", c.url))
            .bearer_auth(&c.admin)
            .header("Host", "outside.example")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn a_scoped_agent_publishes_a_visual_and_reads_it_back() {
    let directory = tempfile::tempdir().unwrap();
    let app = athousandwords::server::app(directory.path()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = Client::new();
    let url = format!("http://{address}");
    // The test supplies its own local companion credential, not a user secret.
    let admin = std::fs::read_to_string(directory.path().join("admin.token")).unwrap();
    let response = client
        .post(format!("{url}/api/workspaces"))
        .bearer_auth(admin.trim())
        .json(&json!({"title":"Architecture"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let connection: Value = response.json().await.unwrap();
    let agent = connection["token"].as_str().unwrap();
    let response = client.post(format!("{url}/api/visuals"))
        .bearer_auth(agent).json(&json!({
            "title":"The communication loop", "format":"svg", "content":"<svg xmlns='http://www.w3.org/2000/svg'/>"
        })).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let visual: Value = response.json().await.unwrap();
    assert_eq!(visual["revision"], 1);
    assert_eq!(visual["presentation"]["viewer_connected"], false);
    assert_eq!(visual["presentation"]["render_status"], "accepted");
    let response = client
        .get(format!(
            "{url}/api/visuals/{}/revisions/1",
            visual["id"].as_str().unwrap()
        ))
        .bearer_auth(agent)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let revision: Value = response.json().await.unwrap();
    assert_eq!(
        revision["content"],
        "<svg xmlns='http://www.w3.org/2000/svg'/>"
    );
    task.abort();
}

#[tokio::test]
async fn revisions_are_retained_stale_updates_conflict_and_workspaces_are_isolated() {
    let companion = Companion::start().await;
    let a = companion.workspace("A").await;
    let b = companion.workspace("B").await;
    let token = a["token"].as_str().unwrap();
    let input = json!({"title":"Flow","format":"mermaid","content":"graph LR; A-->B"});
    let first: Value = companion
        .publish(token, input.clone())
        .await
        .json()
        .await
        .unwrap();
    let mut replacement = input;
    replacement["id"] = first["id"].clone();
    replacement["expected_revision"] = json!(1);
    replacement["content"] = json!("graph LR; A-->C");
    let response = companion.publish(token, replacement.clone()).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.json::<Value>().await.unwrap()["revision"], 2);
    assert_eq!(
        companion.publish(token, replacement.clone()).await.status(),
        StatusCode::CONFLICT
    );
    let path = format!(
        "{}/api/visuals/{}/revisions/1",
        companion.url,
        first["id"].as_str().unwrap()
    );
    let old: Value = companion
        .client
        .get(&path)
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(old["content"], "graph LR; A-->B");
    assert_eq!(
        companion
            .client
            .get(&path)
            .bearer_auth(b["token"].as_str().unwrap())
            .header("x-atw-workspace", a["workspace"].as_str().unwrap())
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    replacement["expected_revision"] = json!(2);
    assert_eq!(
        companion
            .publish(b["token"].as_str().unwrap(), replacement)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn revision_linked_comments_survive_restart_and_resume_without_destructive_reads() {
    let mut companion = Companion::start().await;
    let workspace = companion.workspace("Review").await;
    let token = workspace["token"].as_str().unwrap();
    let first: Value = companion
        .publish(
            token,
            json!({"title":"Flow","format":"html","content":"<p>First</p>"}),
        )
        .await
        .json()
        .await
        .unwrap();
    let id = first["id"].as_str().unwrap();
    let post_comment = |url: String, client: Client, admin: String| async move {
        client
            .post(url)
            .bearer_auth(admin)
            .json(&json!({"visual":id,"revision":1,"text":"Explain this relationship"}))
            .send()
            .await
            .unwrap()
    };
    // A comment submitted after a replacement still belongs to the earlier revision.
    companion.publish(token,json!({"id":id,"expected_revision":1,"title":"Flow","format":"html","content":"<p>Second</p>"})).await;
    let comment_url = format!("{}/api/comments", companion.url);
    let response = post_comment(
        comment_url.clone(),
        companion.client.clone(),
        companion.admin.clone(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let feedback_url = format!("{}/api/feedback?after=0", companion.url);
    let first_read: Value = companion
        .client
        .get(&feedback_url)
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(first_read["comments"][0]["revision"], 1);
    let cursor = first_read["cursor"].as_i64().unwrap();
    let again: Value = companion
        .client
        .get(&feedback_url)
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(again, first_read);
    let interrupted = tokio::spawn(
        companion
            .client
            .get(format!(
                "{}/api/feedback?after={cursor}&wait_ms=30000",
                companion.url
            ))
            .bearer_auth(token)
            .send(),
    );
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    interrupted.abort();
    let wait = companion
        .client
        .get(format!(
            "{}/api/feedback?after={cursor}&wait_ms=1000",
            companion.url
        ))
        .bearer_auth(token)
        .send();
    let add = async {
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        post_comment(
            comment_url,
            companion.client.clone(),
            companion.admin.clone(),
        )
        .await
    };
    let (waited, added) = tokio::join!(wait, add);
    assert_eq!(added.status(), StatusCode::CREATED);
    let waited: Value = waited.unwrap().json().await.unwrap();
    assert_eq!(waited["comments"].as_array().unwrap().len(), 1);
    assert!(waited["cursor"].as_i64().unwrap() > cursor);
    // Open the persisted companion again at a new listener, retaining the connection.
    companion.restart().await;
    let persisted: Value = companion
        .client
        .get(format!("{}/api/feedback?after=0", companion.url))
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(persisted["comments"].as_array().unwrap().len(), 2);
    let resumed: Value = companion
        .client
        .get(format!("{}/api/feedback?after={cursor}", companion.url))
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resumed["comments"].as_array().unwrap().len(), 1);
    assert_eq!(resumed["cursor"], persisted["cursor"]);
    let stored: Value = companion
        .client
        .get(format!("{}/api/visuals/{id}/revisions/2", companion.url))
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stored["content"], "<p>Second</p>");
    let timeout: Value = companion
        .client
        .get(format!(
            "{}/api/feedback?after={}&wait_ms=10",
            companion.url, persisted["cursor"]
        ))
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(timeout["comments"], json!([]));
    assert_eq!(timeout["cursor"], persisted["cursor"]);
}

#[tokio::test]
async fn viewer_presence_is_not_render_evidence_and_deletion_removes_history_and_feedback() {
    let mut c = Companion::start().await;
    let w = c.workspace("Display").await;
    let token = w["token"].as_str().unwrap();
    let visual: Value = c
        .publish(
            token,
            json!({"title":"Image","format":"svg","content":"<svg/>"}),
        )
        .await
        .json()
        .await
        .unwrap();
    let id = visual["id"].as_str().unwrap();
    let list = c
        .client
        .get(format!("{}/api/visuals", c.url))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);
    let response = c
        .client
        .post(format!("{}/api/viewer", c.url))
        .bearer_auth(&c.admin)
        .json(&json!({"viewer":"test-viewer"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let status_url = format!("{}/api/status/{id}/1", c.url);
    let status: Value = c
        .client
        .get(&status_url)
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status["viewer_connected"], true);
    assert_eq!(status["render_status"], "accepted");
    let ack_url = format!("{}/api/render", c.url);
    let ack = json!({"visual":id,"revision":1,"status":"failed","error":"Invalid SVG"});
    assert_eq!(
        c.client
            .post(&ack_url)
            .bearer_auth(token)
            .json(&ack)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        c.client
            .post(&ack_url)
            .bearer_auth(&c.admin)
            .json(&ack)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let status: Value = c
        .client
        .get(&status_url)
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status["render_status"], "failed");
    assert_eq!(status["error"], "Invalid SVG");
    c.client
        .post(format!("{}/api/comments", c.url))
        .bearer_auth(token)
        .json(&json!({"visual":id,"revision":1,"text":"Fix it"}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        c.client
            .delete(format!("{}/api/visuals/{id}", c.url))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        c.client
            .get(&status_url)
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let comments: Value = c
        .client
        .get(format!("{}/api/feedback", c.url))
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(comments["comments"], json!([]));
    assert_eq!(
        c.client
            .delete(format!("{}/api/workspace", c.url))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        c.client
            .get(format!("{}/api/visuals", c.url))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    c.restart().await;
    assert_eq!(
        c.client
            .get(format!("{}/api/visuals", c.url))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let remaining: Value = c
        .client
        .get(format!("{}/api/workspaces", c.url))
        .bearer_auth(&c.admin)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(remaining, json!([]));
}

#[tokio::test]
async fn the_cli_transfers_local_files_without_exposing_connection_credentials() {
    let c = Companion::start().await;
    let workspace = c.workspace("CLI").await;
    let connection = c.directory.path().join("connection.json");
    std::fs::write(
        &connection,
        serde_json::to_vec(
            &json!({"url":c.url,"workspace":workspace["workspace"],"token":workspace["token"]}),
        )
        .unwrap(),
    )
    .unwrap();
    let svg = c.directory.path().join("drawing.svg");
    std::fs::write(&svg, "<svg xmlns='http://www.w3.org/2000/svg'/>").unwrap();
    let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_atw"))
        .arg("--connection")
        .arg(&connection)
        .args([
            "show",
            "--title",
            "From the CLI",
            "--format",
            "svg",
            "--file",
        ])
        .arg(&svg)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("\"revision\""),
        "show must return its visual identity and revision"
    );
    assert!(!text.contains(workspace["token"].as_str().unwrap()));
    let visual: Value = serde_json::from_str(&text).unwrap();
    let revision: Value = c
        .client
        .get(format!(
            "{}/api/visuals/{}/revisions/1",
            c.url,
            visual["id"].as_str().unwrap()
        ))
        .bearer_auth(workspace["token"].as_str().unwrap())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(revision["title"], "From the CLI");
    assert_eq!(
        revision["content"],
        "<svg xmlns='http://www.w3.org/2000/svg'/>"
    );
}

#[tokio::test]
async fn a_stdio_mcp_client_discovers_tools_and_publishes_in_its_workspace() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let c = Companion::start().await;
    let workspace = c.workspace("MCP").await;
    let path = c.directory.path().join("mcp.json");
    std::fs::write(
        &path,
        serde_json::to_vec(
            &json!({"url":c.url,"workspace":workspace["workspace"],"token":workspace["token"]}),
        )
        .unwrap(),
    )
    .unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_atw"))
        .arg("--connection")
        .arg(path)
        .arg("mcp")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    async fn response(output: &mut BufReader<tokio::process::ChildStdout>) -> Value {
        let mut line = String::new();
        let count = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            output.read_line(&mut line),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            count > 0,
            "MCP server must answer the client's request over stdout"
        );
        serde_json::from_str(&line).unwrap()
    }
    input.write_all(format!("{}\n",json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"atw-integration","version":"1"}}})).as_bytes()).await.unwrap();
    let initialized = response(&mut output).await;
    assert!(initialized["result"]["capabilities"]["tools"].is_object());
    input.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n").await.unwrap();
    let tools = response(&mut output).await;
    assert!(
        tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "show_visual")
    );
    input.write_all(format!("{}\n",json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"show_visual","arguments":{"title":"From MCP","format":"svg","content":"<svg/>"}}})).as_bytes()).await.unwrap();
    let published = response(&mut output).await;
    assert_eq!(published["result"]["isError"], false);
    assert_eq!(published["result"]["structuredContent"]["revision"], 1);
    input.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"list_visuals\",\"arguments\":{}}}\n").await.unwrap();
    let listed = response(&mut output).await;
    assert!(
        listed["result"]["structuredContent"].is_object(),
        "Structured MCP results must be objects, including list results"
    );
}
