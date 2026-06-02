use std::time::Duration;

use axum::{extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}}, response::{IntoResponse, Response}};
use futures::{StreamExt, SinkExt};
use rand::{thread_rng, Rng};
use tokio::{sync::{broadcast::error::RecvError, mpsc}, time::timeout};
use tracing::{error, info};

use crate::{AppState, game::GameData, packet::{ClientboundHostPacket, Either, FromBytes, IntoBytes, ServerboundHostPacket}, session_manager::{HostMessage, Session, Team, UserMessage, ViewerMessage}};

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    let session_id = thread_rng().gen();

    ws.on_upgrade(move |ws| handle_socket(ws, session_id, state)).into_response()
}

async fn handle_socket(mut ws: WebSocket, session_id: u32, state: AppState) {
    if let Ok(Some(Ok(Message::Binary(bytes)))) = timeout(Duration::from_secs(3), async { ws.recv().await }).await {
        if let Some(ServerboundHostPacket::GameData { match_number, blue_teams, red_teams, game_type }) = ServerboundHostPacket::from_bytes(bytes) {
            let game_data = match game_type {
                Either::Left(builtin) => builtin.data.clone(),
                Either::Right(custom) => custom,
            };
            session_start(ws, session_id, blue_teams, red_teams, match_number, game_data, state).await;
        }
    } else { ws.close().await.expect("can close ws"); };
}

async fn session_start(mut ws: WebSocket, session_id: u32, blue_teams: Vec<String>, red_teams: Vec<String>, match_number: u16, game_data: GameData, state: AppState) {
    let (mut host_recv, user_sender, viewer_sender) = {
        let mut lock = state.lock().await;
        let session = lock.new_session(session_id, blue_teams, red_teams, match_number, game_data.clone());
        (session.host.sender.subscribe(), session.user.sender.clone(), session.viewer.sender.clone())
    };

    if let Err(err) = ws.send(Message::Binary(ClientboundHostPacket::SessionInfo(session_id, game_data.clone()).into_bytes())).await {
        info!("[{session_id}] could not send info message! {err:?}");
        ws.close().await.expect("can close websocket");
        return;
    }

    info!("[{session_id}] created");

    let (mut sender, mut receiver) = ws.split();
    let (ws_send, mut ws_recv) = mpsc::unbounded_channel();

    let ws_send_task = async {
        while let Some(recv) = ws_recv.recv().await {
            if sender.send(recv).await.is_err() {
                break;
            }
        }
    };

    let ping_task = async {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.tick().await;
        loop {
            interval.tick().await;

            if ws_send.send(Message::Ping(Default::default())).is_err() { break; }
        }
    };

    let recv_message_task = async {
        loop {
            let message = match host_recv.recv().await {
                Ok(message) => message,
                Err(RecvError::Lagged(amount)) => {
                    error!("[{session_id}] receiver lagged by {amount}");
                    continue;
                },
                Err(RecvError::Closed) => {
                    break;
                },
            };
            let message = {
                let mut manager = state.lock().await;
                let Some(session) = manager.get_session_mut(session_id) else {
                    error!("[{session_id}] session already closed");
                    break;
                };
                let (viewer_message, message) = handle_host_message(message, session);
                // an error means there are no viewers
                let _ = viewer_sender.send(viewer_message);
                message
            };

            if ws_send.send(Message::Binary(message.into_bytes())).is_err() { break; }
        }
    };

    let recv_task = async {
        while let Some(message) = receiver.next().await {
            match message {
                Ok(Message::Binary(bytes)) => {
                    if let Some(packet) = ServerboundHostPacket::from_bytes(bytes) {
                        info!("[{session_id}] received message: {packet:?}");

                        let message = match packet {
                            ServerboundHostPacket::StartGame { time_started } => {
                                {
                                    let mut lock = state.lock().await;
                                    let game_state = &mut lock.get_session_mut(session_id).expect("session exists").game_state;
                                    if game_state.time_started.is_some() { continue; }
                                    game_state.time_started = Some(time_started);
                                };
                                info!("[{session_id}] started game");
                                viewer_sender.send(ViewerMessage::GameStart(time_started)).expect("receivers exist for viewer");
                                Some(UserMessage::GameStart)
                            },
                            ServerboundHostPacket::EndGame => {
                                {
                                    let mut lock = state.lock().await;
                                    let game_state = &mut lock.get_session_mut(session_id).expect("session exists").game_state;
                                    if game_state.time_started.is_none() { continue; }
                                    game_state.ended = true;
                                };
                                info!("[{session_id}] ended game");
                                viewer_sender.send(ViewerMessage::GameEnd).expect("receivers exist for viewer");
                                None
                            },
                            ServerboundHostPacket::RevealScore => {
                                {
                                    let mut lock = state.lock().await;
                                    let game_state = &mut lock.get_session_mut(session_id).expect("session exists").game_state;
                                    if !game_state.ended { continue; }
                                    game_state.time_started = None;
                                };
                                info!("[{session_id}] revealed score");
                                viewer_sender.send(ViewerMessage::RevealScore).expect("receivers exist for viewer");
                                Some(UserMessage::GameEnd)
                            },
                            ServerboundHostPacket::PauseGame => {
                                {
                                    let mut lock = state.lock().await;
                                    let game_state = &mut lock.get_session_mut(session_id).expect("session exists").game_state;
                                    if game_state.paused { continue; }
                                    game_state.paused = true;
                                };
                                info!("[{session_id}] paused game");
                                viewer_sender.send(ViewerMessage::GamePause).expect("receivers exist for viewer");
                                None
                            },
                            ServerboundHostPacket::UnpauseGame { time_paused } => {
                                {
                                    let mut lock = state.lock().await;
                                    let game_state = &mut lock.get_session_mut(session_id).expect("session exists").game_state;
                                    if !game_state.paused { continue; }
                                    game_state.time_paused += time_paused;
                                    game_state.paused = false;
                                };
                                info!("[{session_id}] unpaused game (paused for {}s)", time_paused / 1000);
                                viewer_sender.send(ViewerMessage::GameUnpause(time_paused)).expect("receivers exist for viewer");
                                None
                            },
                            ServerboundHostPacket::GameData { .. } => None,
                        };

                        if let Some(message) = message {
                            if user_sender.send(message).is_err() { break; }
                        };
                    } else {
                        error!("[{session_id}] malformed host packet");
                        break;
                    }
                },
                Ok(_) => {},
                Err(err) => {
                    error!("[{session_id}] {err}");
                },
            }
        }
    };

    tokio::select! {
        _ = ping_task => {},
        _ = ws_send_task => {},
        _ = recv_message_task => {},
        _ = recv_task => {},
    };

    {
        let mut lock = state.lock().await;
        lock.close_session(session_id);
    }

    info!("[{session_id}] disconnected");
}

fn handle_host_message(message: HostMessage, session: &mut Session) -> (ViewerMessage, ClientboundHostPacket) {
    match message {
        HostMessage::Score(team, score_id, undo) => {
            let scored = match team {
                Team::Red => &mut session.game_state.red_scored,
                Team::Blue => &mut session.game_state.blue_scored,
            };
            let scores = scored.entry(score_id).or_default();
            if undo {
                scores.undo += 1;
            } else {
                scores.scored += 1;
            }

            (ViewerMessage::Score(team, score_id, undo), ClientboundHostPacket::Score(team, score_id, undo))
        },
    }
}

