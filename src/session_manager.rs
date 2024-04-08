use std::collections::HashMap;

use tokio::sync::broadcast::{Receiver, Sender, self};
use serde::{Deserialize, Serialize};

use crate::game::{GameData, GameState};

pub struct SessionManager {
    sessions: HashMap<u32, Session>,
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager { sessions: HashMap::new() }
    }

    pub fn new_session(&mut self, id: u32, game_data: GameData) -> &Session {
        let (host_sender, host_recv) = broadcast::channel(10);
        let (user_sender, user_recv) = broadcast::channel(10);
        let (viewer_sender, viewer_recv) = broadcast::channel(10);

        let host = Host { sender: host_sender, recv: host_recv };
        let user = User { sender: user_sender, recv: user_recv };
        let viewer = Viewer { sender: viewer_sender, recv: viewer_recv };

        self.sessions.entry(id).or_insert(Session { host, user, viewer, game_data, game_state: Default::default() })
    }

    pub fn get_session(&self, id: u32) -> Option<&Session> {
        self.sessions.get(&id)
    }

    pub fn get_session_mut(&mut self, id: u32) -> Option<&mut Session> {
        self.sessions.get_mut(&id)
    }

    pub fn close_session(&mut self, id: u32) -> Option<Session> {
        self.sessions.remove(&id).inspect(|session| { let _ = session.user.sender.send(UserMessage::Close); })
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Session {
    pub host: Host,
    pub user: User,
    pub viewer: Viewer,
    pub game_data: GameData,
    pub game_state: GameState,
}

pub struct Host {
    pub recv: Receiver<HostMessage>,
    pub sender: Sender<HostMessage>,
}

pub struct User {
    pub recv: Receiver<UserMessage>,
    pub sender: Sender<UserMessage>,
}

pub struct Viewer {
    pub recv: Receiver<ViewerMessage>,
    pub sender: Sender<ViewerMessage>,
}

#[derive(Clone, Copy)]
pub struct HostMessage {
    pub team: Team,
    pub message: HostMessageType,
}

#[derive(Clone, Copy)]
pub enum HostMessageType {
    Score(Team, u8),
}

#[derive(Clone, Copy)]
pub enum UserMessage {
    Close,
    GameStart,
    GameEnd,
}

#[derive(Clone, Copy, Debug)]
pub enum ViewerMessage {
    Score(Team, u8),
    GameStart(u64),
    GameEnd,
    GamePause,
    GameUnpause(u64),
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Team {
    Blue,
    Red,
}

