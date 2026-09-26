use crate::pgn_db::PgnDatabaseWrapper;
use crate::server::search_session::SearchSessionManager;
use crate::server::{DatabaseBackend, RequestMessage, ResponseMessage};
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

pub fn handle_validate_dsl(req: &RequestMessage) -> ResponseMessage {
    let id = req.id;
    let query_str = req
        .params
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    match crate::search::parser::QueryParser::parse_str(query_str) {
        Ok(_) => ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(serde_json::json!({ "valid": true })),
            error: None,
        },
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: Some(serde_json::json!({
                "valid": false,
                "position": e.position,
                "line": e.line,
                "column": e.column,
                "snippet": e.snippet,
                "help": e.help,
            })),
            error: Some(format!("{}", e)),
        },
    }
}

pub fn handle_explain_dsl(req: &RequestMessage) -> ResponseMessage {
    let id = req.id;
    let query_str = req
        .params
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    match crate::search::parser::QueryParser::parse_str(query_str) {
        Ok(parsed_query) => {
            let explanation = crate::search::explain::explain_query(query_str, &parsed_query);
            ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::to_value(&explanation).unwrap_or(serde_json::json!({}))),
                error: None,
            }
        }
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: Some(serde_json::json!({
                "valid": false,
                "position": e.position,
                "line": e.line,
                "column": e.column,
                "snippet": e.snippet,
                "help": e.help,
            })),
            error: Some(format!("{}", e)),
        },
    }
}

pub fn handle_cql_search(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    session_mgr: &mut SearchSessionManager,
    thread_pool: &rayon::ThreadPool,
) -> ResponseMessage {
    let id = req.id;
    let query_str = match req
        .params
        .get("query")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
    {
        Some(q) => q,
        None => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some("Missing 'query' parameter".to_string()),
            };
        }
    };

    let parsed_query = match crate::search::parser::QueryParser::parse_str(query_str) {
        Ok(q) => q,
        Err(e) => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: Some(serde_json::json!({
                    "position": e.position,
                    "line": e.line,
                    "column": e.column,
                    "snippet": e.snippet,
                    "help": e.help,
                })),
                error: Some(format!("{}", e)),
            };
        }
    };

    let custom_pgn_path = req
        .params
        .get("pgn_path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty());

    let start_time = Instant::now();

    if let Some(pgn_path) = custom_pgn_path {
        let path = Path::new(pgn_path);
        if !path.exists() {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some(format!("PGN file does not exist: {}", pgn_path)),
            };
        }
        match PgnDatabaseWrapper::open(path) {
            Ok(pgn_wrapper) => {
                let total_games = pgn_wrapper.game_count();
                let db_key = SearchSessionManager::db_key(&pgn_wrapper.pgn_path, total_games);

                // ⚡ Fast Cache Lookup: reuse identical query on unchanged database
                if let Some(cached) = session_mgr.find_cached(&db_key, query_str) {
                    let matched_count = cached.matches.len();
                    let total_searched = cached.total_searched;
                    let search_id = cached.search_id.clone();
                    return ResponseMessage {
                        id,
                        status: "ok".to_string(),
                        data: Some(serde_json::json!({
                            "search_id": search_id,
                            "total_searched": total_searched,
                            "matched_count": matched_count,
                            "duration_ms": 0,
                            "cached": true,
                        })),
                        error: None,
                    };
                }

                let match_results = thread_pool.install(|| {
                    pgn_wrapper.search_query_with_progress(&parsed_query, |scanned, total, matches_len| {
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

                let duration_ms = start_time.elapsed().as_millis() as u64;
                let matched_count = match_results.len();
                let search_id = session_mgr.create_session(
                    &db_key,
                    query_str,
                    total_games,
                    match_results,
                    duration_ms,
                );

                ResponseMessage {
                    id,
                    status: "ok".to_string(),
                    data: Some(serde_json::json!({
                        "search_id": search_id,
                        "total_searched": total_games,
                        "matched_count": matched_count,
                        "duration_ms": duration_ms,
                        "cached": false,
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
        let db = match current_db.as_ref() {
            Some(d) => d,
            None => {
                return ResponseMessage {
                    id,
                    status: "error".to_string(),
                    data: None,
                    error: Some(
                        "No database currently opened and no 'pgn_path' provided".to_string(),
                    ),
                };
            }
        };

        match db {
            DatabaseBackend::Scid(s) => {
                let total_games = s.game_count();
                let db_key = SearchSessionManager::db_key(&s.index_path, total_games);

                // ⚡ Fast Cache Lookup: reuse identical query on unchanged database
                if let Some(cached) = session_mgr.find_cached(&db_key, query_str) {
                    let matched_count = cached.matches.len();
                    let total_searched = cached.total_searched;
                    let search_id = cached.search_id.clone();
                    return ResponseMessage {
                        id,
                        status: "ok".to_string(),
                        data: Some(serde_json::json!({
                            "search_id": search_id,
                            "total_searched": total_searched,
                            "matched_count": matched_count,
                            "duration_ms": 0,
                            "cached": true,
                        })),
                        error: None,
                    };
                }

                let match_results = thread_pool.install(|| {
                    s.search_query_with_progress(&parsed_query, |scanned, total, matches_len| {
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

                let duration_ms = start_time.elapsed().as_millis() as u64;
                let matched_count = match_results.len();
                let search_id = session_mgr.create_session(
                    &db_key,
                    query_str,
                    total_games,
                    match_results,
                    duration_ms,
                );

                ResponseMessage {
                    id,
                    status: "ok".to_string(),
                    data: Some(serde_json::json!({
                        "search_id": search_id,
                        "total_searched": total_games,
                        "matched_count": matched_count,
                        "duration_ms": duration_ms,
                        "cached": false,
                    })),
                    error: None,
                }
            }
            DatabaseBackend::Pgn(p) => {
                let total_games = p.game_count();
                let db_key = SearchSessionManager::db_key(&p.pgn_path, total_games);

                // ⚡ Fast Cache Lookup: reuse identical query on unchanged database
                if let Some(cached) = session_mgr.find_cached(&db_key, query_str) {
                    let matched_count = cached.matches.len();
                    let total_searched = cached.total_searched;
                    let search_id = cached.search_id.clone();
                    return ResponseMessage {
                        id,
                        status: "ok".to_string(),
                        data: Some(serde_json::json!({
                            "search_id": search_id,
                            "total_searched": total_searched,
                            "matched_count": matched_count,
                            "duration_ms": 0,
                            "cached": true,
                        })),
                        error: None,
                    };
                }

                let match_results = thread_pool.install(|| {
                    p.search_query_with_progress(&parsed_query, |scanned, total, matches_len| {
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

                let duration_ms = start_time.elapsed().as_millis() as u64;
                let matched_count = match_results.len();
                let search_id = session_mgr.create_session(
                    &db_key,
                    query_str,
                    total_games,
                    match_results,
                    duration_ms,
                );

                ResponseMessage {
                    id,
                    status: "ok".to_string(),
                    data: Some(serde_json::json!({
                        "search_id": search_id,
                        "total_searched": total_games,
                        "matched_count": matched_count,
                        "duration_ms": duration_ms,
                        "cached": false,
                    })),
                    error: None,
                }
            }
        }
    }
}
