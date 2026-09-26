use crate::db::{GameFilter, GameSummary, ScidDatabaseWrapper, ScidFormat};
use crate::pgn_db::PgnDatabaseWrapper;
use crate::position_index::{IndexStatus, PositionIndex};
use crate::server::{DatabaseBackend, RequestMessage, ResponseMessage};
use crate::tree_index::TreeIndex;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn handle_open_db(
    req: &RequestMessage,
    current_db: &mut Option<DatabaseBackend>,
    current_pos_index: &mut Option<PositionIndex>,
    current_tree_index: &mut Option<TreeIndex>,
) -> ResponseMessage {
    let id = req.id;
    let path_str = match req
        .params
        .get("path")
        .or_else(|| req.params.get("params").and_then(|p| p.get("path")))
        .and_then(|v| v.as_str())
    {
        Some(p) => p,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Missing 'path' parameter".to_string()),
            }
        }
    };

    let path = Path::new(path_str);
    if path_str.to_lowercase().ends_with(".pgn") {
        match PgnDatabaseWrapper::open(path) {
            Ok(pgn) => {
                let total_games = pgn.game_count();
                let pgn_path_str = pgn.pgn_path.to_string_lossy().to_string();

                let (idx_status, header_opt) = PositionIndex::check_status(path, total_games);
                let status_str = match idx_status {
                    IndexStatus::Valid => "valid",
                    IndexStatus::Outdated => "outdated",
                    IndexStatus::Missing => "missing",
                };
                let pos_count = header_opt.as_ref().map(|h| h.unique_positions).unwrap_or(0);

                let (tree_status, tree_header_opt) = TreeIndex::check_status(path, total_games);
                let tree_status_str = match tree_status {
                    crate::tree_index::IndexStatus::Valid => "valid",
                    crate::tree_index::IndexStatus::Outdated => "outdated",
                    crate::tree_index::IndexStatus::Missing => "missing",
                };
                let tree_pos_count = tree_header_opt
                    .as_ref()
                    .map(|h| h.unique_positions)
                    .unwrap_or(0);

                *current_pos_index = None;
                *current_tree_index = None;

                *current_db = Some(DatabaseBackend::Pgn(pgn));
                ResponseMessage {
                    id,
                    status: "ok".to_string(),
                    data: Some(serde_json::json!({
                        "stats": {
                            "format": "pgn",
                            "total_games": total_games,
                            "active_games": total_games,
                            "deleted_games": 0,
                            "players_count": 0,
                            "events_count": 0,
                            "sites_count": 0,
                            "rounds_count": 0,
                            "path": pgn_path_str,
                            "pos_index_status": status_str,
                            "pos_index_unique_positions": pos_count,
                            "tree_index_status": tree_status_str,
                            "tree_index_unique_positions": tree_pos_count,
                        },
                        "pos_index_status": status_str,
                        "pos_index_unique_positions": pos_count,
                        "tree_index_status": tree_status_str,
                        "tree_index_unique_positions": tree_pos_count,
                        "format": "pgn",
                        "total_games": total_games
                    })),
                    error: None,
                }
            }
            Err(e) => ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some(format!("Failed to open PGN database: {}", e)),
            },
        }
    } else {
        match ScidDatabaseWrapper::open(path) {
            Ok(db) => {
                let total_games = db.game_count();
                let mut stats = serde_json::to_value(db.stats()).unwrap_or_default();

                let (idx_status, header_opt) = PositionIndex::check_status(path, total_games);
                let status_str = match idx_status {
                    IndexStatus::Valid => "valid",
                    IndexStatus::Outdated => "outdated",
                    IndexStatus::Missing => "missing",
                };
                let pos_count = header_opt.as_ref().map(|h| h.unique_positions).unwrap_or(0);

                let (tree_status, tree_header_opt) = TreeIndex::check_status(path, total_games);
                let tree_status_str = match tree_status {
                    crate::tree_index::IndexStatus::Valid => "valid",
                    crate::tree_index::IndexStatus::Outdated => "outdated",
                    crate::tree_index::IndexStatus::Missing => "missing",
                };
                let tree_pos_count = tree_header_opt
                    .as_ref()
                    .map(|h| h.unique_positions)
                    .unwrap_or(0);

                *current_pos_index = None;
                *current_tree_index = None;

                if let Some(obj) = stats.as_object_mut() {
                    obj.insert(
                        "pos_index_status".to_string(),
                        serde_json::json!(status_str),
                    );
                    obj.insert(
                        "pos_index_unique_positions".to_string(),
                        serde_json::json!(pos_count),
                    );
                    obj.insert(
                        "tree_index_status".to_string(),
                        serde_json::json!(tree_status_str),
                    );
                    obj.insert(
                        "tree_index_unique_positions".to_string(),
                        serde_json::json!(tree_pos_count),
                    );
                }

                *current_db = Some(DatabaseBackend::Scid(db));
                ResponseMessage {
                    id,
                    status: "ok".to_string(),
                    data: Some(serde_json::json!({
                        "stats": stats,
                        "pos_index_status": status_str,
                        "pos_index_unique_positions": pos_count,
                        "tree_index_status": tree_status_str,
                        "tree_index_unique_positions": tree_pos_count,
                        "format": stats.get("format").and_then(|v| v.as_str()).unwrap_or("si5"),
                        "total_games": total_games
                    })),
                    error: None,
                }
            }
            Err(e) => ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some(format!("Failed to open SCID database: {}", e)),
            },
        }
    }
}

