use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    routing::post,
};
use ledger_store::LedgerService;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
use tower_http::services::ServeDir;

struct AppState {
    service: LedgerService,
    port: u16,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |key: &str| {
        args.windows(2)
            .find(|pair| pair[0] == key)
            .map(|pair| pair[1].clone())
    };
    let data = PathBuf::from(arg("--data").unwrap_or_else(|| ".data/open-ledger.sqlite".into()));
    let frontend = PathBuf::from(arg("--frontend").unwrap_or_else(|| "dist".into()));
    let port: u16 = arg("--port").unwrap_or_else(|| "4173".into()).parse()?;
    let state = Arc::new(AppState {
        service: LedgerService::open(&data)?,
        port,
    });
    let app = Router::new()
        .route("/api/{command}", post(request))
        .layer(DefaultBodyLimit::max(21 * 1024 * 1024))
        .fallback_service(ServeDir::new(frontend))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    println!("Open Ledger is available at http://127.0.0.1:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
async fn request(
    State(state): State<Arc<AppState>>,
    Path(command): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let allowed_hosts = [
        format!("127.0.0.1:{}", state.port),
        format!("localhost:{}", state.port),
        "127.0.0.1:1420".into(),
        "localhost:1420".into(),
    ];
    if !headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| allowed_hosts.iter().any(|h| h == s))
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error":"This host is not allowed."})),
        ));
    }
    if let Some(origin) = headers.get("origin") {
        let origin = origin.to_str().unwrap_or("");
        if ![
            format!("http://127.0.0.1:{}", state.port),
            format!("http://localhost:{}", state.port),
            "http://127.0.0.1:1420".into(),
            "http://localhost:1420".into(),
        ]
        .iter()
        .any(|allowed| allowed == origin)
        {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({"error":"This origin is not allowed."})),
            ));
        }
    }
    let result = tokio::task::spawn_blocking(move || state.service.request(&command, payload))
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"The request could not be completed."})),
            )
        })?;
    result
        .map(Json)
        .map_err(|error| (StatusCode::BAD_REQUEST, Json(json!({"error":error}))))
}
