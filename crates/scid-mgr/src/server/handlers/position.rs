use crate::position_index::PositionIndex;
use crate::search::evaluator::QueryMatchResult;
use crate::search::ScidMatchResult;
use crate::server::search_session::SearchSessionManager;
use crate::server::{DatabaseBackend, RequestMessage, ResponseMessage};
use std::io::{self, Write};
use std::time::Instant;

pub fn handle_search_position(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    current_pos_index: &mut Option<PositionIndex>,
    session_mgr: &mut SearchSessionManager,
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

    let max_ply = req
        .params
        .get("max_ply")
        .or_else(|| req.params.get("params").and_then(|p| p.get("max_ply")))
        .and_then(|v| v.as_u64())
        .map(|p| p as usize);

    let (total_games, db_key) = match db {
        DatabaseBackend::Scid(s) => {
            let count = s.game_count();
            (count, SearchSessionManager::db_key(&s.index_path, count))
        }
        DatabaseBackend::Pgn(p) => {
            let count = p.game_count();
            (count, SearchSessionManager::db_key(&p.pgn_path, count))
        }
    };

    let query_key = format!(
        "pos:{}:{}:{}:{:?}",
        fen.trim(),
        turn_param.unwrap_or("*"),
        mode_param.unwrap_or("exact"),
        max_ply
    );

    // ⚡ Fast Cache Lookup: reuse identical query on unchanged database
    if let Some(cached) = session_mgr.find_cached(&db_key, &query_key) {
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

    let start_time = Instant::now();

    // ⚡ 1. Ultra-Fast Search Booster (.boost.idx) Scan
    let db_path = match db {
        DatabaseBackend::Scid(s) => s.index_path().to_path_buf(),
        DatabaseBackend::Pgn(p) => p.pgn_path.clone(),
    };
    let booster_path = crate::search_booster::resolve_companion_booster_path(&db_path);
    if booster_path.exists() {
        if let Ok(boost_idx) = crate::search_booster::MmapBoostIndex::open(&booster_path) {
            if boost_idx.num_games() == total_games {
                let evaluator = crate::search_booster::BoostSearchEvaluator::new(&boost_idx);
                let boost_matches_res = evaluator.search_position_with_progress(
                    fen,
                    turn_param,
                    max_ply,
                    |scanned, total, matches_len| {
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
                    },
                );
                if let Ok(boost_matches) = boost_matches_res {
                    let matches: Vec<ScidMatchResult> = boost_matches
                        .into_iter()
                        .map(|bm| ScidMatchResult {
                            game_id: bm.game_id,
                            match_details: QueryMatchResult {
                                is_match: true,
                                matching_plies: bm.matching_plies.clone(),
                                match_count: bm.matching_plies.len(),
                            },
                        })
                        .collect();
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    let matched_count = matches.len();
                    let search_id = session_mgr.create_session(
                        &db_key,
                        &query_key,
                        total_games,
                        matches,
                        duration_ms,
                    );
                    return ResponseMessage {
                        id,
                        status: "ok".to_string(),
                        data: Some(serde_json::json!({
                            "search_id": search_id,
                            "total_searched": total_games,
                            "matched_count": matched_count,
                            "duration_ms": duration_ms,
                            "engine": "search_booster",
                            "cached": false,
                        })),
                        error: None,
                    };
                }
            }
        }
    }

    // ⚡ 2. Instant Sub-Millisecond candidate lookup if PositionIndex is active
    if is_exact && turn_param.is_none() {
        if current_pos_index.is_none() {
            *current_pos_index = PositionIndex::load(&db_path).ok();
        }

        if let Some(pos_idx) = current_pos_index.as_ref() {
            if let Some((_pos, zobrist_hash)) = crate::position_index::parse_target_position(fen) {
                if let Some(game_ids) = pos_idx.get_all_position_games(zobrist_hash) {
                    let matches: Vec<ScidMatchResult> = game_ids
                        .into_iter()
                        .map(|gid| ScidMatchResult {
                            game_id: gid,
                            match_details: QueryMatchResult {
                                is_match: true,
                                matching_plies: vec![0],
                                match_count: 1,
                            },
                        })
                        .collect();
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    let matched_count = matches.len();
                    let search_id = session_mgr.create_session(
                        &db_key,
                        &query_key,
                        total_games,
                        matches,
                        duration_ms,
                    );
                    return ResponseMessage {
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
                    };
                }
            }
        }
    }

    match db {
        DatabaseBackend::Scid(s) => {
            let res = thread_pool.install(|| {
                s.search_position_with_progress(
                    fen,
                    turn_param,
                    mode_param,
                    max_ply,
                    |scanned, total, matches_len| {
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
                    },
                )
            });
            match res {
                Ok(pos_res) => {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    let matched_count = pos_res.matches.len();
                    let matches: Vec<ScidMatchResult> = pos_res
                        .matches
                        .into_iter()
                        .map(|m| ScidMatchResult {
                            game_id: m.game_id,
                            match_details: QueryMatchResult {
                                is_match: true,
                                matching_plies: vec![m.ply],
                                match_count: 1,
                            },
                        })
                        .collect();
                    let search_id = session_mgr.create_session(
                        &db_key,
                        &query_key,
                        total_games,
                        matches,
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
                    error: Some(format!("Position search failed: {}", e)),
                },
            }
        }
        DatabaseBackend::Pgn(p) => {
            let res = thread_pool.install(|| {
                p.search_position(
                    fen,
                    turn_param,
                    mode_param,
                    max_ply,
                    |scanned, total, matches_len| {
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
                    },
                )
            });
            match res {
                Ok(pos_res) => {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    let matched_count = pos_res.matches.len();
                    let matches: Vec<ScidMatchResult> = pos_res
                        .matches
                        .into_iter()
                        .map(|m| ScidMatchResult {
                            game_id: m.game_id,
                            match_details: QueryMatchResult {
                                is_match: true,
                                matching_plies: vec![m.ply],
                                match_count: 1,
                            },
                        })
                        .collect();
                    let search_id = session_mgr.create_session(
                        &db_key,
                        &query_key,
                        total_games,
                        matches,
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
                    error: Some(format!("Position search failed: {}", e)),
                },
            }
        }
    }
}

pub fn handle_search_material(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    session_mgr: &mut SearchSessionManager,
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

    let filter: crate::position_search::MaterialFilter =
        serde_json::from_value(req.params.clone()).unwrap_or_default();

    let (total_games, db_key) = match db {
        DatabaseBackend::Scid(s) => {
            let count = s.game_count();
            (count, SearchSessionManager::db_key(&s.index_path, count))
        }
        DatabaseBackend::Pgn(p) => {
            let count = p.game_count();
            (count, SearchSessionManager::db_key(&p.pgn_path, count))
        }
    };

    let query_key = format!(
        "material:{}",
        serde_json::to_string(&filter).unwrap_or_default()
    );

    // ⚡ Fast Cache Lookup: reuse identical query on unchanged database
    if let Some(cached) = session_mgr.find_cached(&db_key, &query_key) {
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

    let start_time = Instant::now();

    match db {
        DatabaseBackend::Scid(s) => {
            let res = thread_pool.install(|| {
                s.search_material_with_progress(&filter, |scanned, total, matches_len| {
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
                Ok(game_ids) => {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    let matched_count = game_ids.len();
                    let matches: Vec<ScidMatchResult> = game_ids
                        .into_iter()
                        .map(|gid| ScidMatchResult {
                            game_id: gid,
                            match_details: QueryMatchResult {
                                is_match: true,
                                matching_plies: vec![0],
                                match_count: 1,
                            },
                        })
                        .collect();
                    let search_id = session_mgr.create_session(
                        &db_key,
                        &query_key,
                        total_games,
                        matches,
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
                    error: Some(format!("Material search failed: {}", e)),
                },
            }
        }
        DatabaseBackend::Pgn(p) => {
            let res = thread_pool.install(|| {
                p.search_material(&filter, |scanned, total, matches_len| {
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
                Ok(game_ids) => {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    let matched_count = game_ids.len();
                    let matches: Vec<ScidMatchResult> = game_ids
                        .into_iter()
                        .map(|gid| ScidMatchResult {
                            game_id: gid,
                            match_details: QueryMatchResult {
                                is_match: true,
                                matching_plies: vec![0],
                                match_count: 1,
                            },
                        })
                        .collect();
                    let search_id = session_mgr.create_session(
                        &db_key,
                        &query_key,
                        total_games,
                        matches,
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
                    error: Some(format!("Material search failed: {}", e)),
                },
            }
        }
    }
}
