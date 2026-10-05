use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use crate::db::GameFilter;
use crate::endgame_index::{
    resolve_companion_feat_path, EndgameCatalog, EndgameIndexBuilder, EndgameQueryEngine,
    MmapFeatureIndex,
};
use crate::position_index::PositionIndex;
use crate::server::{DatabaseBackend, RequestMessage, ResponseMessage};
use serde_json::json;

pub fn handle_endgames(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    current_pos_index: &mut Option<PositionIndex>,
) -> ResponseMessage {
    let id = req.id;
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

    let fen_opt = req
        .params
        .get("fen")
        .or_else(|| req.params.get("position"))
        .and_then(|v| v.as_str());

    let category_opt = req
        .params
        .get("category")
        .or_else(|| req.params.get("category_id"))
        .and_then(|v| v.as_str());

    let feature_opt = req
        .params
        .get("feature_id")
        .or_else(|| req.params.get("feature"))
        .and_then(|v| v.as_str());

    let max_samples = req
        .params
        .get("max_samples")
        .or_else(|| req.params.get("limit"))
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(20);

    let catalog_path_opt = req
        .params
        .get("catalog_path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);

    let explicit_game_ids: Option<Vec<usize>> = req
        .params
        .get("game_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let use_search_results = req
        .params
        .get("use_search_results")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let filter_opt: Option<GameFilter> = req
        .params
        .get("filter")
        .or_else(|| req.params.get("params"))
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let catalog = EndgameCatalog::load_or_default(catalog_path_opt.as_deref());

    let db_path = match db {
        DatabaseBackend::Scid(s) => s.index_path(),
        DatabaseBackend::Pgn(p) => p.pgn_path.as_path(),
    };

    let feat_path = resolve_companion_feat_path(db_path);

    // Auto-build feature index if it doesn't exist
    if !feat_path.exists() {
        let builder = EndgameIndexBuilder::with_catalog(catalog.clone());
        let build_res = match db {
            DatabaseBackend::Scid(s) => builder.build_for_scid(s, Some(feat_path.clone()), None),
            DatabaseBackend::Pgn(p) => {
                builder.build_for_pgn(&p.pgn_path, Some(feat_path.clone()), None)
            }
        };
        if let Err(e) = build_res {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some(format!("Failed to auto-build endgame feature index: {}", e)),
            };
        }
    }

    let mmap_idx = match MmapFeatureIndex::open(&feat_path) {
        Ok(idx) => idx,
        Err(e) => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some(format!(
                    "Failed to open feature index at {:?}: {}",
                    feat_path, e
                )),
            };
        }
    };

    // 1. Filter Game IDs Resolution (search session, inline filter, or explicit game_ids)
    let mut filter_game_ids: Option<Vec<usize>> = explicit_game_ids;
    if filter_game_ids.is_none() && use_search_results {
        filter_game_ids = match db {
            DatabaseBackend::Scid(s) => s.get_cached_query_indices(),
            DatabaseBackend::Pgn(p) => p.get_cached_query_indices(),
        };
    } else if filter_game_ids.is_none() {
        if let Some(ref f) = filter_opt {
            if !f.is_empty() {
                match db {
                    DatabaseBackend::Scid(s) => {
                        let _ = s.query_games(f, 0, 0);
                    }
                    DatabaseBackend::Pgn(p) => {
                        let _ = p.query_games(f, 0, 0);
                    }
                };
                filter_game_ids = match db {
                    DatabaseBackend::Scid(s) => s.get_cached_query_indices(),
                    DatabaseBackend::Pgn(p) => p.get_cached_query_indices(),
                };
            }
        }
    }

    // 2. Position Filter Candidate Resolution
    let mut position_game_ids: Option<HashSet<usize>> = None;
    if let Some(fen_str) = fen_opt {
        let booster_path = crate::search_booster::resolve_companion_booster_path(db_path);
        if booster_path.exists() {
            if let Ok(boost_idx) = crate::search_booster::MmapBoostIndex::open(&booster_path) {
                let evaluator = crate::search_booster::BoostSearchEvaluator::new(&boost_idx);
                if let Ok(matches) = evaluator.search_position(fen_str, None) {
                    position_game_ids = Some(matches.into_iter().map(|m| m.game_id).collect());
                }
            }
        }
        if position_game_ids.is_none() {
            match db {
                DatabaseBackend::Scid(s) => {
                    if let Ok(res) = s.search_position(fen_str, None, None, None) {
                        position_game_ids =
                            Some(res.matches.into_iter().map(|m| m.game_id).collect());
                    }
                }
                DatabaseBackend::Pgn(p) => {
                    if let Ok(res) = p.search_position(fen_str, None, None, None, |_, _, _| {}) {
                        position_game_ids =
                            Some(res.matches.into_iter().map(|m| m.game_id).collect());
                    }
                }
            }
        }
    }

    // 3. Merge Position & Search Filters
    let candidate_ids: Option<Vec<u32>> = match (position_game_ids, filter_game_ids) {
        (Some(pos_set), Some(filt_list)) => Some(
            filt_list
                .into_iter()
                .filter(|id| pos_set.contains(id))
                .map(|id| id as u32)
                .collect(),
        ),
        (Some(pos_set), None) => Some(pos_set.into_iter().map(|id| id as u32).collect()),
        (None, Some(filt_list)) => Some(filt_list.into_iter().map(|id| id as u32).collect()),
        (None, None) => None,
    };

    // 4. Feature Query Mode
    if let Some(feat_id) = feature_opt {
        match EndgameQueryEngine::query_feature_filtered(
            &mmap_idx,
            &catalog,
            feat_id,
            max_samples,
            candidate_ids.as_deref(),
        ) {
            Ok(rep) => {
                return ResponseMessage {
                    id,
                    status: "ok".to_string(),
                    data: serde_json::to_value(&rep).ok(),
                    error: None,
                };
            }
            Err(e) => {
                return ResponseMessage {
                    id,
                    status: "error".to_string(),
                    data: None,
                    error: Some(format!("Feature query failed: {}", e)),
                };
            }
        }
    }

    // 3. Result Callback Function
    let get_result = move |gid: u32| -> u8 {
        match db {
            DatabaseBackend::Scid(s) => {
                s.entries().get(gid as usize).map(|e| e.result).unwrap_or(0)
            }
            DatabaseBackend::Pgn(p) => p.entries.get(gid as usize).map(|e| e.result).unwrap_or(0),
        }
    };

    let mut report = match EndgameQueryEngine::calculate_popularity(
        &mmap_idx,
        &catalog,
        &db_path.to_string_lossy(),
        candidate_ids.as_deref(),
        fen_opt,
        Some(&get_result),
    ) {
        Ok(rep) => rep,
        Err(e) => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some(format!("Failed to calculate endgame popularity: {}", e)),
            };
        }
    };

    // Filter by category if requested
    if let Some(cat_filter) = category_opt {
        let cat_upper = cat_filter.to_uppercase();
        report
            .categories
            .retain(|c| c.category_id.to_uppercase() == cat_upper);
        report
            .features
            .retain(|f| f.category_id.to_uppercase() == cat_upper);
    }

    ResponseMessage {
        id,
        status: "ok".to_string(),
        data: serde_json::to_value(&report).ok(),
        error: None,
    }
}

