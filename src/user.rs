use axum::{extract::{WebSocketUpgrade, Path, State, ws::WebSocket}, response::{IntoResponse, Response}, http::StatusCode};
use tokio::sync::broadcast::{Sender, Receiver};
use futures::{StreamExt, SinkExt};
use tracing::info;

use crate::{session_manager::{Team, HostMessage, UserMessage}, AppState, packet::{ServerboundUserPacket, ClientboundUserPacket, IntoMessage, FromMessage}, game::GameData};

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
    if let Err(err) = ws.send(ClientboundUserPacket::SessionInfo(started, game_data).into_message()).await {
        info!("[{id}] could not send user score info. {err:?}");
        ws.close().await.expect("can close ws");
        return;
    }

    let (mut sender, mut recv) = ws.split();

    let mut close_task = tokio::spawn(async move {
        while let Ok(message) = user_recv.recv().await {
            let message = match message {
                UserMessage::GameStart => Some(ClientboundUserPacket::StartGame().into_message()),
                UserMessage::GameEnd => Some(ClientboundUserPacket::EndGame().into_message()),
                UserMessage::Close => None,
            };

            if let Some(message) = message {
                if sender.send(message).await.is_err() { break; }
            } else {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(message)) = recv.next().await {
            let message = {
                match ServerboundUserPacket::from_message(message) {
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
        }
    });

    tokio::select! {
        _ = &mut close_task => {},
        _ = &mut recv_task => {},
    };

    info!("[{id}] user disconnected");
}

struct UserInfo {
    session_id: u32,
    team: Team,
}

