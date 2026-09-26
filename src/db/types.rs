use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScidFormat {
    Si4,
    Si5,
}

impl std::fmt::Display for ScidFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScidFormat::Si4 => write!(f, "si4"),
            ScidFormat::Si5 => write!(f, "si5"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSummary {
    pub id: usize,
    pub white: String,
    pub white_elo: u16,
    pub black: String,
    pub black_elo: u16,
    pub result: String,
    pub eco: String,
    pub date: String,
    pub event: String,
    pub site: String,
    pub round: String,
    pub deleted: bool,
    pub non_standard_start: bool,
    pub num_moves: u32,
    pub time_control: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matching_plies: Option<Vec<usize>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_count: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GameFilter {
    pub player: Option<String>,
    pub white: Option<String>,
    pub black: Option<String>,
    pub result: Option<String>,
    pub eco: Option<String>,
    pub date: Option<String>,
    pub event: Option<String>,
    pub site: Option<String>,
    pub include_deleted: Option<bool>,
    pub only_deleted: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_asc: Option<bool>,
    pub fen: Option<String>,
    pub turn: Option<String>,
    pub match_mode: Option<String>,
    pub max_ply: Option<usize>,
    pub start_game: Option<usize>,
    pub end_game: Option<usize>,
    pub material: Option<crate::position_search::MaterialFilter>,
    pub cql: Option<String>,
    pub query: Option<String>,
}

impl GameFilter {
    pub fn same_search_criteria(&self, other: &Self) -> bool {
        self.player == other.player
            && self.white == other.white
            && self.black == other.black
            && self.result == other.result
            && self.eco == other.eco
            && self.date == other.date
            && self.event == other.event
            && self.site == other.site
            && self.include_deleted == other.include_deleted
            && self.only_deleted == other.only_deleted
            && self.fen == other.fen
            && self.turn == other.turn
            && self.match_mode == other.match_mode
            && self.start_game == other.start_game
            && self.end_game == other.end_game
            && self.material == other.material
            && self.cql == other.cql
            && self.query == other.query
    }

    pub fn is_empty(&self) -> bool {
        self.player.as_deref().unwrap_or("").trim().is_empty()
            && self.white.as_deref().unwrap_or("").trim().is_empty()
            && self.black.as_deref().unwrap_or("").trim().is_empty()
            && self.result.as_deref().unwrap_or("").trim().is_empty()
            && self.eco.as_deref().unwrap_or("").trim().is_empty()
            && self.date.as_deref().unwrap_or("").trim().is_empty()
            && self.event.as_deref().unwrap_or("").trim().is_empty()
            && self.site.as_deref().unwrap_or("").trim().is_empty()
            && self.fen.as_deref().unwrap_or("").trim().is_empty()
            && self.cql.as_deref().unwrap_or("").trim().is_empty()
            && self.query.as_deref().unwrap_or("").trim().is_empty()
            && !self.only_deleted.unwrap_or(false)
            && self.material.is_none()
            && self.start_game.is_none()
            && self.end_game.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub format: ScidFormat,
    pub index_path: String,
    pub total_games: usize,
    pub active_games: usize,
    pub deleted_games: usize,
    pub players_count: usize,
    pub events_count: usize,
    pub sites_count: usize,
    pub rounds_count: usize,
    pub index_file_size: u64,
    pub namebase_file_size: u64,
    pub games_file_size: u64,
}

pub fn result_code_to_str(res: u8) -> &'static str {
    match res {
        1 => "1-0",
        2 => "0-1",
        3 => "1/2-1/2",
        _ => "*",
    }
}
