use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub public_key: String,
    #[serde(default)]
    pub users: HashMap<String, User>,
    #[serde(default)]
    pub doors: HashMap<String, Door>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Config(UiConfig),
    Challenge(Challenge),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeEvent {
    Config(Config),
    Challenge(Challenge),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiConfig {
    pub public_key: String,
    pub doors: Vec<UiDoor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiDoor {
    pub id: String,
    pub label: String,
}

impl UiDoor {
    pub fn new(id: String, config: Door) -> Self {
        Self {
            id,
            label: config.label,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Challenge {
    pub user: String,
    pub challenge: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    #[serde(default)]
    pub authorize: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Door {
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Fetch(Fetch),
    Solve(Solve),
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Fetch {
    pub user: Option<String>,
    pub door: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Solve {
    pub user: Option<String>,
    pub door: String,
    pub code: String,
}
