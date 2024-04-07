#![deny(unused_extern_crates)]

use std::sync::Arc;

use axum::{Router, routing::get};
use session_manager::SessionManager;
use shuttle_axum::ShuttleAxum;
use tokio::sync::Mutex;
use tower_http::{services::ServeDir, trace::TraceLayer};

mod host;
mod session_manager;
mod user;
mod game;
mod view;
mod packet;

pub type AppState = Arc<Mutex<SessionManager>>;

#[shuttle_runtime::main]
async fn main() -> ShuttleAxum {
    tracing_subscriber::fmt()
        .init();

    let router = Router::new()
        .nest_service("/", ServeDir::new("public"))
        .route("/ws/host/:game_type", get(host::ws_handler))
        .route("/ws/join/:id/:team_id", get(user::ws_handler))
        .route("/sse/view/:id", get(view::sse_handler))
        .with_state(Arc::new(Mutex::new(SessionManager::new())))
        .layer(TraceLayer::new_for_http());

    Ok(router.into())
}

