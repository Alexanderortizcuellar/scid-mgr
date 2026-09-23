use crate::db::GameFilter;
use crate::position_index::PositionIndex;
use crate::server::{DatabaseBackend, RequestMessage, ResponseMessage};
use crate::tree_index::TreeIndex;
use shakmaty::uci::UciMove;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::Position;
use std::io::{self, Write};
use std::time::Instant;

pub fn handle_opening_tree(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    current_pos_index: &mut Option<PositionIndex>,
    current_tree_index: &mut Option<TreeIndex>,
) -> ResponseMessage {
    let id = req.id;
    let fen = req.params.get("fen").and_then(|v| v.as_str()).unwrap_or("");

    let explicit_game_ids: Option<Vec<usize>> = req
        .params
        .get("game_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let use_search_results = req
        .params
        .get("use_search_results")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let filter_opt: Option<GameFilter> = req
        .params
        .get("filter")
        .or_else(|| req.params.get("params"))
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let include_all_game_ids = req
        .params
        .get("include_all_game_ids")
        .or_else(|| req.params.get("all_game_ids"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let max_sample_ids: Option<usize> = if include_all_game_ids {
        None
    } else {
        req.params
            .get("max_sample_games")
            .or_else(|| req.params.get("max_samples"))
            .or_else(|| req.params.get("sample_games"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .or(Some(20))
    };

    let db = match current_db {
        Some(d) => d,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("No database currently opened".to_string()),
            };
        }
    };

    let mut target_game_ids: Option<Vec<usize>> = explicit_game_ids;
    if target_game_ids.is_none() && use_search_results {
        target_game_ids = match db {
            DatabaseBackend::Scid(s) => s.get_cached_query_indices(),
            DatabaseBackend::Pgn(p) => p.get_cached_query_indices(),
        };
    } else if target_game_ids.is_none() {
        if let Some(ref f) = filter_opt {
            if !f.is_empty() {
                match db {
                    DatabaseBackend::Scid(s) => {
                        let _ = s.query_games(f, 0, 0);
                    }
                    DatabaseBackend::Pgn(p) => {
                        let _ = p.query_games(f, 0, 0);
                    }
                };
                target_game_ids = match db {
                    DatabaseBackend::Scid(s) => s.get_cached_query_indices(),
                    DatabaseBackend::Pgn(p) => p.get_cached_query_indices(),
                };
            }
        }
    }

    let mut report = None;

    // 1. Try fast lookup from .tree.idx file
    if current_tree_index.is_none() {
        let db_path = match db {
            DatabaseBackend::Scid(s) => s.index_path().to_path_buf(),
            DatabaseBackend::Pgn(p) => p.pgn_path.clone(),
        };
        *current_tree_index = TreeIndex::load(&db_path).ok();
    }

    if let Some(tree_idx) = current_tree_index.as_ref() {
        report = tree_idx.query_tree_with_options(fen, target_game_ids.as_deref(), max_sample_ids);
    }

    // 2. Dynamic Fallback: If .tree.idx is missing or position is beyond max depth
    if report.is_none() {
        report = match db {
            DatabaseBackend::Scid(s) => TreeIndex::calculate_tree_for_scid(
                s.entries(),
                s.games_path(),
                fen,
                target_game_ids.as_deref(),
                Some(500),
            ),
            DatabaseBackend::Pgn(p) => TreeIndex::calculate_tree_for_pgn(
                &p.entries,
                p.mmap_ref(),
                fen,
                target_game_ids.as_deref(),
                Some(500),
            ),
        };
    }

    let include_last_played = req
        .params
        .get("include_last_played")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let include_sample_games = req
        .params
        .get("include_sample_games")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    if let Some(mut rep) = report {
        // 1. Resolve sample game IDs from PositionIndex (.pos.idx) for the position
        if (include_sample_games || include_last_played) && rep.sample_game_ids.is_empty() {
            let db_path = match db {
                DatabaseBackend::Scid(s) => s.index_path().to_path_buf(),
                DatabaseBackend::Pgn(p) => p.pgn_path.clone(),
            };
            if current_pos_index.is_none() {
                *current_pos_index = PositionIndex::load(&db_path).ok();
            }
            if let Some(pos_idx) = current_pos_index.as_ref() {
                if let Some(matching_ids) = pos_idx.get_matching_game_ids(rep.zobrist_hash) {
                    let mut filtered_ids: Vec<u32> = if let Some(ref t_ids) = target_game_ids {
                        let t_set: std::collections::HashSet<usize> =
                            t_ids.iter().copied().collect();
                        matching_ids
                            .iter()
                            .filter(|id| t_set.contains(id))
                            .map(|&id| id as u32)
                            .collect()
                    } else {
                        matching_ids.iter().map(|&id| id as u32).collect()
                    };
                    if let Some(limit) = max_sample_ids {
                        filtered_ids.truncate(limit);
                    }
                    rep.sample_game_ids = filtered_ids;
                }
            }
        }

        if let Some(limit) = max_sample_ids {
            rep.sample_game_ids.truncate(limit);
            for m in &mut rep.moves {
                m.sample_game_ids.truncate(limit);
            }
        }

        if include_last_played {
            let target_pos_opt = crate::tree_index::parse_target_position(fen);
            for m in &mut rep.moves {
                if m.sample_game_ids.is_empty() {
                    if let Some((ref pos, _)) = target_pos_opt {
                        if let Ok(uci_move) = m.uci.parse::<UciMove>() {
                            if let Ok(shak_move) = uci_move.to_move(pos) {
                                let mut child_pos = pos.clone();
                                child_pos.play_unchecked(&shak_move);
                                let child_hash: Zobrist64 =
                                    child_pos.zobrist_hash(shakmaty::EnPassantMode::Legal);
                                if let Some(pos_idx) = current_pos_index.as_ref() {
                                    if let Some(child_gids) =
                                        pos_idx.get_matching_game_ids(child_hash.0)
                                    {
                                        let mut gids: Vec<u32> =
                                            child_gids.into_iter().map(|id| id as u32).collect();
                                        if let Some(limit) = max_sample_ids {
                                            gids.truncate(limit.min(5));
                                        } else {
                                            gids.truncate(5);
                                        }
                                        m.sample_game_ids = gids;
                                    }
                                }
                            }
                        }
                    }
                }

                match db {
                    DatabaseBackend::Scid(s) => {
                        let mut max_date: u32 = 0;
                        for &gid in &m.sample_game_ids {
                            let idx = gid as usize;
                            if idx < s.entries().len() {
                                let d = s.entries()[idx].date;
                                if d > max_date {
                                    max_date = d;
                                }
                            }
                        }
                        if max_date > 0 {
                            let d_str = chess_scid_rw::dates::date_to_pgn(max_date);
                            let clean_d = d_str.trim_end_matches(".??").trim_end_matches(".?");
                            if !clean_d.starts_with('?') && !clean_d.is_empty() {
                                m.last_played = Some(clean_d.to_string());
                            }
                        }
                    }
                    DatabaseBackend::Pgn(p) => {
                        let mut max_date_str: Option<String> = None;
                        for &gid in &m.sample_game_ids {
                            let idx = gid as usize;
                            if idx < p.entries.len() {
                                let d = p.entries[idx].date_str();
                                if !d.is_empty()
                                    && !d.starts_with('?')
                                    && max_date_str.as_ref().is_none_or(|cur| d > *cur)
                                {
                                    max_date_str = Some(d);
                                }
                            }
                        }
                        if let Some(d_str) = max_date_str {
                            let clean_d = d_str.trim_end_matches(".??").trim_end_matches(".?");
                            if !clean_d.starts_with('?') && !clean_d.is_empty() {
                                m.last_played = Some(clean_d.to_string());
                            }
                        }
                    }
                }
            }
        }

        if include_sample_games {
            rep.sample_games = match db {
                DatabaseBackend::Scid(s) => rep
                    .sample_game_ids
                    .iter()
                    .take(15)
                    .filter_map(|&gid| s.get_game_summary(gid as usize))
                    .collect(),
                DatabaseBackend::Pgn(p) => rep
                    .sample_game_ids
                    .iter()
                    .take(15)
                    .filter_map(|&gid| {
                        if (gid as usize) < p.entries.len() {
                            Some(p.get_summary(gid as usize))
                        } else {
                            None
                        }
                    })
                    .collect(),
            };
        }

        ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(serde_json::to_value(&rep).unwrap_or_default()),
            error: None,
        }
    } else {
        ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(serde_json::json!({
                "fen": fen,
                "total_games": 0,
                "moves": [],
                "white_wins": 0,
                "draws": 0,
                "black_wins": 0,
                "white_pct": 0.0,
                "draw_pct": 0.0,
                "black_pct": 0.0,
                "sample_game_ids": [],
                "sample_games": [],
            })),
            error: None,
        }
    }
}

pub fn handle_tree_index_status(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    current_tree_index: &Option<TreeIndex>,
) -> ResponseMessage {
    let id = req.id;
    let db = match current_db {
        Some(db) => db,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("No database currently opened".to_string()),
            }
        }
    };

    let (db_path, game_count) = match db {
        DatabaseBackend::Scid(s) => (s.index_path().to_path_buf(), s.game_count()),
        DatabaseBackend::Pgn(p) => (p.pgn_path.clone(), p.game_count()),
    };

    let (status, header) = TreeIndex::check_status(&db_path, game_count);
    let status_str = match status {
        crate::tree_index::IndexStatus::Valid => "valid",
        crate::tree_index::IndexStatus::Outdated => "outdated",
        crate::tree_index::IndexStatus::Missing => "missing",
    };

    ResponseMessage {
        id,
        status: "ok".to_string(),
        data: Some(serde_json::json!({
            "status": status_str,
            "header": header,
            "loaded": current_tree_index.is_some(),
            "unique_positions": current_tree_index.as_ref().map(|i| i.header.unique_positions as usize).unwrap_or(0),
        })),
        error: None,
    }
}