pub fn handle_create_db(
    req: &RequestMessage,
    current_db: &mut Option<DatabaseBackend>,
) -> ResponseMessage {
    let id = req.id;
    let path_str = match req.params.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Missing 'path' parameter".to_string()),
            }
        }
    };
    let format_str = req
        .params
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("si5");
    let format = if format_str.eq_ignore_ascii_case("si4") {
        ScidFormat::Si4
    } else {
        ScidFormat::Si5
    };

    match ScidDatabaseWrapper::create(Path::new(path_str), format) {
        Ok(db) => {
            let stats = db.stats();
            *current_db = Some(DatabaseBackend::Scid(db));
            ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::json!({
                    "stats": stats,
                    "format": stats.format.to_string(),
                    "total_games": stats.total_games
                })),
                error: None,
            }
        }
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Failed to create database: {}", e)),
        },
    }
}

pub fn handle_info_stats(
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

    match db {
        DatabaseBackend::Scid(s) => {
            let stats = s.stats();
            ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::json!({
                    "stats": stats,
                    "format": stats.format.to_string(),
                    "total_games": stats.total_games,
                    "active_games": stats.active_games,
                    "deleted_games": stats.deleted_games,
                    "players_count": stats.players_count,
                    "events_count": stats.events_count,
                    "sites_count": stats.sites_count,
                    "rounds_count": stats.rounds_count,
                })),
                error: None,
            }
        }
        DatabaseBackend::Pgn(p) => {
            let total = p.game_count();
            ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::json!({
                    "stats": {
                        "format": "pgn",
                        "total_games": total,
                        "active_games": total,
                        "deleted_games": 0,
                        "players_count": 0,
                        "events_count": 0,
                        "sites_count": 0,
                        "rounds_count": 0,
                        "path": p.pgn_path.to_string_lossy().to_string()
                    },
                    "format": "pgn",
                    "total_games": total,
                    "active_games": total,
                    "deleted_games": 0,
                    "players_count": 0,
                    "events_count": 0,
                    "sites_count": 0,
                    "rounds_count": 0,
                })),
                error: None,
            }
        }
    }
}

