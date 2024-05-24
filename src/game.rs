use std::collections::HashMap;

use axum::{response::IntoResponse, Json};
use lazy_static::lazy_static;
use serde::Serialize;

use crate::packet::{Writable, Readable, PacketReader, PacketWriter};

#[derive(Clone, Debug, Serialize)]
pub struct GameData {
    pub duration: GameDuration,
    pub score_points: Box<[ScorePoint]>,
}

impl Readable for GameData {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized {
        let duration = reader.read()?;

        let mut score_points = Vec::new();
        while reader.has_next() {
            if score_points.len() == 256 { break; }
            let score_point = reader.read()?;
            score_points.push(score_point);
        }

        if score_points.len() == 256 { return None; }
        Some(GameData { duration, score_points: score_points.into() })
    }
}

impl Writable for GameData {
    fn write(self, writer: &mut PacketWriter) {
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
    pub blue_scored: HashMap<u8, ScoredRecord>,
    pub red_scored: HashMap<u8, ScoredRecord>,
    pub time_started: Option<u64>,
    pub time_paused: u64,
    pub paused: bool,
    pub ended: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ScoredRecord {
    pub scored: i32,
    pub undo: i32,
}

impl ScoredRecord {
    pub fn one_scored() -> Self {
        ScoredRecord { scored: 1, undo: 0 }
    }

    pub fn one_undo() -> Self {
        ScoredRecord { scored: 0, undo: 1 }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ScorePoint {
    pub name: String,
    pub category: String,
    pub points: i8,
}

impl Readable for ScorePoint {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized {
        let name = reader.read()?;
        let category = reader.read()?;
        let points = reader.read()?;

        Some(ScorePoint { name, category, points })
    }
}

#[derive(Clone, Debug)]
pub struct GameDuration {
    pub secs: u16,
}

impl Readable for GameDuration {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized {
        Some(GameDuration { secs: reader.read()? })
    }
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

#[derive(Serialize, Debug)]
pub struct BuiltinGame {
    pub name: String,
    pub data: GameData,
}

impl Readable for &'static BuiltinGame {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized {
        BUILTIN.games.get(reader.read::<usize>()?)
    }
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

