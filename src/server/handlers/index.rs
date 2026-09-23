use crate::position_index::{IndexStatus, PositionIndex};
use crate::server::{DatabaseBackend, RequestMessage, ResponseMessage};
use std::io::{self, Write};
use std::time::Instant;

pub fn handle_unload_pos_index(
    req: &RequestMessage,
    current_pos_index: &mut Option<PositionIndex>,
) -> ResponseMessage {
    *current_pos_index = None;
    ResponseMessage {
        id: req.id,
        status: "ok".to_string(),
        data: Some(serde_json::json!({ "unloaded": true })),
        error: None,
    }
}

pub fn handle_pos_index_status(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    current_pos_index: &Option<PositionIndex>,
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

    let (status, header) = PositionIndex::check_status(&db_path, game_count);
    let status_str = match status {
        IndexStatus::Valid => "valid",
        IndexStatus::Outdated => "outdated",
        IndexStatus::Missing => "missing",
    };

    ResponseMessage {
        id,
        status: "ok".to_string(),
        data: Some(serde_json::json!({
            "status": status_str,
            "header": header,
            "loaded": current_pos_index.is_some(),
            "unique_positions": current_pos_index.as_ref().map(|i| i.header.unique_positions as usize).unwrap_or(0),
        })),
        error: None,
    }
}

pub fn handle_pos_index_diagnostics(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    current_pos_index: &mut Option<PositionIndex>,
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

    if current_pos_index.is_none() {
        *current_pos_index = PositionIndex::load(&db_path).ok();
    }

    match current_pos_index.as_ref() {
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
                error: Some(format!("Failed to scan position index diagnostics: {}", e)),
            },
        },
        None => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some("Position index (.pos.idx) not found or not built".to_string()),
        },
    }
}

pub fn handle_build_pos_index(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    current_pos_index: &mut Option<PositionIndex>,
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
    let max_games = req
        .params
        .get("max_games")
        .or_else(|| req.params.get("max_game_ids"))
        .and_then(|v| v.as_u64())
        .map(|g| g as usize);
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
            PositionIndex::build_for_scid(
                &db_path,
                entries,
                &games_path,
                max_ply,
                max_games,
                min_games,
                threads,
                |scanned, total, positions| {
                    let event_json = serde_json::json!({
                        "event": "build_pos_index_progress",
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
            PositionIndex::build_for_pgn(
                &db_path,
                entries,
                mmap,
                max_ply,
                max_games,
                min_games,
                threads,
                |scanned, total, positions| {
                    let event_json = serde_json::json!({
                        "event": "build_pos_index_progress",
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
            *current_pos_index = Some(idx);
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
            error: Some(format!("Failed to build position index: {}", e)),
        },
    }
}
