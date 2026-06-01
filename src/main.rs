#![deny(unused_extern_crates)]

use std::{error::Error, net::{IpAddr, Ipv4Addr, SocketAddr}, sync::Arc};

use axum::{Router, routing::get};
use session_manager::SessionManager;
use tokio::sync::Mutex;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::info;

mod host;
mod session_manager;
mod user;
mod game;
mod view;
mod packet;

pub type AppState = Arc<Mutex<SessionManager>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let port = std::env::var("PORT")
        .map(|port| port.parse())
        .unwrap_or(Ok(3000u16))?;

    let router = Router::new()
        .fallback_service(ServeDir::new("public"))
        .route("/api/builtin-games", get(game::get_all_builtin))
        .route("/ws/host", get(host::ws_handler))
        .route("/ws/join/{id}/{team_id}", get(user::ws_handler))
        .route("/sse/view/{id}", get(view::sse_handler))
        .with_state(Arc::new(Mutex::new(SessionManager::new())))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("listening on {addr}...");
    Ok(axum::serve(listener, router).await?)
}

