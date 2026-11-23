use crate::{
    model::{CommentInput, Error, Result, VisualInput},
    store::Store,
    viewer::ViewerLauncher,
};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
};
use serde::Deserialize;
use serde_json::Value;
use std::{path::Path as FilePath, sync::Arc};

#[derive(Clone)]
struct Scope(Option<String>);
type Shared = Arc<Store>;

pub fn app(directory: &FilePath) -> anyhow::Result<Router> {
    app_with_viewer_launcher(directory, None)
}

pub fn app_with_viewer_launcher(
    directory: &FilePath,
    launcher: Option<ViewerLauncher>,
) -> anyhow::Result<Router> {
    let store = Arc::new(Store::open(directory)?);
    let api = Router::new()
        .route("/workspaces", post(create_workspace).get(workspaces))
        .route("/workspace", delete(delete_workspace))
        .route("/visuals", post(publish).get(visuals))
        .route("/visuals/{id}", get(history).delete(delete_visual))
        .route("/visuals/{id}/revisions/{revision}", get(revision))
        .route("/status/{id}/{revision}", get(presentation))
        .route("/viewer", post(viewer_connect).delete(viewer_disconnect))
        .route("/render", post(render))
        .route("/comments", post(comment))
        .route("/feedback", get(feedback))
        .layer(Extension(launcher))
        .layer(middleware::from_fn_with_state(store.clone(), authorize));
    Ok(Router::new()
        .nest("/api", api)
        .route("/health", get(|| async { "ok" }))
        .route(
            "/",
            get(|| async { Html(include_str!("../web/index.html")) }),
        )
        .route("/static/{file}", get(asset))
        .route("/frame", get(frame))
        .layer(middleware::from_fn(security_headers))
        .layer(DefaultBodyLimit::max(13 * 1024 * 1024))
        .with_state(store))
}

fn host(headers: &HeaderMap) -> Result<&str> {
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let authority: axum::http::uri::Authority =
        host.parse().map_err(|_| Error::bad("Invalid Host"))?;
    if !matches!(authority.host(), "127.0.0.1" | "localhost" | "[::1]") {
        return Err(Error::bad("Only loopback hosts are accepted"));
    }
    Ok(host)
}
async fn security_headers(request: Request, next: Next) -> Result<Response> {
    let host = host(request.headers())?.to_string();
    let frame = request.uri().path() == "/frame";
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("cache-control", HeaderValue::from_static("no-store"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    if !frame {
        headers.insert("content-security-policy",HeaderValue::from_str(&format!("default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-src http://{host}/frame; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'" )).unwrap());
    }
    Ok(response)
}
async fn asset(Path(file): Path<String>) -> Result<Response> {
    let (mime, content) = match file.as_str() {
        "app.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../web/app.js"),
        ),
        "keyboard.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../web/keyboard.js"),
        ),
        "style.css" => ("text/css; charset=utf-8", include_str!("../web/style.css")),
        "renderer.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../web/dist/renderer.js"),
        ),
        _ => return Err(Error::missing()),
    };
    Ok(([("content-type", mime)], content).into_response())
}
async fn frame(headers: HeaderMap) -> Result<Response> {
    let host = host(&headers)?;
    let mut response=Html("<!doctype html><html><head><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'><style>html{color:#25364a;background:white;font:16px/1.5 'Noto Sans',sans-serif}body{margin:24px}img,svg{max-width:100%;height:auto}#render-root{min-height:100px}button,input,select{font:inherit}</style><script src='/static/renderer.js' defer></script></head><body><div id='render-root'></div></body></html>").into_response();
    response.headers_mut().insert("content-security-policy",HeaderValue::from_str(&format!("default-src 'none'; script-src 'unsafe-inline' http://{host}/static/renderer.js; style-src 'unsafe-inline'; img-src data: blob:; font-src data:; connect-src 'none'; media-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'; frame-ancestors 'self'" )).unwrap());
    response.headers_mut().insert(
        "connection-allowlist",
        HeaderValue::from_str(&format!("(\"http://{host}/static/renderer.js\")")).unwrap(),
    );
    Ok(response)
}

async fn authorize(
    State(store): State<Shared>,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    let host = host(request.headers())?;
    if let Some(origin) = request.headers().get("origin")
        && origin.to_str().ok() != Some(format!("http://{host}").as_str())
    {
        return Err(Error(
            StatusCode::FORBIDDEN,
            "Cross-origin control requests are refused".into(),
        ));
    }
    let token = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    let scope = Scope(store.scope(token)?);
    request.extensions_mut().insert(scope);
    Ok(next.run(request).await)
}

fn admin(scope: &Scope) -> Result<()> {
    if scope.0.is_some() {
        return Err(Error(
            StatusCode::FORBIDDEN,
            "This operation is for the companion owner".into(),
        ));
    }
    Ok(())
}

fn workspace(scope: &Scope, headers: &HeaderMap) -> Result<String> {
    if let Some(id) = &scope.0 {
        return Ok(id.clone());
    }
    headers
        .get("x-atw-workspace")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| Error::bad("Select a workspace"))
}

