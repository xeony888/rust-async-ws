use bincode;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use crate::physics::{check_radial_collision, Moveable2d, PUCK_RADIUS};

pub struct Client {
    pub id: usize,
    pub last_ping_time: u64,
}
impl Client {
    pub fn new(id: usize) -> Self {
        return Client {
            id,
            last_ping_time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
    }
    pub fn update_ping(&mut self) {
        self.last_ping_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}
pub type Games = Arc<RwLock<HashMap<usize, Arc<RwLock<Game>>>>>;

pub trait GameLogic: Send + Sync {
    fn game_type(&self) -> u8;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn update(&mut self, elapsed: f64);
}

#[derive(Serialize, Deserialize)]
pub struct Game {
    pub game_type: u8,
    pub last_update_ms: u128,
    #[serde(
        serialize_with = "serialize_logic",
        deserialize_with = "deserialize_logic"
    )]
    pub logic: Box<dyn GameLogic>,
    pub players: Vec<usize>,
}

impl Game {
    pub fn new<G: GameLogic + 'static>(logic: G, players: Vec<usize>) -> Self {
        let game_type = logic.game_type();

        Self {
            game_type,
            last_update_ms: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            logic: Box::new(logic),
            players,
        }
    }

    pub fn downcast<G: 'static>(&self) -> Option<&G> {
        self.logic.as_any().downcast_ref::<G>()
    }

    pub fn downcast_mut<G: 'static>(&mut self) -> Option<&mut G> {
        self.logic.as_any_mut().downcast_mut::<G>()
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Serialization failed")
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).expect("Deserialization failed")
    }
    pub fn update(&mut self) {
        let elapsed = self.get_and_update_duration() as f64;
        self.logic.update(elapsed);
    }
    pub fn get_and_update_duration(&mut self) -> u128 {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let duration = now - self.last_update_ms;
        self.last_update_ms = now;
        return duration;
    }
}

fn serialize_logic<S>(logic: &Box<dyn GameLogic>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::Error;

    match logic.game_type() {
        1 => {
            let soccer = logic
                .as_any()
                .downcast_ref::<SoccerGame>()
                .ok_or_else(|| S::Error::custom("Failed to downcast to SoccerGame"))?;
            bincode::serialize(soccer)
                .map_err(S::Error::custom)?
                .serialize(serializer)
        }
        _ => Err(S::Error::custom("Unknown game type")),
    }
}

fn deserialize_logic<'de, D>(deserializer: D) -> Result<Box<dyn GameLogic>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let bytes = Vec::<u8>::deserialize(deserializer)?;
    let game_type = bytes
        .first()
        .copied()
        .ok_or_else(|| serde::de::Error::custom("Empty data"))?;

    match game_type {
        1 => {
            let soccer: SoccerGame =
                bincode::deserialize(&bytes).map_err(serde::de::Error::custom)?;
            Ok(Box::new(soccer))
        }
        _ => Err(serde::de::Error::custom("Unknown game type")),
    }
}
#[derive(Serialize, Deserialize)]
pub struct SoccerGame {
    pub pucks: [Moveable2d; 2],
    pub ball: Moveable2d,
}

impl SoccerGame {
    pub fn new() -> Self {
        SoccerGame {
            pucks: [Moveable2d::new(), Moveable2d::new()],
            ball: Moveable2d::new(),
        }
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Serialization failed")
    }
}
const SOCCER_FRICTION: f64 = 0.999;
impl GameLogic for SoccerGame {
    fn game_type(&self) -> u8 {
        return 1;
    }
    fn as_any(&self) -> &dyn std::any::Any {
        return self;
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        return self;
    }
    fn update(&mut self, elapsed: f64) {
        for puck in &mut self.pucks {
            puck.update(SOCCER_FRICTION, elapsed);
        }
        self.ball.update(SOCCER_FRICTION, elapsed);
        for i in 0..self.pucks.len() {
            for j in (i + 1)..self.pucks.len() {
                let (left, right) = self.pucks.split_at_mut(j);
                if check_radial_collision(&left[i], &right[0], PUCK_RADIUS) {
                    left[i].collide(&mut right[0]);
                }
            }
        }
    }
}
