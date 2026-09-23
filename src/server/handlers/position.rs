use crate::position_index::PositionIndex;
use crate::server::{DatabaseBackend, RequestMessage, ResponseMessage};
use std::io::{self, Write};

pub fn handle_search_position(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    current_pos_index: &mut Option<PositionIndex>,
    thread_pool: &rayon::ThreadPool,
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

    let fen = match req
        .params
        .get("fen")
        .or_else(|| req.params.get("params").and_then(|p| p.get("fen")))
        .and_then(|v| v.as_str())
    {
        Some(f) => f,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Missing 'fen' parameter".to_string()),
            };
        }
    };

    let turn_param = req
        .params
        .get("turn")
        .or_else(|| req.params.get("params").and_then(|p| p.get("turn")))
        .and_then(|v| v.as_str());

    let mode_param = req
        .params
        .get("match_mode")
        .or_else(|| req.params.get("mode"))
        .or_else(|| {
            req.params
                .get("params")
                .and_then(|p| p.get("match_mode").or_else(|| p.get("mode")))
        })
        .and_then(|v| v.as_str());

    let is_exact = mode_param
        .map(|m| {
            let m = m.to_lowercase();
            m == "exact" || m == "auto" || m.is_empty()
        })
        .unwrap_or(true);

    // ⚡ Instant Sub-Millisecond candidate lookup if PositionIndex is active
    if is_exact && turn_param.is_none() {
        if current_pos_index.is_none() {
            let db_path = match db {
                DatabaseBackend::Scid(s) => s.index_path().to_path_buf(),
                DatabaseBackend::Pgn(p) => p.pgn_path.clone(),
            };
            *current_pos_index = PositionIndex::load(&db_path).ok();
        }

        if let Some(pos_idx) = current_pos_index.as_ref() {
            if let Some((_pos, zobrist_hash)) = crate::position_index::parse_target_position(fen) {
                if let Some(game_ids) = pos_idx.get_all_position_games(zobrist_hash) {
                    let matches: Vec<crate::position_search::PositionMatch> = game_ids
                        .into_iter()
                        .map(|gid| crate::position_search::PositionMatch {
                            game_id: gid,
                            ply: 0,
                        })
                        .collect();
                    let total_games = match db {
                        DatabaseBackend::Scid(s) => s.game_count(),
                        DatabaseBackend::Pgn(p) => p.game_count(),
                    };
                    let res = crate::position_search::PositionSearchResult {
                        target_fen: fen.to_string(),
                        target_hash: zobrist_hash,
                        matches,
                        total_games_searched: total_games,
                        elapsed_ms: 0.05,
                    };
                    return ResponseMessage {
                        id,
                        status: "ok".to_string(),
                        data: Some(serde_json::to_value(&res).unwrap_or_default()),
                        error: None,
                    };
                }
            }
        }
    }

    let max_ply = req
        .params
        .get("max_ply")
        .or_else(|| req.params.get("params").and_then(|p| p.get("max_ply")))
        .and_then(|v| v.as_u64())
        .map(|p| p as usize);

    match db {
        DatabaseBackend::Scid(s) => {
            let res = thread_pool.install(|| {
                s.search_position_with_progress(fen, turn_param, mode_param, max_ply, |scanned, total, matches_len| {
                    let event_json = serde_json::json!({
                        "event": "search_progress",
                        "data": {
                            "scanned": scanned,
                            "total": total,
                            "matches": matches_len,
                            "percent": if total > 0 { (scanned as f64 / total as f64) * 100.0 } else { 100.0 }
                        }
                    });
                    if let Ok(line) = serde_json::to_string(&event_json) {
                        let mut out = io::stdout().lock();
                        let _ = writeln!(out, "{}", line);
                        let _ = out.flush();
                    }
                })
            });
            match res {
                Ok(res) => ResponseMessage {
                    id,
                    status: "ok".to_string(),
                    data: Some(serde_json::to_value(&res).unwrap_or_default()),
                    error: None,
                },
                Err(e) => ResponseMessage {
                    id,
                    status: "error".to_string(),
                    data: None,
                    error: Some(format!("Position search failed: {}", e)),
                },
            }
        }
        DatabaseBackend::Pgn(p) => {
            let res = thread_pool.install(|| {
                p.search_position(fen, turn_param, mode_param, max_ply, |scanned, total, matches_len| {
                    let event_json = serde_json::json!({
                        "event": "search_progress",
                        "data": {
                            "scanned": scanned,
                            "total": total,
                            "matches": matches_len,
                            "percent": if total > 0 { (scanned as f64 / total as f64) * 100.0 } else { 100.0 }
                        }
                    });
                    if let Ok(line) = serde_json::to_string(&event_json) {
                        let mut out = io::stdout().lock();
                        let _ = writeln!(out, "{}", line);
                        let _ = out.flush();
                    }
                })
            });
            match res {
                Ok(res) => ResponseMessage {
                    id,
                    status: "ok".to_string(),
                    data: Some(serde_json::to_value(&res).unwrap_or_default()),
                    error: None,
                },
                Err(e) => ResponseMessage {
                    id,
                    status: "error".to_string(),
                    data: None,
                    error: Some(format!("Position search failed: {}", e)),
                },
            }
        }
    }
}

pub fn handle_search_material(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
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

    let filter: crate::position_search::MaterialFilter =
        serde_json::from_value(req.params.clone()).unwrap_or_default();
    let start = std::time::Instant::now();
    match db {
        DatabaseBackend::Scid(s) => match s.search_material(&filter) {
            Ok(matches) => {
                let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                ResponseMessage {
                    id,
                    status: "ok".to_string(),
                    data: Some(serde_json::json!({
                        "matches": matches,
                        "match_count": matches.len(),
                        "total_games": s.game_count(),
                        "elapsed_ms": elapsed_ms,
                    })),
                    error: None,
                }
            }
            Err(e) => ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some(format!("Material search failed: {}", e)),
            },
        },
        DatabaseBackend::Pgn(p) => {
            let res = p.search_material(&filter, |scanned, total, matches_len| {
                let event_json = serde_json::json!({
                    "event": "search_progress",
                    "data": {
                        "scanned": scanned,
                        "total": total,
                        "matches": matches_len,
                        "percent": if total > 0 { (scanned as f64 / total as f64) * 100.0 } else { 100.0 }
                    }
                });
                if let Ok(line) = serde_json::to_string(&event_json) {
                    let mut out = io::stdout().lock();
                    let _ = writeln!(out, "{}", line);
                    let _ = out.flush();
                }
            });
            match res {
                Ok(matches) => {
                    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                    ResponseMessage {
                        id,
                        status: "ok".to_string(),
                        data: Some(serde_json::json!({
                            "matches": matches,
                            "match_count": matches.len(),
                            "total_games": p.game_count(),
                            "elapsed_ms": elapsed_ms,
                        })),
                        error: None,
                    }
                }
                Err(e) => ResponseMessage {
                    id,
                    status: "error".to_string(),
                    data: None,
                    error: Some(format!("Material search failed: {}", e)),
                },
            }
        }
    }
}
