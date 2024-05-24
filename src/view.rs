use std::{time::Duration, convert::Infallible};

use axum::{extract::{Path, State}, response::{Sse, sse::{Event, KeepAlive}, Response, IntoResponse}, http::StatusCode};
use serde::Serialize;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tracing::info;

use crate::{AppState, game::{GameData, GameState}, session_manager::{ViewerMessage, Team}};

pub async fn sse_handler(
    Path(session_id): Path<u32>,
    State(state): State<AppState>,
) -> Response {
    let res = {
        let lock = state.lock().await;
        lock.get_session(session_id).map(|session| {
            let recv = session.viewer.sender.subscribe();
            let data = session.game_data.clone();
            let state = session.game_state.clone();
            (recv, data, state)
        })
    };

    if let Some((viewer_recv, data, state)) = res {
        info!("[{session_id}] viewer connected");

        let init_event = ViewerEvent::SessionInfo { data, state };

        let stream = BroadcastStream::new(viewer_recv)
            .map(|viewer_message| {
                let event = viewer_message.ok().map(ViewerEvent::from);
                Ok(event.and_then(|event| Event::default().json_data(&event).ok()).unwrap_or_default())
            });
        let stream = tokio_stream::once(Ok::<Event, Infallible>(Event::default().json_data(&init_event).expect("valid json")))
            .merge(stream);
        Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("keep-alive-text")).into_response()
    } else {
        (StatusCode::BAD_REQUEST, "invalid session id").into_response()
    }
}

#[derive(Serialize)]
#[serde(tag = "type", content = "content", rename_all = "snake_case")]
enum ViewerEvent {
    SessionInfo { state: GameState, data: GameData },
    Score { team: Team, score_id: u8, undo: bool },
    GameStart { time_started: u64 },
    GameEnd,
    GamePause,
    GameUnpause { paused_time: u64 },
}

impl From<ViewerMessage> for ViewerEvent {
    fn from(value: ViewerMessage) -> Self {
        match value {
            ViewerMessage::Score(team, score_id, undo) => Self::Score { team, score_id, undo },
            ViewerMessage::GameStart(time_started) => Self::GameStart { time_started },
            ViewerMessage::GameEnd => Self::GameEnd,
            ViewerMessage::GamePause => Self::GamePause,
            ViewerMessage::GameUnpause(paused_time) => Self::GameUnpause { paused_time }
        }
    }
}

