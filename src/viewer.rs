use serde_json::{Value, json};
use std::{
    io,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

type OpenApp = dyn Fn(&str, &str) -> io::Result<()> + Send + Sync;
struct Attempt {
    at: Instant,
    failed: bool,
}

#[derive(Clone)]
pub struct ViewerLauncher {
    base_url: String,
    open: Arc<OpenApp>,
    last_attempt: Arc<Mutex<Option<Attempt>>>,
}

impl ViewerLauncher {
    pub fn new(
        base_url: String,
        open: impl Fn(&str, &str) -> io::Result<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            base_url,
            open: Arc::new(open),
            last_attempt: Arc::new(Mutex::new(None)),
        }
    }
    pub fn connected(&self) {
        *self.last_attempt.lock().unwrap() = None;
    }

    pub async fn open_if_needed(&self, workspace: &str) -> Value {
        {
            let mut last = self.last_attempt.lock().unwrap();
            if let Some(attempt) = last
                .as_ref()
                .filter(|a| a.at.elapsed() < Duration::from_secs(10))
            {
                return opening(if attempt.failed { "failed" } else { "pending" });
            }
            *last = Some(Attempt {
                at: Instant::now(),
                failed: false,
            });
        }
        let open = self.open.clone();
        let url = self.base_url.clone();
        let workspace = workspace.to_string();
        let succeeded = tokio::task::spawn_blocking(move || open(&url, &workspace))
            .await
            .is_ok_and(|result| result.is_ok());
        if !succeeded && let Some(attempt) = self.last_attempt.lock().unwrap().as_mut() {
            attempt.failed = true;
        }
        opening(if succeeded { "requested" } else { "failed" })
    }
}

fn opening(status: &str) -> Value {
    json!({"status":status,"error":if status=="failed" {Some("Could not launch the desktop app. Check the companion's desktop session and Electron installation, or run atw open.")}else{None}})
}

pub fn launch_desktop(
    directory: &std::path::Path,
    url: &str,
    workspace: Option<&str>,
    background: bool,
) -> io::Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut command = std::process::Command::new(root.join("node_modules/electron/dist/electron"));
    command
        .arg(root.join("desktop/main.cjs"))
        .arg("--data-dir")
        .arg(directory)
        .arg("--url")
        .arg(url)
        .env_remove("ELECTRON_RUN_AS_NODE")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if let Some(workspace) = workspace {
        command.arg("--workspace").arg(workspace);
    }
    if background {
        command.arg("--background");
    }
    let mut child = command.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
