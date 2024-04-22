use std::collections::HashMap;

use axum::{response::IntoResponse, Json};
use lazy_static::lazy_static;
use serde::Serialize;

use crate::packet::Writable;

#[derive(Clone, Debug, Serialize)]
pub struct GameData {
    pub duration: GameDuration,
    pub score_points: Box<[ScorePoint]>,
}

impl Writable for GameData {
    fn write(self, writer: &mut crate::packet::PacketWriter) {
        writer.write(self.duration.secs);
        for ScorePoint { name, category, points } in Vec::from(self.score_points) {
            writer.write(name);
            writer.write(category);
            writer.write(points);
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct GameState {
    pub blue_scored: HashMap<u8, i32>,
    pub red_scored: HashMap<u8, i32>,
    pub time_started: Option<u64>,
    pub time_paused: u64,
    pub paused: bool,
    pub ended: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScorePoint {
    pub name: String,
    pub category: String,
    pub points: i8,
}

#[derive(Clone, Debug)]
pub struct GameDuration {
    pub secs: u16,
}

impl Serialize for GameDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer
    {
        serializer.serialize_u16(self.secs)
    }
}

impl GameDuration {
    pub fn from_min(min: u16) -> Self {
        GameDuration { secs: min * 60 }
    }
}

#[derive(Serialize)]
pub struct BuiltinGame {
    pub name: String,
    pub data: GameData,
}

#[derive(Serialize)]
pub struct BuiltinGames {
    pub games: Box<[BuiltinGame]>,
}

lazy_static! {
    pub static ref BUILTIN: BuiltinGames = BuiltinGames::default();
}

macro_rules! builtin_games {
    ($($n: literal : { duration: $d: expr, data: { $($c: literal : { $($s: literal : $p: literal),+ $(,)? }),* $(,)? } $(,)? }),* $(,)?) => {
        impl Default for crate::game::BuiltinGames {
            fn default() -> Self {
                let mut games = Vec::new();
                $({
                    let data = crate::game::GameData {
                        duration: $d,
                        score_points: Box::new([$($(ScorePoint { name: $s.to_string(), category: $c.to_string(), points: $p }),+),*]),
                    };
                    games.push(crate::game::BuiltinGame { name: $n.to_string(), data });
                })*
                Self {
                    games: games.into(),
                }
            }
        }
    };
}

builtin_games! {
    "FRC Rapid React 2023": {
        duration: GameDuration::from_min(5),
        data: {
            "cube": { "cube": 2 },
            "cone": { "cone": 3 },
            "penalty": {
                "hit penalty": -2,
                "side penalty": -3,
            },
        },
    },
    "FRC Crescendo 2024": {
        duration: GameDuration::from_min(5),
        data: {
            "amp": { "amp": 1 },
            "speaker": { "speaker": 3 },
            "stage": {
                "park": 1,
                "climb": 2,
                "buddy climb": 4,
            },
            "penalty": {
                "hit penalty": -2,
                "side penalty": -3,
            },
        }
    }
}

pub async fn get_all_builtin() -> impl IntoResponse {
    Json(&BUILTIN.games)
}

