use crate::db::ScidDatabaseWrapper;
use crate::pgn_db::PgnDatabaseWrapper;
use crate::position_index::PositionIndex;
use crate::tree_index::TreeIndex;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseCheckReport {
    pub db_path: String,
    pub db_type: String,
    pub db_valid: bool,
    pub total_games: usize,
    pub active_games: usize,
    pub deleted_games: usize,
    pub players_count: usize,
    pub events_count: usize,
    pub position_index: Option<IndexCheckReport>,
    pub tree_index: Option<IndexCheckReport>,
    pub overall_status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexCheckReport {
    pub path: String,
    pub exists: bool,
    pub valid: bool,
    pub in_sync: bool,
    pub indexed_games: usize,
    pub db_games: usize,
    pub unique_positions: u32,
    pub file_size_bytes: u64,
    pub status_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detailed_diagnostics: Option<serde_json::Value>,
}

pub fn handle_check(db_path: &Path, detailed: bool, json: bool) -> Result<()> {
    let path_str = db_path.to_string_lossy().to_string();
    let is_pgn = path_str.ends_with(".pgn");

    let (db_type, total_games, active_games, deleted_games, players_count, events_count, db_valid) =
        if is_pgn {
            let pgn_db = PgnDatabaseWrapper::open(db_path)
                .with_context(|| format!("Failed to open PGN database: {}", db_path.display()))?;
            let count = pgn_db.game_count();
            let players = pgn_db.names.players.len();
            let events = pgn_db.names.events.len();
            (
                "PGN Database".to_string(),
                count,
                count,
                0,
                players,
                events,
                true,
            )
        } else {
            let scid_db = ScidDatabaseWrapper::open(db_path)
                .with_context(|| format!("Failed to open SCID database: {}", db_path.display()))?;
            let stats = scid_db.stats();
            let fmt_name = format!("SCID {}", scid_db.format());
            (
                fmt_name,
                stats.total_games,
                stats.active_games,
                stats.deleted_games,
                stats.players_count,
                stats.events_count,
                true,
            )
        };

    // Check Position Index (.pos.idx)
    let pos_idx_res = PositionIndex::load(db_path);
    let pos_idx_path = db_path.with_extension("pos.idx");
    let pos_idx_exists = pos_idx_path.exists();

    let (pos_report, pos_diag_struct) = if pos_idx_exists {
        match pos_idx_res {
            Ok(idx) => {
                let indexed_games = idx.header.db_game_count as usize;
                let in_sync = indexed_games == total_games;
                let file_size = std::fs::metadata(&idx.path).map(|m| m.len()).unwrap_or(0);
                let status_message = if in_sync {
                    "Synchronized (Up to date)".to_string()
                } else {
                    format!(
                        "Out of sync: indexed {} games vs database {} games (Rebuild recommended)",
                        indexed_games, total_games
                    )
                };

                let diag = if detailed {
                    idx.scan_diagnostics().ok()
                } else {
                    None
                };

                let detailed_diagnostics = diag.as_ref().and_then(|d| serde_json::to_value(d).ok());

                (
                    Some(IndexCheckReport {
                        path: idx.path.to_string_lossy().to_string(),
                        exists: true,
                        valid: true,
                        in_sync,
                        indexed_games,
                        db_games: total_games,
                        unique_positions: idx.header.unique_positions,
                        file_size_bytes: file_size,
                        status_message,
                        detailed_diagnostics,
                    }),
                    diag,
                )
            }
            Err(e) => (
                Some(IndexCheckReport {
                    path: pos_idx_path.to_string_lossy().to_string(),
                    exists: true,
                    valid: false,
                    in_sync: false,
                    indexed_games: 0,
                    db_games: total_games,
                    unique_positions: 0,
                    file_size_bytes: 0,
                    status_message: format!("Corrupted or incompatible header: {}", e),
                    detailed_diagnostics: None,
                }),
                None,
            ),
        }
    } else {
        (None, None)
    };

    // Check Tree Index (.tree.idx)
    let tree_idx_res = TreeIndex::load(db_path);
    let tree_idx_path = db_path.with_extension("tree.idx");
    let tree_idx_exists = tree_idx_path.exists();

    let (tree_report, tree_diag_struct) = if tree_idx_exists {
        match tree_idx_res {
            Ok(idx) => {
                let indexed_games = idx.header.db_game_count as usize;
                let in_sync = indexed_games == total_games;
                let file_size = std::fs::metadata(&idx.path).map(|m| m.len()).unwrap_or(0);
                let status_message = if in_sync {
                    "Synchronized (Up to date)".to_string()
                } else {
                    format!(
                        "Out of sync: indexed {} games vs database {} games (Rebuild recommended)",
                        indexed_games, total_games
                    )
                };

                let diag = if detailed {
                    idx.scan_diagnostics().ok()
                } else {
                    None
                };

                let detailed_diagnostics = diag.as_ref().and_then(|d| serde_json::to_value(d).ok());

                (
                    Some(IndexCheckReport {
                        path: idx.path.to_string_lossy().to_string(),
                        exists: true,
                        valid: true,
                        in_sync,
                        indexed_games,
                        db_games: total_games,
                        unique_positions: idx.header.unique_positions,
                        file_size_bytes: file_size,
                        status_message,
                        detailed_diagnostics,
                    }),
                    diag,
                )
            }
            Err(e) => (
                Some(IndexCheckReport {
                    path: tree_idx_path.to_string_lossy().to_string(),
                    exists: true,
                    valid: false,
                    in_sync: false,
                    indexed_games: 0,
                    db_games: total_games,
                    unique_positions: 0,
                    file_size_bytes: 0,
                    status_message: format!("Corrupted or incompatible header: {}", e),
                    detailed_diagnostics: None,
                }),
                None,
            ),
        }
    } else {
        (None, None)
    };

    let all_in_sync = pos_report.as_ref().map(|r| r.in_sync).unwrap_or(true)
        && tree_report.as_ref().map(|r| r.in_sync).unwrap_or(true);

    let overall_status = if !db_valid {
        "Database Error".to_string()
    } else if all_in_sync {
        "Clean & Healthy".to_string()
    } else {
        "Attention Needed (Indexes out of sync)".to_string()
    };

    let report = DatabaseCheckReport {
        db_path: path_str.clone(),
        db_type,
        db_valid,
        total_games,
        active_games,
        deleted_games,
        players_count,
        events_count,
        position_index: pos_report,
        tree_index: tree_report,
        overall_status,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    // Terminal Human-Readable Presentation
    println!("============================================================");
    println!(" SCID / PGN Database & Index Health Check                   ");
    println!("============================================================");
    println!(" Database:   {} ({})", path_str, report.db_type);
    println!(
        " Games:      {} total ({} active, {} deleted)",
        report.total_games, report.active_games, report.deleted_games
    );
    println!(
        " Players:    {} | Events: {}",
        report.players_count, report.events_count
    );
    println!(" Status:     [OK] Valid database files");
    println!("------------------------------------------------------------");

    // Position Index
    println!(" Companion Indexes:");
    if let Some(pos) = &report.position_index {
        if pos.valid {
            let status_tag = if pos.in_sync { "[OK]" } else { "[WARN]" };
            println!(
                "  {} Position Index (.pos.idx):\n       Status:           {}\n       Unique Positions: {}\n       Size:             {:.2} MB",
                status_tag,
                pos.status_message,
                pos.unique_positions,
                pos.file_size_bytes as f64 / 1_048_576.0
            );

            if let Some(d) = &pos_diag_struct {
                println!("       --- Detailed Postings Diagnostics ---");
                println!("       Total Postings:     {}", d.total_postings);
                println!(
                    "       Inlined Singletons: {} ({:.1}%)",
                    d.inlined_singletons,
                    if d.total_positions > 0 {
                        (d.inlined_singletons as f64 / d.total_positions as f64) * 100.0
                    } else {
                        0.0
                    }
                );
                println!("       Delta-Varint Sets:  {}", d.delta_varint_count);
                println!(
                    "       Frequency: 1-10 games: {}, 11-100: {}, 101-1k: {}, 1k-10k: {}, 10k+: {}",
                    d.bucket_1_10, d.bucket_11_100, d.bucket_101_1k, d.bucket_1k_10k, d.bucket_10k_100k + d.bucket_100k_plus
                );
            }
        } else {
            println!(
                "  [ERROR] Position Index (.pos.idx):\n       Status: {}",
                pos.status_message
            );
        }
    } else {
        println!(
            "  [--] Position Index (.pos.idx): Not generated (Build with 'scid-mgr build-pos-idx')"
        );
    }

    // Tree Index
    if let Some(tree) = &report.tree_index {
        if tree.valid {
            let status_tag = if tree.in_sync { "[OK]" } else { "[WARN]" };
            println!(
                "  {} Opening Tree Index (.tree.idx):\n       Status:           {}\n       Unique Positions: {}\n       Size:             {:.2} MB",
                status_tag,
                tree.status_message,
                tree.unique_positions,
                tree.file_size_bytes as f64 / 1_048_576.0
            );

            if let Some(d) = &tree_diag_struct {
                println!("       --- Detailed Tree Diagnostics ---");
                println!("       Total Stored Moves: {}", d.total_tree_moves);
                println!(
                    "       Payload Bytes:      {} ({:.2} MB)",
                    d.bytes_total,
                    d.bytes_total as f64 / 1_048_576.0
                );
                println!(
                    "       Frequency: 1-10 games: {}, 11-100: {}, 101-1k: {}, 1k-10k: {}, 10k+: {}",
                    d.bucket_1_10, d.bucket_11_100, d.bucket_101_1k, d.bucket_1k_10k, d.bucket_10k_100k + d.bucket_100k_plus
                );
            }
        } else {
            println!(
                "  [ERROR] Opening Tree Index (.tree.idx):\n       Status: {}",
                tree.status_message
            );
        }
    } else {
        println!("  [--] Opening Tree Index (.tree.idx): Not generated (Build with 'scid-mgr build-tree')");
    }

    println!("============================================================");
    println!(" Summary: {}", report.overall_status);
    println!("============================================================");

    Ok(())
}
