use crate::db::{GameFilter, ScidDatabaseWrapper, ScidFormat};
use crate::pgn_utils::import_pgn_file_with_progress;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

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

    let mut current_db: Option<ScidDatabaseWrapper> = None;

    if let Some(path) = initial_db_path {
        if path.exists() {
            match ScidDatabaseWrapper::open(&path) {
                Ok(db) => {
                    eprintln!("[Server] Auto-opened database: {}", path.display());
                    current_db = Some(db);
                }
                Err(e) => {
                    eprintln!("[Server] Failed to auto-open {}: {}", path.display(), e);
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

        let resp = handle_command(&mut current_db, req);
        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
        stdout.flush()?;
        line.clear();
    }

    Ok(())
}

fn handle_command(
    current_db: &mut Option<ScidDatabaseWrapper>,
    req: RequestMessage,
) -> ResponseMessage {
    let id = req.id;
    let cmd = req.command.as_str();

    match cmd {
        "open" => {
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

            match ScidDatabaseWrapper::open(Path::new(path_str)) {
                Ok(db) => {
                    let stats = db.stats();
                    *current_db = Some(db);
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
                    error: Some(format!("Failed to open database: {}", e)),
                },
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
                    *current_db = Some(db);
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

            let stats = db.stats();
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
            let (games, total) = db.query_games(&filter, page, page_size);

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

            let max_ply = req
                .params
                .get("max_ply")
                .and_then(|v| v.as_u64())
                .map(|p| p as usize);

            match db.search_position(fen, max_ply) {
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
            match db.search_material(&filter) {
                Ok(matches) => {
                    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                    ResponseMessage {
                        id,
                        status: "ok".to_string(),
                        data: Some(serde_json::json!({
                            "matches": matches,
                            "match_count": matches.len(),
                            "total_games": db.game_count(),
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

            match db.game_pgn(index) {
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
            let export_result = crate::pgn_utils::export_pgn_ultra_fast(db, out_path, |prog| {
                let event_json = serde_json::json!({
                    "event": "export_progress",
                    "data": prog
                });
                if let Ok(line) = serde_json::to_string(&event_json) {
                    let mut out = io::stdout().lock();
                    let _ = writeln!(out, "{}", line);
                    let _ = out.flush();
                }
            });

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

        unknown => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Unknown command: '{}'", unknown)),
        },
    }
}