pub fn handle_tree_index_diagnostics(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    current_tree_index: &mut Option<TreeIndex>,
) -> ResponseMessage {
    let id = req.id;
    let db = match current_db {
        Some(db) => db,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("No database currently opened".to_string()),
            }
        }
    };

    let db_path = match db {
        DatabaseBackend::Scid(s) => s.index_path().to_path_buf(),
        DatabaseBackend::Pgn(p) => p.pgn_path.clone(),
    };

    if current_tree_index.is_none() {
        *current_tree_index = TreeIndex::load(&db_path).ok();
    }

    match current_tree_index.as_ref() {
        Some(idx) => match idx.scan_diagnostics() {
            Ok(stats) => ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::to_value(&stats).unwrap_or_default()),
                error: None,
            },
            Err(e) => ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some(format!("Failed to scan tree index diagnostics: {}", e)),
            },
        },
        None => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some("Opening tree index (.tree.idx) not found or not built".to_string()),
        },
    }
}

pub fn handle_build_tree_index(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    current_tree_index: &mut Option<TreeIndex>,
    current_thread_count: usize,
) -> ResponseMessage {
    let id = req.id;
    let db = match current_db {
        Some(db) => db,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("No database currently opened".to_string()),
            }
        }
    };

    let max_ply = req
        .params
        .get("max_ply")
        .and_then(|v| v.as_u64())
        .unwrap_or(24) as usize;
    let min_games = req
        .params
        .get("min_games")
        .and_then(|v| v.as_u64())
        .map(|g| g as usize);
    let threads = req
        .params
        .get("threads")
        .and_then(|v| v.as_u64())
        .map(|t| t as usize)
        .or(Some(current_thread_count));
    let start = Instant::now();

    let res = match db {
        DatabaseBackend::Scid(s) => {
            let games_path = s.games_path().to_path_buf();
            let entries = s.entries();
            let db_path = s.index_path().to_path_buf();
            TreeIndex::build_for_scid(
                &db_path,
                entries,
                &games_path,
                max_ply,
                None,
                min_games,
                threads,
                |scanned, total, positions| {
                    let event_json = serde_json::json!({
                        "event": "build_tree_progress",
                        "data": {
                            "scanned": scanned,
                            "total": total,
                            "positions": positions,
                            "percent": if total > 0 { (scanned as f64 / total as f64) * 100.0 } else { 100.0 }
                        }
                    });
                    if let Ok(line) = serde_json::to_string(&event_json) {
                        let mut out = io::stdout().lock();
                        let _ = writeln!(out, "{}", line);
                        let _ = out.flush();
                    }
                },
            )
        }
        DatabaseBackend::Pgn(p) => {
            let db_path = p.pgn_path.clone();
            let entries = &p.entries;
            let mmap = p.mmap_ref();
            TreeIndex::build_for_pgn(
                &db_path,
                entries,
                mmap,
                max_ply,
                None,
                min_games,
                threads,
                |scanned, total, positions| {
                    let event_json = serde_json::json!({
                        "event": "build_tree_progress",
                        "data": {
                            "scanned": scanned,
                            "total": total,
                            "positions": positions,
                            "percent": if total > 0 { (scanned as f64 / total as f64) * 100.0 } else { 100.0 }
                        }
                    });
                    if let Ok(line) = serde_json::to_string(&event_json) {
                        let mut out = io::stdout().lock();
                        let _ = writeln!(out, "{}", line);
                        let _ = out.flush();
                    }
                },
            )
        }
    };

    match res {
        Ok(idx) => {
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            let unique_positions = idx.header.unique_positions as usize;
            let diagnostics = idx.scan_diagnostics().ok();
            let file_size = std::fs::metadata(&idx.path).map(|m| m.len()).unwrap_or(0);
            *current_tree_index = Some(idx);
            ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::json!({
                    "status": "valid",
                    "unique_positions": unique_positions,
                    "elapsed_ms": elapsed_ms,
                    "file_size": file_size,
                    "diagnostics": diagnostics,
                })),
                error: None,
            }
        }
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Failed to build tree index: {}", e)),
        },
    }
}
