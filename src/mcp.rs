use crate::{
    client::{self, CompanionClient},
    model::{CommentInput, Format, VisualInput},
};
use reqwest::Method;
use rmcp::{
    ServiceExt,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock},
    schemars, tool, tool_router,
};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

#[derive(Clone)]
pub struct VisualServer {
    client: CompanionClient,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct Show {
    title: String,
    format: Format,
    /// Supply exactly one of content or local_file. Images in content are base64 encoded.
    content: Option<String>,
    /// File on the agent/MCP adapter's machine, transferred to the companion.
    local_file: Option<String>,
    #[serde(default)]
    description: String,
    id: Option<String>,
    expected_revision: Option<i64>,
}
#[derive(Deserialize, schemars::JsonSchema)]
struct Id {
    id: String,
}
#[derive(Deserialize, schemars::JsonSchema)]
struct Revision {
    id: String,
    revision: i64,
}
#[derive(Deserialize, schemars::JsonSchema)]
struct Feedback {
    #[serde(default)]
    after: i64,
    #[serde(default)]
    wait_ms: u64,
}

fn result(value: anyhow::Result<Value>) -> CallToolResult {
    match value {
        Ok(value) => {
            let mut result = CallToolResult::success(vec![ContentBlock::text(value.to_string())]);
            result.structured_content = Some(if value.is_object() {
                value
            } else {
                serde_json::json!({"items":value})
            });
            result
        }
        Err(error) => CallToolResult::error(vec![ContentBlock::text(format!("{error:#}"))]),
    }
}
#[tool_router(server_handler)]
impl VisualServer {
    #[tool(
        description = "Show a diagram, drawing, image, or interactive rendering in this conversation's workspace. Use proactively when a visual explains an idea better than prose. The desktop companion automatically opens its dedicated app when needed and reuses a connected app. Reuse id with expected_revision when refining a visual. Acceptance or a requested app launch does not mean rendered; inspect visual_status."
    )]
    async fn show_visual(&self, Parameters(input): Parameters<Show>) -> CallToolResult {
        result(
            async {
                let content = match (input.content, input.local_file) {
                    (Some(content), None) => content,
                    (None, Some(path)) => {
                        client::content(&input.format, Some(Path::new(&path))).await?
                    }
                    _ => anyhow::bail!("Supply exactly one of content or local_file"),
                };
                let value = VisualInput {
                    title: input.title,
                    format: input.format,
                    content,
                    description: input.description,
                    id: input.id,
                    expected_revision: input.expected_revision,
                };
                self.client
                    .call(Method::POST, "/visuals", Some(serde_json::to_value(value)?))
                    .await
            }
            .await,
        )
    }
    #[tool(
        description = "List visuals in this conversation's workspace, including current revision numbers."
    )]
    async fn list_visuals(&self) -> CallToolResult {
        result(self.client.call(Method::GET, "/visuals", None).await)
    }
    #[tool(description = "List retained revisions of one visual in this conversation.")]
    async fn list_revisions(&self, Parameters(input): Parameters<Id>) -> CallToolResult {
        result(
            self.client
                .call(Method::GET, &format!("/visuals/{}", input.id), None)
                .await,
        )
    }
    #[tool(
        description = "Read a visual's source, description and rendering status at an exact revision."
    )]
    async fn get_visual(&self, Parameters(input): Parameters<Revision>) -> CallToolResult {
        result(
            self.client
                .call(
                    Method::GET,
                    &format!("/visuals/{}/revisions/{}", input.id, input.revision),
                    None,
                )
                .await,
        )
    }
    #[tool(
        description = "Check viewer presence and actual render acknowledgment for an exact revision. Rendered does not establish human attention."
    )]
    async fn visual_status(&self, Parameters(input): Parameters<Revision>) -> CallToolResult {
        result(
            self.client
                .call(
                    Method::GET,
                    &format!("/status/{}/{}", input.id, input.revision),
                    None,
                )
                .await,
        )
    }
    #[tool(
        description = "Read revision-linked comments after a saved cursor, optionally waiting 0–30000 milliseconds. Save the returned cursor. Reads are non-destructive; interrupted calls may replay comments. Does not wake an idle harness."
    )]
    async fn read_feedback(&self, Parameters(input): Parameters<Feedback>) -> CallToolResult {
        result(
            self.client
                .call(
                    Method::GET,
                    &format!("/feedback?after={}&wait_ms={}", input.after, input.wait_ms),
                    None,
                )
                .await,
        )
    }
    #[tool(description = "Attach a comment to an exact visual revision in this conversation.")]
    async fn add_comment(&self, Parameters(input): Parameters<CommentInput>) -> CallToolResult {
        result(
            self.client
                .call(Method::POST, "/comments", serde_json::to_value(input).ok())
                .await,
        )
    }
    #[tool(
        description = "Permanently delete one visual, all its revisions, and its comments from this workspace."
    )]
    async fn delete_visual(&self, Parameters(input): Parameters<Id>) -> CallToolResult {
        result(
            self.client
                .call(Method::DELETE, &format!("/visuals/{}", input.id), None)
                .await,
        )
    }
    #[tool(
        description = "Permanently delete this entire conversation workspace, including visuals, revisions, assets and comments. Its connection will stop working."
    )]
    async fn delete_workspace(&self) -> CallToolResult {
        result(self.client.call(Method::DELETE, "/workspace", None).await)
    }
}

pub async fn serve(client: CompanionClient) -> anyhow::Result<()> {
    let service = VisualServer { client }
        .serve(rmcp::transport::stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}
