//! Persistent data (homes, warps, balances, ...) plus short-lived runtime state.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Loc {
    pub world: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerData {
    pub name: String,
    pub homes: BTreeMap<String, Loc>,
    pub balance: f64,
    pub last_seen: u64,
    pub muted: bool,
    pub nick: Option<String>,
    pub vanished: bool,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Data {
    /// Keyed by the player's UUID string.
    pub players: BTreeMap<String, PlayerData>,
    pub warps: BTreeMap<String, Loc>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub starting_balance: f64,
    pub currency_symbol: String,
    /// 0 = unlimited homes.
    pub max_homes: usize,
    pub tpa_timeout_secs: u64,
    pub nick_prefix: String,
    pub show_motd_on_join: bool,
    pub motd: Vec<String>,
    pub rules: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            starting_balance: 0.0,
            currency_symbol: "$".into(),
            max_homes: 3,
            tpa_timeout_secs: 120,
            nick_prefix: "~".into(),
            show_motd_on_join: true,
            motd: vec![
                "&6Welcome, &c{player}&6!".into(),
                "&6Type &c/rules &6to read the server rules.".into(),
            ],
            rules: vec![
                "&61. Be kind to other players.".into(),
                "&62. No griefing or stealing.".into(),
                "&63. No cheating.".into(),
            ],
        }
    }
}

pub struct TpaRequest {
    pub from: String,
    /// `true` for /tpahere (the *target* teleports to the requester).
    pub here: bool,
    pub expires: u64,
}

#[derive(Default)]
pub struct State {
    pub folder: String,
    pub config: Config,
    pub data: Data,
    // ---- runtime only, never saved ----
    pub back: HashMap<String, Loc>,
    /// Pending requests keyed by the *target's* UUID.
    pub tpa: HashMap<String, TpaRequest>,
    pub last_msg: HashMap<String, String>,
    pub afk: HashSet<String>,
}

static STATE: OnceLock<Mutex<State>> = OnceLock::new();

/// Run `f` with exclusive access to the global plugin state.
pub fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let m = STATE.get_or_init(|| Mutex::new(State::default()));
    let mut guard = m.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl State {
    fn path(&self, file: &str) -> String {
        format!("{}/{}", self.folder, file)
    }

    /// (Re)load config.json and data.json, writing a default config if none exists.
    pub fn load(&mut self) {
        let cfg_path = self.path("config.json");
        match fs::read_to_string(&cfg_path) {
            Ok(s) => match serde_json::from_str(&s) {
                Ok(c) => self.config = c,
                Err(e) => tracing::warn!("config.json is invalid ({e}); using defaults"),
            },
            Err(_) => {
                self.config = Config::default();
                if let Ok(s) = serde_json::to_string_pretty(&self.config) {
                    let _ = fs::write(&cfg_path, s);
                }
            }
        }
        if let Ok(s) = fs::read_to_string(self.path("data.json")) {
            match serde_json::from_str(&s) {
                Ok(d) => self.data = d,
                Err(e) => tracing::warn!("data.json is invalid ({e}); starting empty"),
            }
        }
    }

    pub fn save(&self) {
        match serde_json::to_string_pretty(&self.data) {
            Ok(s) => {
                if let Err(e) = fs::write(self.path("data.json"), s) {
                    tracing::error!("failed to save data.json: {e}");
                }
            }
            Err(e) => tracing::error!("failed to serialise data: {e}"),
        }
    }

    /// Get (or create) the record for a player.
    pub fn pd(&mut self, id: &str, name: &str) -> &mut PlayerData {
        let start = self.config.starting_balance;
        let d = self.data.players.entry(id.to_string()).or_insert_with(|| PlayerData {
            name: name.to_string(),
            balance: start,
            ..Default::default()
        });
        if !name.is_empty() {
            d.name = name.to_string();
        }
        d
    }

    /// Case-insensitive lookup of a (possibly offline) player by name.
    pub fn find_by_name(&self, name: &str) -> Option<(String, &PlayerData)> {
        self.data
            .players
            .iter()
            .find(|(_, d)| d.name.eq_ignore_ascii_case(name))
            .map(|(id, d)| (id.clone(), d))
    }

    pub fn money(&self, amount: f64) -> String {
        format!("{}{:.2}", self.config.currency_symbol, amount)
    }
}
