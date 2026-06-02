use std::time::Duration;

use axum::{extract::{Path, State, WebSocketUpgrade, ws::{Message, WebSocket}}, http::StatusCode, response::{IntoResponse, Response}};
use tokio::sync::{broadcast::{Receiver, Sender}, mpsc};
use futures::{SinkExt, StreamExt};
use tracing::{error, info};

use crate::{session_manager::{Team, HostMessage, UserMessage}, AppState, packet::{ServerboundUserPacket, ClientboundUserPacket, IntoBytes, FromBytes}, game::GameData};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path((id, team)): Path<(u32, Team)>,
    State(state): State<AppState>,
) -> Response {
    let res = {
        let lock = state.lock().await;
        let session = lock.get_session(id);
        session.map(|session| {
            (session.game_state.time_started.is_some(), session.game_data.clone(), session.host.sender.clone(), session.user.sender.subscribe())
        })
    };
    let (started, game_data, host_sender, user_recv) = if let Some((started, game_data, host_sender, user_recv)) = res {
        (started, game_data, host_sender, user_recv)
    } else {
        return (StatusCode::BAD_REQUEST, "invalid session id").into_response();
    };

    info!("[{id}] user connected");

    ws.on_upgrade(move |ws| handle_upgrade(ws, UserInfo { session_id: id, team }, started, game_data, host_sender, user_recv, state)).into_response()
}

async fn handle_upgrade(
    mut ws: WebSocket,
    UserInfo { session_id: id, team }: UserInfo,
    started: bool,
    game_data: GameData,
    host_sender: Sender<HostMessage>,
    mut user_recv: Receiver<UserMessage>,
    state: AppState,
) {
    if let Err(err) = ws.send(Message::Binary(ClientboundUserPacket::SessionInfo(started, game_data).into_bytes())).await {
        info!("[{id}] could not send user score info. {err:?}");
        ws.close().await.expect("can close ws");
        return;
    }

    let (mut sender, mut recv) = ws.split();
    let (ws_sender, mut ws_recv) = mpsc::unbounded_channel();

    let ws_send_task = async {
        while let Some(message) = ws_recv.recv().await {
            if sender.send(message).await.is_err() { break; }
        }
    };

    let ping_task = async {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.tick().await;

        loop {
            interval.tick().await;
            if ws_sender.send(Message::Ping(Default::default())).is_err() { break; }
        }
    };

    let close_task = async {
        while let Ok(message) = user_recv.recv().await {
            let bytes = match message {
                UserMessage::GameStart => Some(ClientboundUserPacket::StartGame().into_bytes()),
                UserMessage::GameEnd => Some(ClientboundUserPacket::EndGame().into_bytes()),
                UserMessage::Close => None,
            };

            if let Some(bytes) = bytes {
                if ws_sender.send(Message::Binary(bytes)).is_err() { break; }
            } else {
                break;
            }
        }
    };

    let recv_task = async {
        while let Some(message) = recv.next().await {
            match message {
                Ok(Message::Binary(bytes)) => {
                    let message = {
                        match ServerboundUserPacket::from_bytes(bytes) {
                            Some(ServerboundUserPacket::Score { score_type, undo }) => {
                                let can_score = {
                                    let lock = state.lock().await;
                                    let session = lock.get_session(id).expect("session exists");
                                    let started = session.game_state.time_started.is_some();

                                    started && (score_type as usize) < session.game_data.score_points.len()
                                };

                                if can_score { HostMessage::Score(team, score_type, undo) }
                                else { break; }
                            },
                            None => break,
                        }
                    };

                    if host_sender.send(message).is_err() { break; }
                },
                Ok(_) => {},
                Err(err) => error!("[user {id}] {err}"),
            }
        }
    };

    tokio::select! {
        _ = ws_send_task => {},
        _ = ping_task => {},
        _ = close_task => {},
        _ = recv_task => {},
    };

    info!("[{id}] user disconnected");
}

struct UserInfo {
    session_id: u32,
    team: Team,
}

