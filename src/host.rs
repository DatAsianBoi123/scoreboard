use axum::{extract::{WebSocketUpgrade, ws::WebSocket, State, Path}, response::{IntoResponse, Response}, http::StatusCode};
use futures::{StreamExt, SinkExt};
use rand::{thread_rng, Rng};
use tracing::info;

use crate::{AppState, session_manager::{HostMessageType, UserMessage, ViewerMessage, Team}, game::{self, GameData}, packet::{ClientboundHostPacket, IntoMessage, ServerboundHostPacket, FromMessage}};

pub async fn ws_handler(ws: WebSocketUpgrade, Path(game_type): Path<u8>, State(state): State<AppState>) -> Response {
    let session_id = thread_rng().gen();

    if let Some(score_points) = game::from_id(game_type) {
        ws.on_upgrade(move |ws| handle_socket(ws, session_id, score_points, state)).into_response()
    } else {
        (StatusCode::BAD_REQUEST, format!("game type with id {game_type} not found")).into_response()
    }
}

async fn handle_socket(mut ws: WebSocket, session_id: u32, game_data: GameData, state: AppState) {
    let (mut host_recv, user_sender, viewer_sender) = {
        let mut lock = state.lock().await;
        let session = lock.new_session(session_id, game_data.clone());
        (session.host.sender.subscribe(), session.user.sender.clone(), session.viewer.sender.clone())
    };

    if let Err(err) = ws.send(ClientboundHostPacket::SessionInfo(session_id, game_data.clone()).into_message()).await {
        info!("[{session_id}] could not send info message! {err:?}");
        ws.close().await.expect("can close websocket");
        return;
    }

    info!("[{session_id}] created");

    let (mut sender, mut receiver) = ws.split();

    let send_viewer_sender = viewer_sender.clone();
    let send_state = state.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(message) = host_recv.recv().await {
            let message = match message.message {
                HostMessageType::Score(team, score_id) => {
                    {
                        let mut lock = send_state.lock().await;
                        let session = lock.get_session_mut(session_id).expect("session exists");
                        let game_state = &mut session.game_state;
                        let map = match team {
                            Team::Blue => &mut game_state.blue_scored,
                            Team::Red => &mut game_state.red_scored,
                        };
                        map.entry(score_id).and_modify(|scored| *scored += 1).or_insert(1);
                    }
                    send_viewer_sender.send(ViewerMessage::Score(team, score_id)).ok()
                        .map(|_| ClientboundHostPacket::Score(team, score_id).into_message())
                }
            };

            if let Some(message) = message {
                if sender.send(message).await.is_err() { break; }
            } else {
                break;
            }
        };
    });

    let recv_state = state.clone();
    let mut receive_task = tokio::spawn(async move {
        while let Some(Ok(message)) = receiver.next().await {
            if let Some(packet) = ServerboundHostPacket::from_message(message) {
                info!("[{session_id}] received message: {packet:?}");

                let message = match packet {
                    ServerboundHostPacket::StartGame { time_started } => {
                        {
                            let mut lock = recv_state.lock().await;
                            let game_state = &mut lock.get_session_mut(session_id).expect("session exists").game_state;
                            if game_state.time_started.is_some() { break; }
                            game_state.time_started = Some(time_started);
                        };
                        info!("[{session_id}] started game");
                        viewer_sender.send(ViewerMessage::GameStart(time_started)).expect("receivers exist for viewer");
                        Some(UserMessage::GameStart)
                    },
                    ServerboundHostPacket::EndGame => {
                        {
                            let mut lock = recv_state.lock().await;
                            let game_state = &mut lock.get_session_mut(session_id).expect("session exists").game_state;
                            if game_state.time_started.is_none() { break; }
                            game_state.time_started = None;
                            game_state.ended = true;
                        };
                        info!("[{session_id}] ended game");
                        viewer_sender.send(ViewerMessage::GameEnd).expect("receivers exist for viewer");
                        Some(UserMessage::GameEnd)
                    },
                    ServerboundHostPacket::PauseGame => {
                        {
                            let mut lock = recv_state.lock().await;
                            let game_state = &mut lock.get_session_mut(session_id).expect("session exists").game_state;
                            if game_state.paused { break; }
                            game_state.paused = true;
                        };
                        info!("[{session_id}] paused game");
                        viewer_sender.send(ViewerMessage::GamePause).expect("receivers exist for viewer");
                        None
                    },
                    ServerboundHostPacket::UnpauseGame { time_paused } => {
                        {
                            let mut lock = recv_state.lock().await;
                            let game_state = &mut lock.get_session_mut(session_id).expect("session exists").game_state;
                            if !game_state.paused { break; }
                            game_state.time_paused += time_paused;
                            game_state.paused = false;
                        };
                        info!("[{session_id}] unpaused game (paused for {}s)", time_paused / 1000);
                        viewer_sender.send(ViewerMessage::GameUnpause(time_paused)).expect("receivers exist for viewer");
                        None
                    },
                };

                if let Some(message) = message {
                    if user_sender.send(message).is_err() { break; }
                };
            } else {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => receive_task.abort(),
        _ = (&mut receive_task) => send_task.abort(),
    };

    {
        let mut lock = state.lock().await;
        lock.close_session(session_id);
    }

    info!("[{session_id}] disconnected");
}

