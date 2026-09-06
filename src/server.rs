use crate::db::{GameFilter, ScidDatabaseWrapper, ScidFormat};
use crate::pgn_db::PgnDatabaseWrapper;
use crate::pgn_utils::import_pgn_file_with_progress;
use crate::position_index::{IndexStatus, PositionIndex};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[allow(clippy::large_enum_variant)]
pub enum DatabaseBackend {
    Scid(ScidDatabaseWrapper),
    Pgn(PgnDatabaseWrapper),
}

#[derive(Debug, Deserialize)]
pub struct RequestMessage {
    pub id: Option<u64>,
    pub command: String,
    #[serde(flatten)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct ResponseMessage {
    pub id: Option<u64>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn run_interactive_server(
    initial_db_path: Option<PathBuf>,
    initial_threads: Option<usize>,
) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = stdin.lock();

    let max_system_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let mut current_thread_count = initial_threads.unwrap_or(max_system_threads).max(1);
    let mut thread_pool = rayon::ThreadPoolBuilder::new().num_threads(current_thread_count).build()?;

    let mut current_db: Option<DatabaseBackend> = None;
    let mut current_pos_index: Option<PositionIndex> = None;

    if let Some(path) = initial_db_path {
        if path.exists() {
            let path_str = path.to_string_lossy().to_lowercase();
            if path_str.ends_with(".pgn") {
                match PgnDatabaseWrapper::open(&path) {
                    Ok(pgn) => {
                        eprintln!("[Server] Auto-opened PGN database: {}", path.display());
                        current_db = Some(DatabaseBackend::Pgn(pgn));
                    }
                    Err(e) => {
                        eprintln!("[Server] Failed to auto-open {}: {}", path.display(), e);
                    }
                }
            } else {
                match ScidDatabaseWrapper::open(&path) {
                    Ok(db) => {
                        eprintln!("[Server] Auto-opened SCID database: {}", path.display());
                        current_db = Some(DatabaseBackend::Scid(db));
                    }
                    Err(e) => {
                        eprintln!("[Server] Failed to auto-open {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        let req: RequestMessage = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let resp = ResponseMessage {
                    id: None,
                    status: "error".to_string(),
                    data: None,
                    error: Some(format!("Invalid JSON request: {}", e)),
                };
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
                line.clear();
                continue;
            }
        };

        if req.command == "shutdown" || req.command == "exit" || req.command == "quit" {
            let resp = ResponseMessage {
                id: req.id,
                status: "ok".to_string(),
                data: Some(serde_json::json!({"message": "Shutting down"})),
                error: None,
            };
            writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            stdout.flush()?;
            break;
        }

        let resp = handle_command(
            &mut current_db,
            &mut current_pos_index,
            &mut thread_pool,
            &mut current_thread_count,
            max_system_threads,
            req,
        );
        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
        stdout.flush()?;
        line.clear();
    }

    Ok(())
}

fn handle_command(
    current_db: &mut Option<DatabaseBackend>,
    current_pos_index: &mut Option<PositionIndex>,
    thread_pool: &mut rayon::ThreadPool,
    current_thread_count: &mut usize,
    max_system_threads: usize,
    req: RequestMessage,
) -> ResponseMessage {
    let id = req.id;
    let cmd = req.command.as_str();

    match cmd {
        "set_threads" | "set_config" => {
            let threads = req
                .params
                .get("threads")
                .or_else(|| req.params.get("params").and_then(|p| p.get("threads")))
                .and_then(|v| v.as_u64())
                .unwrap_or(max_system_threads as u64) as usize;
            let clamped = threads.clamp(1, max_system_threads * 2);
            match rayon::ThreadPoolBuilder::new().num_threads(clamped).build() {
                Ok(new_pool) => {
                    *thread_pool = new_pool;
                    *current_thread_count = clamped;
                    ResponseMessage {
                        id,
                        status: "ok".to_string(),
                        data: Some(serde_json::json!({
                            "threads": *current_thread_count,
                            "max_threads": max_system_threads,
                        })),
                        error: None,
                    }
                }
                Err(e) => ResponseMessage {
                    id,
                    status: "error".to_string(),
                    data: None,
                    error: Some(format!("Failed to configure thread pool: {}", e)),
                },
            }
        }

        "get_threads" | "get_config" => {
            ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::json!({
                    "threads": *current_thread_count,
                    "max_threads": max_system_threads,
                })),
                error: None,
            }
        }
        "open" | "open_db" => {
            let path_str = match req.params.get("path").or_else(|| req.params.get("params").and_then(|p| p.get("path"))).and_then(|v| v.as_str()) {
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
                        *current_pos_index = None; // Keep index unloaded in RAM until explicitly requested

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
                                },
                                "pos_index_status": status_str,
                                "pos_index_unique_positions": pos_count,
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
                        *current_pos_index = None; // Keep index unloaded in RAM until explicitly requested

                        if let Some(obj) = stats.as_object_mut() {
                            obj.insert("pos_index_status".to_string(), serde_json::json!(status_str));
                            obj.insert("pos_index_unique_positions".to_string(), serde_json::json!(pos_count));
                        }

                        *current_db = Some(DatabaseBackend::Scid(db));
                        ResponseMessage {
                            id,
                            status: "ok".to_string(),
                            data: Some(serde_json::json!({
                                "stats": stats,
                                "pos_index_status": status_str,
                                "pos_index_unique_positions": pos_count,
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

        "create" => {
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

        "info" | "stats" => {
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

        "query_games" | "get_games" => {
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
                .or_else(|| req.params.get("params").and_then(|p| p.get("page_size").or_else(|| p.get("limit"))))
                .and_then(|v| v.as_u64())
                .unwrap_or(100) as usize;

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

        "get_game_summaries" => {
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
                DatabaseBackend::Scid(s) => ids.iter().filter_map(|&gid| s.get_game_summary(gid)).collect(),
                DatabaseBackend::Pgn(p) => ids.iter().filter_map(|&gid| {
                    if gid < p.entries.len() {
                        Some(p.get_summary(gid))
                    } else {
                        None
                    }
                }).collect(),
            };
            ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::json!({ "game_summaries": summaries })),
                error: None,
            }
        }


        "search_position" | "position_search" => {
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
                .or_else(|| req.params.get("params").and_then(|p| p.get("match_mode").or_else(|| p.get("mode"))))
                .and_then(|v| v.as_str());

            let is_exact = mode_param.map(|m| {
                let m = m.to_lowercase();
                m == "exact" || m == "auto" || m.is_empty()
            }).unwrap_or(true);

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
                                .map(|gid| crate::position_search::PositionMatch { game_id: gid as usize, ply: 0 })
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

        "opening_tree" | "query_tree" => {
            let fen = req.params.get("fen").and_then(|v| v.as_str()).unwrap_or("");

            let explicit_game_ids: Option<Vec<usize>> = req.params.get("game_ids")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let use_search_results = req.params.get("use_search_results")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let filter_opt: Option<GameFilter> = req.params.get("filter")
                .or_else(|| req.params.get("params"))
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let include_all_game_ids = req.params.get("include_all_game_ids")
                .or_else(|| req.params.get("all_game_ids"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let max_sample_ids: Option<usize> = if include_all_game_ids {
                None
            } else {
                req.params.get("max_sample_games")
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
                            DatabaseBackend::Scid(s) => { let _ = s.query_games(f, 0, 0); },
                            DatabaseBackend::Pgn(p) => { let _ = p.query_games(f, 0, 0); },
                        };
                        target_game_ids = match db {
                            DatabaseBackend::Scid(s) => s.get_cached_query_indices(),
                            DatabaseBackend::Pgn(p) => p.get_cached_query_indices(),
                        };
                    }
                }
            }

            let mut report = None;

            // 1. Try fast lookup from indexed .pos.idx file (both unfiltered and filtered using inverted index!)
            if current_pos_index.is_none() {
                let db_path = match db {
                    DatabaseBackend::Scid(s) => s.index_path().to_path_buf(),
                    DatabaseBackend::Pgn(p) => p.pgn_path.clone(),
                };
                *current_pos_index = PositionIndex::load(&db_path).ok();
            }

            if let Some(pos_idx) = current_pos_index.as_ref() {
                report = pos_idx.query_tree_with_options(fen, target_game_ids.as_deref(), max_sample_ids);
            }

            // 2. Dynamic Fallback: If .pos.idx is missing or position is beyond max depth
            if report.is_none() {
                report = match db {
                    DatabaseBackend::Scid(s) => {
                        PositionIndex::calculate_tree_for_scid(
                            s.entries(),
                            s.games_path(),
                            fen,
                            target_game_ids.as_deref(),
                            Some(500),
                        )
                    }
                    DatabaseBackend::Pgn(p) => {
                        PositionIndex::calculate_tree_for_pgn(
                            &p.entries,
                            p.mmap_ref(),
                            fen,
                            target_game_ids.as_deref(),
                            Some(500),
                        )
                    }
                };
            }

            let include_last_played = req.params.get("include_last_played").and_then(|v| v.as_bool()).unwrap_or(true);
            let include_sample_games = req.params.get("include_sample_games").and_then(|v| v.as_bool()).unwrap_or(true);

            if let Some(mut rep) = report {
                if include_last_played {
                    for m in &mut rep.moves {
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
                                        if !d.is_empty() && !d.starts_with('?') {
                                            if max_date_str.as_ref().map_or(true, |cur| d > *cur) {
                                                max_date_str = Some(d);
                                            }
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
                        DatabaseBackend::Scid(s) => rep.sample_game_ids.iter().take(15).filter_map(|&gid| s.get_game_summary(gid as usize)).collect(),
                        DatabaseBackend::Pgn(p) => rep.sample_game_ids.iter().take(15).filter_map(|&gid| {
                            if (gid as usize) < p.entries.len() {
                                Some(p.get_summary(gid as usize))
                            } else {
                                None
                            }
                        }).collect(),
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

        "unload_pos_index" => {
            *current_pos_index = None;
            ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::json!({ "unloaded": true })),
                error: None,
            }
        }

        "pos_index_status" | "get_pos_index_status" => {
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

        "pos_index_diagnostics" | "get_pos_index_diagnostics" => {
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

        "build_pos_index" | "rebuild_pos_index" => {
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

            let max_ply = req.params.get("max_ply").and_then(|v| v.as_u64()).unwrap_or(24) as usize;
            let max_games = req.params.get("max_games").or_else(|| req.params.get("max_game_ids")).and_then(|v| v.as_u64()).map(|g| g as usize);
            let min_games = req.params.get("min_games").and_then(|v| v.as_u64()).map(|g| g as usize);
            let threads = req.params.get("threads").and_then(|v| v.as_u64()).map(|t| t as usize).or(Some(*current_thread_count));
            let start = Instant::now();

            let res = match db {
                DatabaseBackend::Scid(s) => {
                    let games_path = s.games_path().to_path_buf();
                    let entries = s.entries();
                    let db_path = s.index_path().to_path_buf();
                    PositionIndex::build_for_scid(&db_path, entries, &games_path, max_ply, max_games, min_games, threads, |scanned, total, positions| {
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
                    })
                }
                DatabaseBackend::Pgn(p) => {
                    let db_path = p.pgn_path.clone();
                    let entries = &p.entries;
                    let mmap = p.mmap_ref();
                    PositionIndex::build_for_pgn(&db_path, entries, mmap, max_ply, max_games, min_games, threads, |scanned, total, positions| {
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
                    })
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

        "search_material" | "material_search" => {
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

        "get_pgn" | "get_game" | "get_game_pgn" => {
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

        "add_game" => {
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

        "update_game" => {
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

        "delete_game" => {
            let db = match current_db {
                Some(DatabaseBackend::Scid(s)) => s,
                Some(DatabaseBackend::Pgn(_)) => {
                    return ResponseMessage {
                        id,
                        status: "error".to_string(),
                        data: None,
                        error: Some("Deleting games is only supported on SCID (.si5) databases.".to_string()),
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

        "undelete_game" => {
            let db = match current_db {
                Some(DatabaseBackend::Scid(s)) => s,
                Some(DatabaseBackend::Pgn(_)) => {
                    return ResponseMessage {
                        id,
                        status: "error".to_string(),
                        data: None,
                        error: Some("Undeleting games is only supported on SCID (.si5) databases.".to_string()),
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

        "compact" => {
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

        "save" => {
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

        "import_pgn" => {
            let db = match current_db {
                Some(DatabaseBackend::Scid(s)) => s,
                Some(DatabaseBackend::Pgn(_)) => {
                    return ResponseMessage {
                        id,
                        status: "error".to_string(),
                        data: None,
                        error: Some("Importing into an existing raw PGN is not supported. Please create or open a SCID (.si5) database.".to_string()),
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

            let pgn_path_str = match req.params.get("pgn_path").and_then(|v| v.as_str()) {
                Some(p) => p,
                None => {
                    return ResponseMessage {
                        id,
                        status: "error".to_string(),
                        data: None,
                        error: Some("Missing 'pgn_path' parameter".to_string()),
                    };
                }
            };
            let pgn_path = Path::new(pgn_path_str);
            let scid_exe_opt = req.params.get("scid_exe").and_then(|v| v.as_str());

            let import_result = match scid_exe_opt {
                Some(scid_exe) if Path::new(scid_exe).exists() => {
                    crate::pgn_utils::import_pgn_with_scid_cli(db, pgn_path, Path::new(scid_exe))
                }
                _ => import_pgn_file_with_progress(db, pgn_path, |prog| {
                    let event_json = serde_json::json!({
                        "event": "import_progress",
                        "data": prog
                    });
                    if let Ok(line) = serde_json::to_string(&event_json) {
                        let mut out = io::stdout().lock();
                        let _ = writeln!(out, "{}", line);
                        let _ = out.flush();
                    }
                }),
            };

            match import_result {
                Ok((imported, errors)) => {
                    let stats = db.stats();
                    ResponseMessage {
                        id,
                        status: "ok".to_string(),
                        data: Some(serde_json::json!({
                            "imported": imported,
                            "errors": errors,
                            "stats": stats
                        })),
                        error: None,
                    }
                }
                Err(e) => ResponseMessage {
                    id,
                    status: "error".to_string(),
                    data: None,
                    error: Some(format!("PGN import failed: {}", e)),
                },
            }
        }

        "export_pgn" => {
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

            let out_path_str = match req.params.get("output_path").and_then(|v| v.as_str()) {
                Some(p) => p,
                None => {
                    return ResponseMessage {
                        id,
                        status: "error".to_string(),
                        data: None,
                        error: Some("Missing 'output_path' parameter".to_string()),
                    }
                }
            };

            let out_path = Path::new(out_path_str);
            let export_result = match db {
                DatabaseBackend::Scid(s) => crate::pgn_utils::export_pgn_ultra_fast(s, out_path, |prog| {
                    let event_json = serde_json::json!({
                        "event": "export_progress",
                        "data": prog
                    });
                    if let Ok(line) = serde_json::to_string(&event_json) {
                        let mut out = io::stdout().lock();
                        let _ = writeln!(out, "{}", line);
                        let _ = out.flush();
                    }
                }),
                DatabaseBackend::Pgn(p) => {
                    std::fs::copy(&p.pgn_path, out_path)
                        .map(|_| p.game_count())
                        .map_err(|e| anyhow::anyhow!("Failed to export PGN: {}", e))
                }
            };

            match export_result {
                Ok(exported) => ResponseMessage {
                    id,
                    status: "ok".to_string(),
                    data: Some(serde_json::json!({ "exported": exported })),
                    error: None,
                },
                Err(e) => ResponseMessage {
                    id,
                    status: "error".to_string(),
                    data: None,
                    error: Some(format!("PGN export failed: {}", e)),
                },
            }
        }

        "benchmark" | "bench" => {
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

            let path = match db {
                DatabaseBackend::Scid(s) => s.index_path().to_path_buf(),
                DatabaseBackend::Pgn(p) => p.pgn_path.clone(),
            };

            let heavy = req.params.get("heavy").and_then(|v| v.as_bool()).unwrap_or(false);

            match crate::benchmark::run_benchmark(&path, heavy) {
                Ok(report) => ResponseMessage {
                    id,
                    status: "ok".to_string(),
                    data: Some(serde_json::to_value(&report).unwrap_or_default()),
                    error: None,
                },
                Err(e) => ResponseMessage {
                    id,
                    status: "error".to_string(),
                    data: None,
                    error: Some(format!("Benchmark failed: {}", e)),
                },
            }
        }

        unknown => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Unknown command: '{}'", unknown)),
        },
    }
}
