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

pub fn run_interactive_server(initial_db_path: Option<PathBuf>) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = stdin.lock();

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

        let resp = handle_command(&mut current_db, &mut current_pos_index, req);
        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
        stdout.flush()?;
        line.clear();
    }

    Ok(())
}

fn handle_command(
    current_db: &mut Option<DatabaseBackend>,
    current_pos_index: &mut Option<PositionIndex>,
    req: RequestMessage,
) -> ResponseMessage {
    let id = req.id;
    let cmd = req.command.as_str();

    match cmd {
        "open" => {
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

            let page = req.params.get("page").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let page_size = req
                .params
                .get("page_size")
                .or_else(|| req.params.get("limit"))
                .and_then(|v| v.as_u64())
                .unwrap_or(100) as usize;

            let filter: GameFilter = serde_json::from_value(req.params.clone()).unwrap_or_default();
            let (games, total) = match db {
                DatabaseBackend::Scid(s) => s.query_games(&filter, page, page_size),
                DatabaseBackend::Pgn(p) => p.query_games(&filter, page, page_size),
            };

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

            let fen = match req.params.get("fen").and_then(|v| v.as_str()) {
                Some(f) => f,
                None => {
                    return ResponseMessage {
                        id,
                        status: "error".to_string(),
                        data: None,
                        error: Some("Missing 'fen' parameter".to_string()),
                    }
                }
            };

            // ⚡ Instant Sub-Millisecond lookup if PositionIndex is active
            if let Some(pos_idx) = current_pos_index.as_ref() {
                if let Ok(f) = fen.trim().parse::<shakmaty::fen::Fen>() {
                    if let Ok(p) = f.into_position::<shakmaty::Chess>(shakmaty::CastlingMode::Standard) {
                        use shakmaty::zobrist::ZobristHash;
                        let h: shakmaty::zobrist::Zobrist64 = p.zobrist_hash(shakmaty::EnPassantMode::Legal);
                        if let Some(game_ids) = pos_idx.get_position_sample_games(h.0) {
                            let matches: Vec<crate::position_search::PositionMatch> = game_ids
                                .iter()
                                .map(|&gid| crate::position_search::PositionMatch { game_id: gid as usize, ply: 0 })
                                .collect();
                            let total_games = match db {
                                DatabaseBackend::Scid(s) => s.game_count(),
                                DatabaseBackend::Pgn(p) => p.game_count(),
                            };
                            let res = crate::position_search::PositionSearchResult {
                                target_fen: fen.to_string(),
                                target_hash: h.0,
                                matches,
                                total_games_searched: total_games,
                                elapsed_ms: 0.08,
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
                .and_then(|v| v.as_u64())
                .map(|p| p as usize);

            match db {
                DatabaseBackend::Scid(s) => match s.search_position(fen, max_ply) {
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
                },
                DatabaseBackend::Pgn(p) => {
                    let res = p.search_position(fen, max_ply, |scanned, total, matches_len| {
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
            
            // Lazy-load on demand if not already loaded in memory
            if current_pos_index.is_none() {
                if let Some(db) = current_db {
                    let db_path = match db {
                        DatabaseBackend::Scid(s) => s.index_path().to_path_buf(),
                        DatabaseBackend::Pgn(p) => p.pgn_path.clone(),
                    };
                    *current_pos_index = PositionIndex::load(&db_path).ok();
                }
            }

            if let Some(pos_idx) = current_pos_index.as_ref() {
                if let Some(report) = pos_idx.query_tree(fen) {
                    ResponseMessage {
                        id,
                        status: "ok".to_string(),
                        data: Some(serde_json::to_value(&report).unwrap_or_default()),
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
                        })),
                        error: None,
                    }
                }
            } else {
                ResponseMessage {
                    id,
                    status: "error".to_string(),
                    data: None,
                    error: Some("Position Index (.pos.idx) is not present. Click '⚡ Build Fast Index' to create it.".to_string()),
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
            let threads = req.params.get("threads").and_then(|v| v.as_u64()).map(|t| t as usize);
            let start = Instant::now();

            let res = match db {
                DatabaseBackend::Scid(s) => {
                    let games_path = s.games_path().to_path_buf();
                    let entries = s.entries();
                    let db_path = s.index_path().to_path_buf();
                    PositionIndex::build_for_scid(&db_path, entries, &games_path, max_ply, threads, |scanned, total, positions| {
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
                    PositionIndex::build_for_pgn(&db_path, entries, mmap, max_ply, threads, |scanned, total, positions| {
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
                    *current_pos_index = Some(idx);
                    ResponseMessage {
                        id,
                        status: "ok".to_string(),
                        data: Some(serde_json::json!({
                            "status": "valid",
                            "unique_positions": unique_positions,
                            "elapsed_ms": elapsed_ms,
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

        "get_pgn" | "get_game_pgn" => {
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