pub fn handle_query_games(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    session_mgr: &crate::server::search_session::SearchSessionManager,
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

    let page = req
        .params
        .get("page")
        .or_else(|| req.params.get("params").and_then(|p| p.get("page")))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let page_size = req
        .params
        .get("page_size")
        .or_else(|| req.params.get("limit"))
        .or_else(|| {
            req.params
                .get("params")
                .and_then(|p| p.get("page_size").or_else(|| p.get("limit")))
        })
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;

    let search_id_opt = req
        .params
        .get("search_id")
        .or_else(|| req.params.get("params").and_then(|p| p.get("search_id")))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty());

    // ⚡ Search Session Pagination: Paginate through cached search results and resolve headers on-demand
    if let Some(search_id) = search_id_opt {
        let session = match session_mgr.get_session(search_id) {
            Some(s) => s,
            None => {
                return ResponseMessage {
                    id,
                    status: "error".to_string(),
                    data: None,
                    error: Some(format!(
                        "Search session '{}' not found or expired",
                        search_id
                    )),
                };
            }
        };

        let total = session.matches.len();
        let start = page * page_size;
        let games: Vec<GameSummary> = if start >= total {
            Vec::new()
        } else {
            let end = usize::min(start + page_size, total);
            let slice = &session.matches[start..end];
            match db {
                DatabaseBackend::Scid(s) => slice
                    .iter()
                    .filter_map(|m| {
                        let mut summ = s.get_game_summary(m.game_id)?;
                        summ.matching_plies = Some(m.match_details.matching_plies.clone());
                        summ.match_count = Some(m.match_details.match_count);
                        Some(summ)
                    })
                    .collect(),
                DatabaseBackend::Pgn(p) => slice
                    .iter()
                    .map(|m| {
                        let mut summ = p.get_summary(m.game_id);
                        summ.matching_plies = Some(m.match_details.matching_plies.clone());
                        summ.match_count = Some(m.match_details.match_count);
                        summ
                    })
                    .collect(),
            }
        };

        return ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(serde_json::json!({
                "page": page,
                "page_size": page_size,
                "total": total,
                "search_id": search_id,
                "games": games
            })),
            error: None,
        };
    }

    let filter_value = req.params.get("params").unwrap_or(&req.params);
    let filter: GameFilter = serde_json::from_value(filter_value.clone()).unwrap_or_default();
    let (games, total) = thread_pool.install(|| match db {
        DatabaseBackend::Scid(s) => s.query_games_with_progress(&filter, page, page_size, |scanned, total, matches_len| {
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
        }),
        DatabaseBackend::Pgn(p) => p.query_games_with_progress(&filter, page, page_size, |scanned, total, matches_len| {
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
        }),
    });

    ResponseMessage {
        id,
        status: "ok".to_string(),
        data: Some(serde_json::json!({
            "page": page,
            "page_size": page_size,
            "total": total,
            "games": games
        })),
        error: None,
    }
}

pub fn handle_get_game_summaries(
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
    let ids: Vec<usize> = req
        .params
        .get("game_ids")
        .or_else(|| req.params.get("ids"))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let summaries: Vec<crate::db::GameSummary> = match db {
        DatabaseBackend::Scid(s) => ids
            .iter()
            .filter_map(|&gid| s.get_game_summary(gid))
            .collect(),
        DatabaseBackend::Pgn(p) => ids
            .iter()
            .filter_map(|&gid| {
                if gid < p.entries.len() {
                    Some(p.get_summary(gid))
                } else {
                    None
                }
            })
            .collect(),
    };
    ResponseMessage {
        id,
        status: "ok".to_string(),
        data: Some(serde_json::json!({ "game_summaries": summaries })),
        error: None,
    }
}

pub fn handle_get_game_pgn(
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

    let index = match req
        .params
        .get("index")
        .or_else(|| req.params.get("id"))
        .and_then(|v| v.as_u64())
    {
        Some(i) => i as usize,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Missing 'index' parameter".to_string()),
            }
        }
    };

    let res = match db {
        DatabaseBackend::Scid(s) => s.game_pgn(index),
        DatabaseBackend::Pgn(p) => p.get_game_pgn(index),
    };

    match res {
        Ok(pgn) => ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(serde_json::json!({
                "index": index,
                "pgn": pgn
            })),
            error: None,
        },
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Error reading PGN for game {}: {}", index, e)),
        },
    }
}

