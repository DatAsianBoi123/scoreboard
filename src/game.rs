use std::collections::HashMap;

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct GameData {
    pub duration: GameDuration,
    pub score_points: Box<[ScorePoint]>,
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

macro_rules! score_points {
    ($($c: literal : { $($n: literal : $p: literal),+ $(,)? }),* $(,)?) => {
        Box::new([$($(ScorePoint { name: $n.to_string(), category: $c.to_string(), points: $p }),+),*])
    };
}

fn summer_camp_2023() -> GameData {
    GameData {
        duration: GameDuration::from_min(5),
        score_points: score_points! {
            "cube": { "cube": 2 },
            "cone": { "cone": 3 },
            "penalty": {
                "hit penalty": -2,
                "side penalty": -3,
            },
        },
    }
}

fn summer_camp_2024() -> GameData {
    GameData {
        duration: GameDuration::from_min(5),
        score_points: score_points! {
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
            }
        },
    }
}

pub fn from_id(id: u8) -> Option<GameData> {
    match id {
        0 => Some(summer_camp_2023()),
        1 => Some(summer_camp_2024()),
        _ => None,
    }
}

