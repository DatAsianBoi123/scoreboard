#![deny(unused_extern_crates)]

use std::sync::Arc;

use axum::{Router, routing::get};
use session_manager::SessionManager;
use tokio::sync::Mutex;
use tower_http::{services::ServeDir, trace::TraceLayer};

mod host;
mod session_manager;
mod user;
mod game;
mod view;
mod packet;

pub type AppState = Arc<Mutex<SessionManager>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let router = Router::new()
        .fallback_service(ServeDir::new("public"))
        .route("/api/builtin-games", get(game::get_all_builtin))
        .route("/ws/host", get(host::ws_handler))
        .route("/ws/join/{id}/{team_id}", get(user::ws_handler))
        .route("/sse/view/{id}", get(view::sse_handler))
        .with_state(Arc::new(Mutex::new(SessionManager::new())))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