#[derive(Deserialize)]
struct WorkspaceInput {
    title: String,
}
async fn create_workspace(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
    Json(input): Json<WorkspaceInput>,
) -> Result<(StatusCode, Json<Value>)> {
    admin(&scope)?;
    Ok((
        StatusCode::CREATED,
        Json(store.create_workspace(&input.title)?),
    ))
}
async fn publish(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
    Extension(launcher): Extension<Option<ViewerLauncher>>,
    headers: HeaderMap,
    Json(input): Json<VisualInput>,
) -> Result<(StatusCode, Json<Value>)> {
    let workspace = workspace(&scope, &headers)?;
    let mut result = store.publish(&workspace, input)?;
    result["presentation"]["viewer_connected"] = serde_json::json!(if launcher.is_some() {
        store.desktop_connected()
    } else {
        store.viewer_connected()
    });
    result["presentation"]["opening"] = match launcher {
        Some(launcher) if !store.desktop_connected() => launcher.open_if_needed(&workspace).await,
        Some(_) => serde_json::json!({"status":"already_connected","error":null}),
        None => serde_json::json!({"status":"disabled","error":null}),
    };
    Ok((StatusCode::CREATED, Json(result)))
}

async fn workspaces(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
) -> Result<Json<Value>> {
    Ok(Json(store.workspaces(scope.0.as_deref())?))
}
async fn visuals(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    Ok(Json(store.visuals(&workspace(&scope, &headers)?)?))
}
async fn history(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    Ok(Json(store.history(&workspace(&scope, &headers)?, &id)?))
}
async fn delete_visual(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    store.delete_visual(&workspace(&scope, &headers)?, &id)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn delete_workspace(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
    headers: HeaderMap,
) -> Result<StatusCode> {
    store.delete_workspace(&workspace(&scope, &headers)?)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn presentation(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
    Extension(launcher): Extension<Option<ViewerLauncher>>,
    headers: HeaderMap,
    Path((id, revision)): Path<(String, i64)>,
) -> Result<Json<Value>> {
    let mut result = store.presentation(&workspace(&scope, &headers)?, &id, revision)?;
    if launcher.is_some() {
        result["viewer_connected"] = serde_json::json!(store.desktop_connected());
    }
    Ok(Json(result))
}
#[derive(Deserialize)]
struct ViewerInput {
    viewer: String,
    #[serde(default)]
    desktop: bool,
}
async fn viewer_connect(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
    Extension(launcher): Extension<Option<ViewerLauncher>>,
    Json(input): Json<ViewerInput>,
) -> Result<Json<Value>> {
    admin(&scope)?;
    store.viewer(&input.viewer, true, input.desktop)?;
    if input.desktop
        && let Some(launcher) = launcher
    {
        launcher.connected();
    }
    Ok(Json(serde_json::json!({"connected":true})))
}
async fn viewer_disconnect(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
    Json(input): Json<ViewerInput>,
) -> Result<StatusCode> {
    admin(&scope)?;
    store.viewer(&input.viewer, false, input.desktop)?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
struct RenderInput {
    visual: String,
    revision: i64,
    status: String,
    error: Option<String>,
    #[serde(default)]
    desktop: bool,
}
async fn render(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
    Extension(launcher): Extension<Option<ViewerLauncher>>,
    Json(input): Json<RenderInput>,
) -> Result<StatusCode> {
    admin(&scope)?;
    // During migration a stale default-browser tab may still send failures.
    // In desktop mode only the dedicated app establishes rendering evidence.
    if launcher.is_some() && !input.desktop {
        return Ok(StatusCode::OK);
    }
    store.render(
        &input.visual,
        input.revision,
        &input.status,
        input.error.as_deref(),
    )?;
    Ok(StatusCode::OK)
}
async fn revision(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
    headers: HeaderMap,
    Path((id, revision)): Path<(String, i64)>,
) -> Result<Json<Value>> {
    Ok(Json(store.revision(
        &workspace(&scope, &headers)?,
        &id,
        revision,
    )?))
}

async fn comment(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
    Json(input): Json<CommentInput>,
) -> Result<(StatusCode, Json<Value>)> {
    Ok((
        StatusCode::CREATED,
        Json(store.comment(scope.0.as_deref(), input)?),
    ))
}
#[derive(Deserialize)]
struct FeedbackQuery {
    #[serde(default)]
    after: i64,
    #[serde(default)]
    wait_ms: u64,
}
async fn feedback(
    State(store): State<Shared>,
    Extension(scope): Extension<Scope>,
    headers: HeaderMap,
    Query(query): Query<FeedbackQuery>,
) -> Result<Json<Value>> {
    if query.wait_ms > 30_000 {
        return Err(Error::bad("Wait must be 0–30000 milliseconds"));
    }
    let workspace = workspace(&scope, &headers)?;
    let mut changed = store.changed.subscribe();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(query.wait_ms);
    loop {
        let feedback = store.feedback(&workspace, query.after)?;
        if !feedback["comments"].as_array().unwrap().is_empty() || query.wait_ms == 0 {
            return Ok(Json(feedback));
        }
        if tokio::time::timeout_at(deadline, changed.changed())
            .await
            .is_err()
        {
            return Ok(Json(feedback));
        }
    }
}