pub fn handle_add_game(
    req: &RequestMessage,
    current_db: &mut Option<DatabaseBackend>,
) -> ResponseMessage {
    let id = req.id;
    let db = match current_db {
        Some(DatabaseBackend::Scid(s)) => s,
        Some(DatabaseBackend::Pgn(_)) => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Direct editing is not supported on raw .pgn files. Please import this PGN into a SCID (.si5) database.".to_string()),
            }
        }
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("No database currently opened".to_string()),
            }
        }
    };

    let pgn = match req.params.get("pgn").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Missing 'pgn' parameter".to_string()),
            }
        }
    };

    match db.add_game(pgn) {
        Ok(idx) => ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(serde_json::json!({
                "index": idx,
                "total": db.game_count()
            })),
            error: None,
        },
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Failed to add game: {}", e)),
        },
    }
}

pub fn handle_update_game(
    req: &RequestMessage,
    current_db: &mut Option<DatabaseBackend>,
) -> ResponseMessage {
    let id = req.id;
    let db = match current_db {
        Some(DatabaseBackend::Scid(s)) => s,
        Some(DatabaseBackend::Pgn(_)) => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Direct editing is not supported on raw .pgn files. Please import this PGN into a SCID (.si5) database.".to_string()),
            }
        }
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("No database currently opened".to_string()),
            }
        }
    };

    let index = match req
        .params
        .get("index")
        .or_else(|| req.params.get("id"))
        .and_then(|v| v.as_u64())
    {
        Some(i) => i as usize,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Missing 'index' parameter".to_string()),
            }
        }
    };

    let pgn = match req.params.get("pgn").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Missing 'pgn' parameter".to_string()),
            }
        }
    };

    match db.update_game(index, pgn) {
        Ok(()) => ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(serde_json::json!({ "index": index })),
            error: None,
        },
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Failed to update game {}: {}", index, e)),
        },
    }
}

pub fn handle_delete_game(
    req: &RequestMessage,
    current_db: &mut Option<DatabaseBackend>,
) -> ResponseMessage {
    let id = req.id;
    let db = match current_db {
        Some(DatabaseBackend::Scid(s)) => s,
        Some(DatabaseBackend::Pgn(_)) => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some(
                    "Deleting games is only supported on SCID (.si5) databases.".to_string(),
                ),
            }
        }
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("No database currently opened".to_string()),
            }
        }
    };

    let index = match req
        .params
        .get("index")
        .or_else(|| req.params.get("id"))
        .and_then(|v| v.as_u64())
    {
        Some(i) => i as usize,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Missing 'index' parameter".to_string()),
            }
        }
    };

    match db.delete_game(index) {
        Ok(()) => ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(serde_json::json!({ "index": index, "deleted": true })),
            error: None,
        },
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Failed to delete game {}: {}", index, e)),
        },
    }
}

pub fn handle_undelete_game(
    req: &RequestMessage,
    current_db: &mut Option<DatabaseBackend>,
) -> ResponseMessage {
    let id = req.id;
    let db = match current_db {
        Some(DatabaseBackend::Scid(s)) => s,
        Some(DatabaseBackend::Pgn(_)) => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some(
                    "Undeleting games is only supported on SCID (.si5) databases.".to_string(),
                ),
            }
        }
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("No database currently opened".to_string()),
            }
        }
    };

    let index = match req
        .params
        .get("index")
        .or_else(|| req.params.get("id"))
        .and_then(|v| v.as_u64())
    {
        Some(i) => i as usize,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Missing 'index' parameter".to_string()),
            }
        }
    };

    match db.undelete_game(index) {
        Ok(()) => ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(serde_json::json!({ "index": index, "deleted": false })),
            error: None,
        },
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Failed to undelete game {}: {}", index, e)),
        },
    }
}