pub fn handle_build_endgames(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
) -> ResponseMessage {
    let id = req.id;
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

    let custom_output = req
        .params
        .get("output_path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);

    let catalog_path_opt = req
        .params
        .get("catalog_path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);

    let catalog = EndgameCatalog::load_or_default(catalog_path_opt.as_deref());
    let builder = EndgameIndexBuilder::with_catalog(catalog);

    let progress_cb = Arc::new(|done: usize, total: usize, pct: f64| {
        let event_json = json!({
            "event": "build_endgames_progress",
            "data": {
                "scanned": done,
                "total": total,
                "percent": pct,
            }
        });
        if let Ok(line) = serde_json::to_string(&event_json) {
            let mut out = std::io::stdout().lock();
            let _ = writeln!(out, "{}", line);
            let _ = out.flush();
        }
    });

    let res = match db {
        DatabaseBackend::Scid(s) => {
            let out_target =
                custom_output.unwrap_or_else(|| resolve_companion_feat_path(s.index_path()));
            builder.build_for_scid(s, Some(out_target), Some(progress_cb))
        }
        DatabaseBackend::Pgn(p) => {
            let out_target =
                custom_output.unwrap_or_else(|| resolve_companion_feat_path(&p.pgn_path));
            builder.build_for_pgn(&p.pgn_path, Some(out_target), Some(progress_cb))
        }
    };

    match res {
        Ok((path, total_games, elapsed_ms)) => ResponseMessage {
            id,
            status: "ok".to_string(),
            data: Some(json!({
                "output_path": path.to_string_lossy(),
                "total_games": total_games,
                "elapsed_ms": elapsed_ms,
            })),
            error: None,
        },
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Failed to build endgame feature index: {}", e)),
        },
    }
}
