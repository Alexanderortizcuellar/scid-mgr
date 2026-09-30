use crate::pgn_utils::import_pgn_file_with_progress;
use crate::server::{DatabaseBackend, RequestMessage, ResponseMessage};
use std::io::{self, Write};
use std::path::Path;

pub fn handle_import_pgn(
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

    let import_result = import_pgn_file_with_progress(db, pgn_path, |prog| {
        let event_json = serde_json::json!({
            "event": "import_progress",
            "data": prog
        });
        if let Ok(line) = serde_json::to_string(&event_json) {
            let mut out = io::stdout().lock();
            let _ = writeln!(out, "{}", line);
            let _ = out.flush();
        }
    });

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

pub fn handle_export_pgn(
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
        DatabaseBackend::Pgn(p) => std::fs::copy(&p.pgn_path, out_path)
            .map(|_| p.game_count())
            .map_err(|e| anyhow::anyhow!("Failed to export PGN: {}", e)),
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

pub fn handle_benchmark(
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

    let path = match db {
        DatabaseBackend::Scid(s) => s.index_path().to_path_buf(),
        DatabaseBackend::Pgn(p) => p.pgn_path.clone(),
    };

    let heavy = req
        .params
        .get("heavy")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

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