pub fn handle_compact(
    req: &RequestMessage,
    current_db: &mut Option<DatabaseBackend>,
) -> ResponseMessage {
    let id = req.id;
    let db = match current_db {
        Some(DatabaseBackend::Scid(s)) => s,
        Some(DatabaseBackend::Pgn(_)) => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Compacting is only supported on SCID (.si5) databases.".to_string()),
            }
        }
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("No database currently opened".to_string()),
            }
        }
    };

    match db.compact() {
        Ok(reclaimed) => ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(serde_json::json!({ "reclaimed_bytes": reclaimed })),
            error: None,
        },
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Compaction failed: {}", e)),
        },
    }
}

pub fn handle_save(
    req: &RequestMessage,
    current_db: &mut Option<DatabaseBackend>,
) -> ResponseMessage {
    let id = req.id;
    let db = match current_db {
        Some(DatabaseBackend::Scid(s)) => s,
        Some(DatabaseBackend::Pgn(_)) => {
            return ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::json!({ "message": "PGN file is saved on disk." })),
                error: None,
            }
        }
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("No database currently opened".to_string()),
            }
        }
    };

    match db.save() {
        Ok(()) => {
            let stats = db.stats();
            ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::json!({ "stats": stats })),
                error: None,
            }
        }
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Save failed: {}", e)),
        },
    }
}

pub fn handle_sort_database(
    req: &RequestMessage,
    current_db: &mut Option<DatabaseBackend>,
    current_pos_index: &mut Option<PositionIndex>,
) -> ResponseMessage {
    let id = req.id;
    let s = match current_db {
        Some(DatabaseBackend::Scid(ref mut s)) => s,
        Some(DatabaseBackend::Pgn(_)) => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some(
                    "Cannot sort PGN database in-place with sort_database; use sort_pgn instead"
                        .to_string(),
                ),
            };
        }
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("No database currently opened".to_string()),
            };
        }
    };

    let sort_by = req
        .params
        .get("sort_by")
        .and_then(|v| v.as_str())
        .unwrap_or("date");
    let sort_asc = req
        .params
        .get("sort_asc")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let delete_removed = req
        .params
        .get("delete_removed")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let output_path = req.params.get("output_path").and_then(|v| v.as_str());

    let res = if let Some(out_p) = output_path {
        s.sort_database_to(Path::new(out_p), sort_by, sort_asc, delete_removed)
    } else {
        s.sort_database(sort_by, sort_asc, delete_removed)
    };

    match res {
        Ok(count) => {
            *current_pos_index = None;
            ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(
                    serde_json::json!({ "sorted_games": count, "sort_by": sort_by, "sort_asc": sort_asc }),
                ),
                error: None,
            }
        }
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Database sort failed: {}", e)),
        },
    }
}

pub fn handle_sort_pgn(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
) -> ResponseMessage {
    let id = req.id;
    let input_path = req.params.get("input_path").and_then(|v| v.as_str());
    let output_path = req.params.get("output_path").and_then(|v| v.as_str());
    let sort_by = req
        .params
        .get("sort_by")
        .and_then(|v| v.as_str())
        .unwrap_or("date");
    let sort_asc = req
        .params
        .get("sort_asc")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let in_p_buf = match input_path {
        Some(p) => PathBuf::from(p),
        None => {
            if let Some(DatabaseBackend::Pgn(ref p)) = current_db {
                p.pgn_path.clone()
            } else {
                return ResponseMessage {
                    id,
                    status: "error".to_string(),
                    data: None,
                    error: Some(
                        "Missing 'input_path' parameter and no active PGN database".to_string(),
                    ),
                };
            }
        }
    };

    let out_p = match output_path {
        Some(p) => Path::new(p),
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Missing 'output_path' parameter".to_string()),
            };
        }
    };

    match crate::pgn_db::sort_pgn_file(&in_p_buf, out_p, Some(sort_by), sort_asc) {
        Ok(count) => ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(
                serde_json::json!({ "sorted_games": count, "sort_by": sort_by, "sort_asc": sort_asc }),
            ),
            error: None,
        },
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("PGN sort failed: {}", e)),
        },
    }
}
