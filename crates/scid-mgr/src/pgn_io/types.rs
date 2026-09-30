use chess_scid_rw::names::NameTables;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub percent: f64,
    pub imported_games: usize,
    pub errors: usize,
    pub speed_gps: f64,
    pub eta_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportProgress {
    pub exported_games: usize,
    pub total_games: usize,
    pub percent: f64,
    pub speed_gps: f64,
    pub eta_seconds: u64,
}

#[derive(Default, Debug, Clone)]
pub struct RawPgnTags {
    pub white: String,
    pub black: String,
    pub event: String,
    pub site: String,
    pub round: String,
    pub date: u32,
    pub result: u8,
    pub eco_code: u16,
    pub white_elo: u16,
    pub black_elo: u16,
    pub fen: Option<String>,
}

pub struct FastNameTables {
    pub players: Vec<String>,
    pub events: Vec<String>,
    pub sites: Vec<String>,
    pub rounds: Vec<String>,
    player_map: HashMap<String, u32>,
    event_map: HashMap<String, u32>,
    site_map: HashMap<String, u32>,
    round_map: HashMap<String, u32>,
}

impl FastNameTables {
    pub fn from_name_tables(tables: &NameTables) -> Self {
        let mut player_map = HashMap::with_capacity(tables.players.len() * 2);
        for (i, p) in tables.players.iter().enumerate() {
            player_map.insert(p.clone(), i as u32);
        }

        let mut event_map = HashMap::with_capacity(tables.events.len() * 2);
        for (i, e) in tables.events.iter().enumerate() {
            event_map.insert(e.clone(), i as u32);
        }

        let mut site_map = HashMap::with_capacity(tables.sites.len() * 2);
        for (i, s) in tables.sites.iter().enumerate() {
            site_map.insert(s.clone(), i as u32);
        }

        let mut round_map = HashMap::with_capacity(tables.rounds.len() * 2);
        for (i, r) in tables.rounds.iter().enumerate() {
            round_map.insert(r.clone(), i as u32);
        }

        Self {
            players: tables.players.clone(),
            events: tables.events.clone(),
            sites: tables.sites.clone(),
            rounds: tables.rounds.clone(),
            player_map,
            event_map,
            site_map,
            round_map,
        }
    }

    #[inline]
    pub fn player_id(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.player_map.get(name) {
            return id;
        }
        let id = self.players.len() as u32;
        let owned = name.to_string();
        self.players.push(owned.clone());
        self.player_map.insert(owned, id);
        id
    }

    #[inline]
    pub fn event_id(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.event_map.get(name) {
            return id;
        }
        let id = self.events.len() as u32;
        let owned = name.to_string();
        self.events.push(owned.clone());
        self.event_map.insert(owned, id);
        id
    }

    #[inline]
    pub fn site_id(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.site_map.get(name) {
            return id;
        }
        let id = self.sites.len() as u32;
        let owned = name.to_string();
        self.sites.push(owned.clone());
        self.site_map.insert(owned, id);
        id
    }

    #[inline]
    pub fn round_id(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.round_map.get(name) {
            return id;
        }
        let id = self.rounds.len() as u32;
        let owned = name.to_string();
        self.rounds.push(owned.clone());
        self.round_map.insert(owned, id);
        id
    }

    pub fn to_name_tables(&self) -> NameTables {
        NameTables {
            players: self.players.clone(),
            events: self.events.clone(),
            sites: self.sites.clone(),
            rounds: self.rounds.clone(),
        }
    }
}
