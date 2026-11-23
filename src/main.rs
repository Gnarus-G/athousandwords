use anyhow::Context;
use athousandwords::{
    client::{self, CompanionClient, ConnectionSettings, DEFAULT_URL},
    model::{Format, VisualInput},
};
use clap::{Parser, Subcommand};
use reqwest::Method;
use serde_json::json;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "A visual communication companion for agents")]
struct Args {
    #[arg(long, global = true, env = "ATW_DATA_DIR")]
    data_dir: Option<PathBuf>,
    #[arg(long, global = true, env = "ATW_CONNECTION")]
    connection: Option<PathBuf>,
    #[arg(long, global = true, env = "ATW_URL")]
    url: Option<String>,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Run the companion; publishing opens its desktop app when needed.
    Serve {
        #[arg(long, default_value_t = 7331)]
        port: u16,
        /// Store visuals without automatically opening the desktop app (headless use).
        #[arg(long)]
        no_open: bool,
    },
    /// Open the dedicated desktop app, or raise its existing window.
    Open,
    /// Create or list conversation workspaces as the local companion owner.
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    /// Present source from a file, or stdin if --file is omitted.
    Show {
        #[arg(long)]
        title: String,
        #[arg(long, value_enum)]
        format: Format,
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(long, default_value = "")]
        description: String,
        #[arg(long, requires = "expected_revision")]
        id: Option<String>,
        #[arg(long, requires = "id")]
        expected_revision: Option<i64>,
    },
    List,
    History {
        id: String,
    },
    Get {
        id: String,
        revision: i64,
    },
    Status {
        id: String,
        revision: i64,
    },
    Feedback {
        #[arg(long, default_value_t = 0)]
        after: i64,
        #[arg(long, default_value_t = 0)]
        wait_ms: u64,
    },
    Comment {
        id: String,
        revision: i64,
        text: String,
    },
    Delete {
        id: String,
    },
    /// Delete this workspace and all its visuals, history, assets, and comments.
    DeleteWorkspace,
    /// Serve workspace-scoped MCP over stdin/stdout.
    Mcp,
}
#[derive(Subcommand)]
enum WorkspaceCommand {
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        out: PathBuf,
    },
    List,
}

fn data_dir(value: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(path) = value {
        return Ok(path);
    }
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path).join("athousandwords"));
    }
    Ok(PathBuf::from(
        std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("Set --data-dir or HOME"))?,
    )
    .join(".local/share/athousandwords"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let directory = data_dir(args.data_dir)?;
    match args.command {
        Command::Serve { port, no_open } => {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
            let url = format!("http://{}", listener.local_addr()?);
            let desktop_directory = directory.clone();
            let launcher = (!no_open).then(|| {
                athousandwords::viewer::ViewerLauncher::new(url, move |url, workspace| {
                    athousandwords::viewer::launch_desktop(
                        &desktop_directory,
                        url,
                        Some(workspace),
                        true,
                    )
                })
            });
            let app = athousandwords::server::app_with_viewer_launcher(&directory, launcher)?;
            eprintln!(
                "A Thousand Words listening on http://{}; desktop auto-opening is {}.",
                listener.local_addr()?,
                if no_open { "disabled" } else { "enabled" }
            );
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await?;
            return Ok(());
        }
        Command::Open => {
            let url = args.url.unwrap_or_else(|| DEFAULT_URL.into());
            CompanionClient::new(ConnectionSettings {
                url: url.clone(),
                workspace: String::new(),
                token: String::new(),
            })?;
            athousandwords::viewer::launch_desktop(&directory,&url,None,false)
                .context("Could not start the desktop app; install its runtime with npm ci and node node_modules/electron/install.js")?;
            return Ok(());
        }
        Command::Workspace { command } => {
            let token = std::fs::read_to_string(directory.join("admin.token")).map_err(|_| {
                anyhow::anyhow!(
                    "Start the companion first; its owner connection is not initialized"
                )
            })?;
            let url = args.url.unwrap_or_else(|| DEFAULT_URL.into());
            let owner = CompanionClient::new(ConnectionSettings {
                url: url.clone(),
                workspace: String::new(),
                token,
            })?;
            let result = match command {
                WorkspaceCommand::List => owner.call(Method::GET, "/workspaces", None).await?,
                WorkspaceCommand::Create { title, out } => {
                    let mut file = client::reserve_connection(&out)?;
                    let created = match owner
                        .call(Method::POST, "/workspaces", Some(json!({"title":title})))
                        .await
                    {
                        Ok(created) => created,
                        Err(error) => {
                            let _ = std::fs::remove_file(&out);
                            return Err(error);
                        }
                    };
                    if let Err(error) = client::write_connection(
                        &mut file,
                        &ConnectionSettings {
                            url,
                            workspace: created["workspace"].as_str().unwrap().into(),
                            token: created["token"].as_str().unwrap().into(),
                        },
                    ) {
                        let _ = std::fs::remove_file(&out);
                        anyhow::bail!(
                            "Workspace {} was created but its connection file could not be written; delete that workspace in the viewer before retrying: {error}",
                            created["workspace"].as_str().unwrap()
                        );
                    }
                    json!({"workspace":created["workspace"],"title":created["title"],"connection_file":out})
                }
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(());
        }
        command => {
            let connection = args.connection.ok_or_else(|| {
                anyhow::anyhow!(
                    "Supply --connection with this conversation's private connection file"
                )
            })?;
            let client = CompanionClient::from_file(&connection, args.url)?;
            let result = match command {
                Command::Show {
                    title,
                    format,
                    file,
                    description,
                    id,
                    expected_revision,
                } => {
                    let content = client::content(&format, file.as_deref()).await?;
                    client
                        .call(
                            Method::POST,
                            "/visuals",
                            Some(serde_json::to_value(VisualInput {
                                title,
                                format,
                                content,
                                description,
                                id,
                                expected_revision,
                            })?),
                        )
                        .await?
                }
                Command::List => client.call(Method::GET, "/visuals", None).await?,
                Command::History { id } => {
                    client
                        .call(Method::GET, &format!("/visuals/{id}"), None)
                        .await?
                }
                Command::Get { id, revision } => {
                    client
                        .call(
                            Method::GET,
                            &format!("/visuals/{id}/revisions/{revision}"),
                            None,
                        )
                        .await?
                }
                Command::Status { id, revision } => {
                    client
                        .call(Method::GET, &format!("/status/{id}/{revision}"), None)
                        .await?
                }
                Command::Feedback { after, wait_ms } => {
                    client
                        .call(
                            Method::GET,
                            &format!("/feedback?after={after}&wait_ms={wait_ms}"),
                            None,
                        )
                        .await?
                }
                Command::Comment { id, revision, text } => {
                    client
                        .call(
                            Method::POST,
                            "/comments",
                            Some(json!({"visual":id,"revision":revision,"text":text})),
                        )
                        .await?
                }
                Command::Delete { id } => {
                    client
                        .call(Method::DELETE, &format!("/visuals/{id}"), None)
                        .await?
                }
                Command::DeleteWorkspace => client.call(Method::DELETE, "/workspace", None).await?,
                Command::Mcp => {
                    athousandwords::mcp::serve(client).await?;
                    return Ok(());
                }
                _ => unreachable!(),
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }
    Ok(())
}
