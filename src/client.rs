use crate::model::Format;
use anyhow::{Context, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{io::Write, os::unix::fs::OpenOptionsExt, path::Path};
use tokio::io::AsyncReadExt;

pub const DEFAULT_URL: &str = "http://127.0.0.1:7331";
pub const MAX_CONTENT: u64 = 8 * 1024 * 1024;

#[derive(Clone, Deserialize, Serialize)]
pub struct ConnectionSettings {
    pub url: String,
    pub workspace: String,
    pub token: String,
}

#[derive(Clone)]
pub struct CompanionClient {
    settings: ConnectionSettings,
    http: Client,
}

impl CompanionClient {
    pub fn new(settings: ConnectionSettings) -> anyhow::Result<Self> {
        let url = reqwest::Url::parse(&settings.url)?;
        if url.scheme() != "http"
            || !matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            bail!("Use an HTTP loopback URL, directly or through SSH forwarding");
        }
        let http = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(35))
            .build()?;
        Ok(Self { settings, http })
    }
    pub fn from_file(path: &Path, url: Option<String>) -> anyhow::Result<Self> {
        let mut settings: ConnectionSettings = serde_json::from_slice(
            &std::fs::read(path).context("Read the workspace connection file")?,
        )
        .context("Invalid workspace connection file")?;
        if let Some(url) = url {
            settings.url = url;
        }
        Self::new(settings)
    }
    pub async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> anyhow::Result<Value> {
        if path.contains("..") || path.contains(['%', '#']) {
            bail!("Invalid identifier in companion request");
        }
        let mut request = self
            .http
            .request(
                method,
                format!("{}/api{path}", self.settings.url.trim_end_matches('/')),
            )
            .bearer_auth(&self.settings.token)
            .header("x-atw-workspace", &self.settings.workspace);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.context(
            "Contact the companion; check that it is running and any SSH forwarding is connected",
        )?;
        let status = response.status();
        if !status.is_success() {
            bail!("Companion returned {status}: {}", response.text().await?);
        }
        let bytes = response.bytes().await?;
        if bytes.is_empty() {
            return Ok(serde_json::json!({"ok":true}));
        }
        Ok(serde_json::from_slice(&bytes)?)
    }
}

pub async fn content(format: &Format, path: Option<&Path>) -> anyhow::Result<String> {
    let mut bytes = Vec::new();
    if let Some(path) = path {
        if tokio::fs::metadata(path).await?.len() > MAX_CONTENT {
            bail!("Content exceeds 8 MiB");
        }
        tokio::fs::File::open(path)
            .await?
            .take(MAX_CONTENT + 1)
            .read_to_end(&mut bytes)
            .await?;
    } else {
        tokio::io::stdin()
            .take(MAX_CONTENT + 1)
            .read_to_end(&mut bytes)
            .await?;
    }
    if bytes.len() as u64 > MAX_CONTENT {
        bail!("Content exceeds 8 MiB");
    }
    match format {
        Format::Png | Format::Jpeg | Format::Webp => Ok(STANDARD.encode(bytes)),
        _ => Ok(String::from_utf8(bytes).context("Text content must be UTF-8")?),
    }
}

pub fn reserve_connection(path: &Path) -> anyhow::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .context("Create a new private connection file (its parent must exist and the output must not already exist)")
}

pub fn write_connection(
    file: &mut std::fs::File,
    settings: &ConnectionSettings,
) -> anyhow::Result<()> {
    file.write_all(&serde_json::to_vec_pretty(settings)?)?;
    file.sync_all()?;
    Ok(())
}
