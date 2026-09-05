use crate::db::{GameFilter, ScidDatabaseWrapper};
use crate::pgn_db::PgnDatabaseWrapper;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkItem {
    pub category: String,
    pub name: String,
    pub elapsed_ms: f64,
    pub count: usize,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub db_path: String,
    pub format: String,
    pub total_games: usize,
    pub total_players: usize,
    pub total_events: usize,
    pub total_sites: usize,
    pub file_size_mb: f64,
    pub results: Vec<BenchmarkItem>,
    pub total_time_ms: f64,
}

pub fn run_benchmark(db_path: &Path, include_heavy_search: bool) -> Result<BenchmarkReport> {
    let path_str = db_path.to_string_lossy().to_lowercase();
    let is_pgn = path_str.ends_with(".pgn");

    let mut results = Vec::new();
    let overall_start = Instant::now();

    if is_pgn {
        // --- PGN BENCHMARK ---
        let load_start = Instant::now();
        let pgn_db = PgnDatabaseWrapper::open(db_path)?;
        let load_ms = load_start.elapsed().as_secs_f64() * 1000.0;
        let total_games = pgn_db.game_count();
        let file_size_mb = std::fs::metadata(db_path).map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0);

        results.push(BenchmarkItem {
            category: "Database Open".to_string(),
            name: "Open & Memory Map PGN".to_string(),
            elapsed_ms: load_ms,
            count: total_games,
            notes: format!("{:.2} MB on disk", file_size_mb),
        });

        // 1. Sorting benchmarks
        let sort_cols = [
            ("date", "Sort by Date"),
            ("white", "Sort by White Player"),
            ("black", "Sort by Black Player"),
            ("white_elo", "Sort by White ELO"),
            ("eco", "Sort by ECO Code"),
            ("result", "Sort by Result"),
            ("event", "Sort by Event"),
        ];

        for (col, label) in sort_cols {
            let start = Instant::now();
            let filter = GameFilter {
                sort_by: Some(col.to_string()),
                sort_asc: Some(true),
                ..Default::default()
            };
            let (games, total) = pgn_db.query_games(&filter, 0, 50);
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkItem {
                category: "Sorting".to_string(),
                name: label.to_string(),
                elapsed_ms: ms,
                count: total,
                notes: format!("Fetched page 0 ({} items) out of {} games", games.len(), total),
            });
        }

        // 2. Filter benchmarks
        let search_filters = [
            (GameFilter { player: Some("Carlsen".to_string()), ..Default::default() }, "Player search (Carlsen)"),
            (GameFilter { eco: Some("B".to_string()), ..Default::default() }, "ECO prefix search (B)"),
            (GameFilter { result: Some("1-0".to_string()), ..Default::default() }, "Result search (1-0)"),
        ];

        for (flt, label) in search_filters {
            let start = Instant::now();
            let (games, total) = pgn_db.query_games(&flt, 0, 50);
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkItem {
                category: "Filtering".to_string(),
                name: label.to_string(),
                elapsed_ms: ms,
                count: total,
                notes: format!("Matched {} games (retrieved page of {})", total, games.len()),
            });
        }

        // 3. Random seek / PGN retrieval
        if total_games > 0 {
            let seek_count = usize::min(total_games, 500);
            let start = Instant::now();
            let step = usize::max(1, total_games / seek_count);
            for i in (0..total_games).step_by(step).take(seek_count) {
                let _ = pgn_db.get_game_pgn(i);
            }
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            let ops_per_sec = (seek_count as f64 / ms) * 1000.0;
            results.push(BenchmarkItem {
                category: "Game Retrieval".to_string(),
                name: format!("Random PGN Slices ({} games)", seek_count),
                elapsed_ms: ms,
                count: seek_count,
                notes: format!("{:.0} games/sec via memory map", ops_per_sec),
            });
        }

        let total_time_ms = overall_start.elapsed().as_secs_f64() * 1000.0;

        Ok(BenchmarkReport {
            db_path: db_path.to_string_lossy().to_string(),
            format: "PGN".to_string(),
            total_games,
            total_players: pgn_db.names.players.len(),
            total_events: pgn_db.names.events.len(),
            total_sites: pgn_db.names.sites.len(),
            file_size_mb,
            results,
            total_time_ms,
        })
    } else {
        // --- SCID BENCHMARK ---
        let load_start = Instant::now();
        let db = ScidDatabaseWrapper::open(db_path)?;
        let load_ms = load_start.elapsed().as_secs_f64() * 1000.0;
        let stats = db.stats();
        let total_games = stats.total_games;
        let file_size_mb = (stats.index_file_size + stats.namebase_file_size + stats.games_file_size) as f64 / 1_048_576.0;

        results.push(BenchmarkItem {
            category: "Database Open".to_string(),
            name: format!("Read & Parse {} Database", stats.format),
            elapsed_ms: load_ms,
            count: total_games,
            notes: format!("{:.2} MB total (.si + .sn + .sg)", file_size_mb),
        });

        // Precompute rank tables benchmark
        let rank_start = Instant::now();
        let _ = db.get_player_ranks();
        let rank_ms = rank_start.elapsed().as_secs_f64() * 1000.0;
        results.push(BenchmarkItem {
            category: "Indexing".to_string(),
            name: format!("Alphabetical Name Ranking ({} players)", stats.players_count),
            elapsed_ms: rank_ms,
            count: stats.players_count,
            notes: "Parallel radix/quicksort lookup array".to_string(),
        });

        // 1. In-Memory Parallel Sorting Benchmarks
        let sort_cols = [
            ("id", "Sort by ID / Game Index"),
            ("date", "Sort by Date"),
            ("white", "Sort by White Player (Ranked)"),
            ("black", "Sort by Black Player (Ranked)"),
            ("white_elo", "Sort by White ELO"),
            ("black_elo", "Sort by Black ELO"),
            ("eco", "Sort by ECO Code"),
            ("event", "Sort by Event (Ranked)"),
            ("site", "Sort by Site (Ranked)"),
            ("result", "Sort by Result"),
        ];

        for (col, label) in sort_cols {
            let start = Instant::now();
            let filter = GameFilter {
                sort_by: Some(col.to_string()),
                sort_asc: Some(true),
                ..Default::default()
            };
            let (_games, total) = db.query_games(&filter, 0, 50);
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkItem {
                category: "Sorting (Parallel)".to_string(),
                name: label.to_string(),
                elapsed_ms: ms,
                count: total,
                notes: format!("Sorted {} games in {:.2} ms ({:.0} games/s)", total, ms, (total as f64 / (ms / 1000.0))),
            });
        }

        // 2. Parallel Header Filtering Benchmarks
        let search_filters = [
            (GameFilter { player: Some("Kasparov".to_string()), ..Default::default() }, "Player Name (Kasparov)"),
            (GameFilter { eco: Some("B90".to_string()), ..Default::default() }, "Exact ECO (B90 Sicilian)"),
            (GameFilter { date: Some("2024".to_string()), ..Default::default() }, "Year Filter (2024)"),
            (GameFilter { result: Some("1-0".to_string()), ..Default::default() }, "Result Filter (1-0 White Win)"),
        ];

        for (flt, label) in search_filters {
            let start = Instant::now();
            let (_games, total) = db.query_games(&flt, 0, 50);
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkItem {
                category: "Filtering (Parallel)".to_string(),
                name: label.to_string(),
                elapsed_ms: ms,
                count: total,
                notes: format!("Found {} matching games out of {}", total, total_games),
            });
        }

        // 3. Material Search Benchmark
        {
            let mat_filter = crate::position_search::MaterialFilter {
                white_rooks: Some(1),
                black_rooks: Some(1),
                white_queens: Some(0),
                black_queens: Some(0),
                ..Default::default()
            };
            let start = Instant::now();
            let matches = db.search_material(&mat_filter).unwrap_or_default();
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkItem {
                category: "Material Search".to_string(),
                name: "Rook Endgame Search (WR=1, BR=1, WQ=0, BQ=0)".to_string(),
                elapsed_ms: ms,
                count: matches.len(),
                notes: format!("Found {} games in {:.2} ms via bitboard index", matches.len(), ms),
            });
        }

        // 4. Heavy searches if requested or small db
        if include_heavy_search || total_games <= 100_000 {
            let start = Instant::now();
            let pos_res = db.search_position("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", None, None, Some(5));
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            if let Ok(res) = pos_res {
                results.push(BenchmarkItem {
                    category: "Position Search".to_string(),
                    name: "Start Position Search (Max Ply = 5)".to_string(),
                    elapsed_ms: ms,
                    count: res.matches.len(),
                    notes: format!("Found {} matches across {} games in {:.2} ms", res.matches.len(), res.total_games_searched, ms),
                });
            }
        }

        // 5. Game PGN Reconstruction Throughput
        if total_games > 0 {
            let seek_count = usize::min(total_games, 200);
            let start = Instant::now();
            let step = usize::max(1, total_games / seek_count);
            let mut count_ok = 0;
            for i in (0..total_games).step_by(step).take(seek_count) {
                if let Ok(_) = db.game_pgn(i) {
                    count_ok += 1;
                }
            }
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            let ops_per_sec = (count_ok as f64 / ms) * 1000.0;
            results.push(BenchmarkItem {
                category: "PGN Reconstruction".to_string(),
                name: format!("Full Binary Decode & PGN Generation ({} games)", count_ok),
                elapsed_ms: ms,
                count: count_ok,
                notes: format!("{:.0} games/sec decode speed", ops_per_sec),
            });
        }

        let total_time_ms = overall_start.elapsed().as_secs_f64() * 1000.0;

        Ok(BenchmarkReport {
            db_path: db_path.to_string_lossy().to_string(),
            format: format!("{}", stats.format),
            total_games,
            total_players: stats.players_count,
            total_events: stats.events_count,
            total_sites: stats.sites_count,
            file_size_mb,
            results,
            total_time_ms,
        })
    }
}
