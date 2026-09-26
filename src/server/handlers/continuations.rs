use serde_json::json;
use std::time::Instant;

use crate::continuation_index::{
    calculate_continuations_for_pgn, calculate_continuations_for_scid, resolve_companion_hot_path,
    ContinuationQuery, ContinuationResult, HotGraphBuildConfig, HotGraphQueryable, MmapHotGraph,
};
use crate::position_index::PositionIndex;
use crate::server::{DatabaseBackend, RequestMessage, ResponseMessage};
use shakmaty::zobrist::{Zobrist64, ZobristHash};

pub fn handle_continuations(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    current_pos_index: &mut Option<PositionIndex>,
) -> ResponseMessage {
    let id = req.id;
    let fen_str = req
        .params
        .get("fen")
        .or_else(|| req.params.get("position"))
        .and_then(|v| v.as_str())
        .unwrap_or("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

    let max_depth = req
        .params
        .get("max_depth")
        .or_else(|| req.params.get("depth"))
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(8);

    let max_lines = req
        .params
        .get("max_lines")
        .or_else(|| req.params.get("lines"))
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(10);

    let min_games = req
        .params
        .get("min_games")
        .and_then(|v| v.as_u64())
        .unwrap_or(1);

    let min_percentage = req
        .params
        .get("min_percentage")
        .or_else(|| req.params.get("percentage"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

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

    let query = ContinuationQuery {
        position: fen_str.to_string(),
        max_depth,
        max_lines,
        min_games,
        min_percentage,
        hot_idx: None,
        pos_idx: None,
    };

    let target_pos = match query.validate() {
        Ok(p) => p,
        Err(e) => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some(format!("Invalid FEN or parameters: {}", e)),
            };
        }
    };

    let db_path = match db {
        DatabaseBackend::Scid(s) => s.index_path(),
        DatabaseBackend::Pgn(p) => p.pgn_path.as_path(),
    };

    let hot_path = resolve_companion_hot_path(db_path);

    // 1. Fast Memory-Mapped Hot Graph path if companion index exists
    if hot_path.exists() {
        if let Ok(mmap_hot) = MmapHotGraph::open(&hot_path) {
            let res = mmap_hot.query_continuations(
                &target_pos,
                fen_str,
                max_depth,
                max_lines,
                min_games,
                min_percentage,
            );
            return ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::to_value(res).unwrap_or(json!({}))),
                error: None,
            };
        }
    }

    // 2. Candidate acceleration via companion .pos.idx if present
    let mut candidate_ids: Option<Vec<usize>> = None;
    let target_hash_val: Zobrist64 = target_pos.zobrist_hash(shakmaty::EnPassantMode::Legal);
    let target_hash = target_hash_val.0;

    if let Some(pos_idx) = current_pos_index.as_ref() {
        if let Some(postings) = pos_idx.get_all_position_games(target_hash) {
            candidate_ids = Some(postings.into_iter().map(|id| id as usize).collect());
        } else {
            let empty_res = ContinuationResult {
                starting_fen: fen_str.to_string(),
                total_games_processed: pos_idx.header.db_game_count,
                games_reaching_position: 0,
                lines: Vec::new(),
                tree: None,
            };
            return ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::to_value(empty_res).unwrap_or(json!({}))),
                error: None,
            };
        }
    }

    // 3. Dynamic on-the-fly calculation
    let res = match db {
        DatabaseBackend::Scid(s) => calculate_continuations_for_scid(
            s.entries(),
            s.games_path(),
            &query,
            candidate_ids.as_deref(),
        ),
        DatabaseBackend::Pgn(p) => {
            calculate_continuations_for_pgn(&p.pgn_path, &query, candidate_ids.as_deref())
        }
    };

    match res {
        Some(result) => ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(serde_json::to_value(result).unwrap_or(json!({}))),
            error: None,
        },
        None => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some("Failed to calculate continuations".to_string()),
        },
    }
}

use std::io::Write;

pub fn handle_build_continuations_index(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    current_thread_count: usize,
) -> ResponseMessage {
    let id = req.id;
    let max_ply = req
        .params
        .get("max_ply")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(24);

    let min_games = req
        .params
        .get("min_games")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(1);

    let threads = req
        .params
        .get("threads")
        .and_then(|v| v.as_u64())
        .map(|t| t as usize)
        .or(Some(current_thread_count));

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

    let start = Instant::now();
    let config = HotGraphBuildConfig { max_ply, min_games };

    let res = match db {
        DatabaseBackend::Scid(s) => {
            let dest_path = resolve_companion_hot_path(s.index_path());
            crate::continuation_index::build_for_scid_direct(
                s.index_path(),
                s.entries(),
                s.games_path(),
                &dest_path,
                config,
                threads,
                |scanned, total, nodes| {
                    let event_json = serde_json::json!({
                        "event": "build_continuations_progress",
                        "data": {
                            "scanned": scanned,
                            "total": total,
                            "nodes": nodes,
                            "positions": nodes,
                            "percent": if total > 0 { (scanned as f64 / total as f64) * 100.0 } else { 100.0 }
                        }
                    });
                    if let Ok(line) = serde_json::to_string(&event_json) {
                        let mut out = std::io::stdout().lock();
                        let _ = writeln!(out, "{}", line);
                        let _ = out.flush();
                    }
                },
            )
        }
        DatabaseBackend::Pgn(p) => {
            let dest_path = resolve_companion_hot_path(&p.pgn_path);
            crate::continuation_index::build_for_pgn_direct(
                &p.pgn_path,
                &dest_path,
                config,
                threads,
                |scanned, total, nodes| {
                    let event_json = serde_json::json!({
                        "event": "build_continuations_progress",
                        "data": {
                            "scanned": scanned,
                            "total": total,
                            "nodes": nodes,
                            "positions": nodes,
                            "percent": if total > 0 { (scanned as f64 / total as f64) * 100.0 } else { 100.0 }
                        }
                    });
                    if let Ok(line) = serde_json::to_string(&event_json) {
                        let mut out = std::io::stdout().lock();
                        let _ = writeln!(out, "{}", line);
                        let _ = out.flush();
                    }
                },
            )
        }
    };

    match res {
        Ok(meta) => ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(json!({
                "elapsed_ms": start.elapsed().as_millis(),
                "node_count": meta.node_count,
                "edge_count": meta.edge_count,
                "hash_count": meta.hash_count,
                "db_game_count": meta.db_game_count,
            })),
            error: None,
        },
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Failed to build continuations index: {}", e)),
        },
    }
}
