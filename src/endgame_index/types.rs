use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturePopularity {
    pub id: String,
    pub bit: u8,
    pub name: String,
    pub short_name: String,
    pub gbr_code: Option<String>,
    pub category_id: String,
    pub game_count: usize,
    pub percentage: f64,
    pub white_wins: usize,
    pub draws: usize,
    pub black_wins: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryPopularity {
    pub category_id: String,
    pub name: String,
    pub total_games: usize,
    pub percentage: f64,
    pub features: Vec<FeaturePopularity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndgamePopularityReport {
    pub db_path: String,
    pub total_db_games: usize,
    pub games_reaching_position: usize,
    pub position_filtered: bool,
    pub fen: Option<String>,
    pub categories: Vec<CategoryPopularity>,
    pub features: Vec<FeaturePopularity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureQueryReport {
    pub feature_id: String,
    pub bit: u8,
    pub name: String,
    pub short_name: String,
    pub gbr_code: Option<String>,
    pub category_id: String,
    pub total_db_games: usize,
    pub matching_games_count: usize,
    pub percentage: f64,
    pub sample_game_ids: Vec<u32>,
}
