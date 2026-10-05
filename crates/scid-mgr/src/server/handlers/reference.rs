use crate::continuation_index::{ContinuationQuery, HotGraphQueryable};
use crate::db::GameFilter;
use crate::endgame_index::{
    resolve_companion_feat_path, EndgameCatalog, EndgameQueryEngine, MmapFeatureIndex,
};
use crate::position_index::PositionIndex;
use crate::search::evaluator::QueryMatchResult;
use crate::search::ScidMatchResult;
use crate::server::search_session::{
    PositionMatchMode, SearchSessionManager, SessionOwner, SessionQuery,
};
use crate::server::{DatabaseBackend, RequestMessage, ResponseMessage};
use crate::tree_index::{OpeningTreeReport, TreeIndex};
use serde_json::json;
use shakmaty::fen::Fen;
use shakmaty::{CastlingMode, Chess};
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

/// Unified Reference Explorer endpoint orchestrating Tree, Continuations, Endgames, and Games Table
pub fn handle_reference(
    req: &RequestMessage,
    current_db: &Option<DatabaseBackend>,
    current_pos_index: &mut Option<PositionIndex>,
    current_tree_index: &mut Option<TreeIndex>,
    session_mgr: &mut SearchSessionManager,
    _thread_pool: &rayon::ThreadPool,
    cancel_token: &Arc<AtomicBool>,
) -> ResponseMessage {
    let start_time = Instant::now();
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

    let fen_param = req
        .params
        .get("fen")
        .or_else(|| req.params.get("position"))
        .or_else(|| {
            req.params
                .get("params")
                .and_then(|p| p.get("fen").or_else(|| p.get("position")))
        })
        .and_then(|v| v.as_str())
        .unwrap_or("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

    let fen_str = fen_param.trim();

    // Parse and validate FEN
    let fen_parsed: Result<Fen, _> = fen_str.parse();
    let target_pos = match fen_parsed {
        Ok(fen) => match fen.into_position::<Chess>(CastlingMode::Chess960) {
            Ok(p) => p,
            Err(e) => {
                return ResponseMessage {
                    id,
                    status: "error".to_string(),
                    data: None,
                    error: Some(format!("Invalid chess position '{}': {}", fen_str, e)),
                };
            }
        },
        Err(e) => {
            return ResponseMessage {
                id,
                status: "error".to_string(),
                data: None,
                error: Some(format!("Invalid FEN '{}': {}", fen_str, e)),
            };
        }
    };

    // Component Inclusion Flags
    let include_tree = req
        .params
        .get("include_tree")
        .or_else(|| req.params.get("tree"))
        .or_else(|| {
            req.params
                .get("params")
                .and_then(|p| p.get("include_tree").or_else(|| p.get("tree")))
        })
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let include_continuations = req
        .params
        .get("include_continuations")
        .or_else(|| req.params.get("continuations"))
        .or_else(|| {
            req.params.get("params").and_then(|p| {
                p.get("include_continuations")
                    .or_else(|| p.get("continuations"))
            })
        })
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let include_endgames = req
        .params
        .get("include_endgames")
        .or_else(|| req.params.get("endgames"))
        .or_else(|| {
            req.params
                .get("params")
                .and_then(|p| p.get("include_endgames").or_else(|| p.get("endgames")))
        })
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let include_games = req
        .params
        .get("include_games")
        .or_else(|| req.params.get("games"))
        .or_else(|| {
            req.params
                .get("params")
                .and_then(|p| p.get("include_games").or_else(|| p.get("games")))
        })
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // Games Pagination Parameters
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
        .unwrap_or(20) as usize;

    let sort_by = req
        .params
        .get("sort_by")
        .or_else(|| req.params.get("params").and_then(|p| p.get("sort_by")))
        .and_then(|v| v.as_str());

    let sort_asc = req
        .params
        .get("sort_asc")
        .or_else(|| req.params.get("params").and_then(|p| p.get("sort_asc")))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let filter_opt: Option<GameFilter> = req
        .params
        .get("filter")
        .or_else(|| req.params.get("params").and_then(|p| p.get("filter")))
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let max_sample_ids: Option<usize> = req
        .params
        .get("max_sample_games")
        .or_else(|| req.params.get("max_samples"))
        .or_else(|| {
            req.params
                .get("params")
                .and_then(|p| p.get("max_sample_games").or_else(|| p.get("max_samples")))
        })
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .or(Some(20));

    let continuation_depth = req
        .params
        .get("continuation_depth")
        .or_else(|| req.params.get("depth"))
        .or_else(|| req.params.get("max_depth"))
        .or_else(|| {
            req.params.get("params").and_then(|p| {
                p.get("continuation_depth")
                    .or_else(|| p.get("depth"))
                    .or_else(|| p.get("max_depth"))
            })
        })
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(8);

    let max_lines = req
        .params
        .get("max_lines")
        .or_else(|| req.params.get("lines"))
        .or_else(|| {
            req.params
                .get("params")
                .and_then(|p| p.get("max_lines").or_else(|| p.get("lines")))
        })
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(10);

    let min_games = req
        .params
        .get("min_games")
        .or_else(|| req.params.get("params").and_then(|p| p.get("min_games")))
        .and_then(|v| v.as_u64())
        .unwrap_or(1);

    let min_percentage = req
        .params
        .get("min_percentage")
        .or_else(|| req.params.get("percentage"))
        .or_else(|| {
            req.params
                .get("params")
                .and_then(|p| p.get("min_percentage").or_else(|| p.get("percentage")))
        })
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let continuation_config = if include_continuations {
        Some(crate::continuation_index::ContinuationQuery {
            position: fen_str.to_string(),
            max_depth: continuation_depth,
            max_lines,
            min_games,
            min_percentage,
            hot_idx: None,
            pos_idx: None,
        })
    } else {
        None
    };

    let (total_db_games, db_key) = match db {
        DatabaseBackend::Scid(s) => {
            let count = s.game_count();
            (count, SearchSessionManager::db_key(&s.index_path, count))
        }
        DatabaseBackend::Pgn(p) => {
            let count = p.game_count();
            (count, SearchSessionManager::db_key(&p.pgn_path, count))
        }
    };

    let db_path = match db {
        DatabaseBackend::Scid(s) => s.index_path().to_path_buf(),
        DatabaseBackend::Pgn(p) => p.pgn_path.clone(),
    };

    let query_key = format!("ref_pos:{}:{:?}", fen_str, filter_opt);

    // 1. Position Search Session Resolution (Reference LRU Pool)
    let search_id = if let Some(cached) = session_mgr.find_cached(&db_key, &query_key) {
        cached.search_id.clone()
    } else {
        let pos_matches = evaluate_position_matches(fen_str, db, &db_path, filter_opt.as_ref());

        let duration = start_time.elapsed().as_millis() as u64;
        let session_query = match filter_opt.clone() {
            Some(f) => SessionQuery::FilteredPosition {
                fen: fen_str.to_string(),
                match_mode: PositionMatchMode::Exact,
                max_ply: None,
                filter: Box::new(f),
            },
            None => SessionQuery::PurePosition {
                fen: fen_str.to_string(),
                match_mode: PositionMatchMode::Exact,
                max_ply: None,
            },
        };

        session_mgr.create_session_with_metadata(
            &db_key,
            &query_key,
            total_db_games,
            pos_matches,
            duration,
            SessionOwner::Reference,
            session_query,
        )
    };

    // Extract candidate game IDs for downstream computations
    let (candidate_game_ids, matched_total) = {
        let session = session_mgr.get_session(&search_id).unwrap();
        let ids: Vec<usize> = session.matches.iter().map(|m| m.game_id).collect();
        let count = session.matches.len();
        (ids, count)
    };

    // 2. Opening Tree & Continuations Computation (Single-Pass Combined)
    let (tree_data, continuations_data) = if include_tree || include_continuations {
        let report = calculate_tree_and_continuations(
            fen_str,
            &target_pos,
            db,
            &db_path,
            current_tree_index,
            current_pos_index,
            if filter_opt.is_some() {
                Some(&candidate_game_ids[..])
            } else {
                None
            },
            max_sample_ids,
            continuation_config.as_ref(),
            cancel_token,
        );

        match report {
            Some(rep) => {
                let tree_val = if include_tree {
                    Some(json!({
                        "moves": rep.moves,
                        "total_games": rep.total_games,
                        "white_wins": rep.white_wins,
                        "draws": rep.draws,
                        "black_wins": rep.black_wins,
                        "white_pct": rep.white_pct,
                        "draw_pct": rep.draw_pct,
                        "black_pct": rep.black_pct,
                        "sample_game_ids": rep.sample_game_ids,
                    }))
                } else {
                    None
                };
                let cont_val = if include_continuations {
                    Some(json!(rep.continuations.unwrap_or_default()))
                } else {
                    None
                };
                (tree_val, cont_val)
            }
            None => (
                if include_tree {
                    Some(
                        json!({ "moves": [], "total_games": 0, "white_pct": 0.0, "draw_pct": 0.0, "black_pct": 0.0 }),
                    )
                } else {
                    None
                },
                if include_continuations {
                    Some(json!([]))
                } else {
                    None
                },
            ),
        }
    } else {
        (None, None)
    };

    // 3. Endgames Popularity Computation
    let endgames_data = if include_endgames {
        calculate_endgame_report(
            fen_str,
            db,
            &db_path,
            if filter_opt.is_some() || !candidate_game_ids.is_empty() {
                Some(&candidate_game_ids[..])
            } else {
                None
            },
        )
    } else {
        None
    };

    // 4. Games Table Virtual Pagination (from Session)
    let games_data = if include_games {
        let session = session_mgr.get_session_mut(&search_id).unwrap();
        let (slice, _) = session.get_sorted_slice(sort_by, sort_asc, page, page_size, db);
        let summaries: Vec<serde_json::Value> = match db {
            DatabaseBackend::Scid(s) => slice
                .iter()
                .filter_map(|m| {
                    let mut summ = s.get_game_summary(m.game_id)?;
                    summ.matching_plies = Some(m.match_details.matching_plies.clone());
                    summ.match_count = Some(m.match_details.match_count);
                    serde_json::to_value(&summ).ok()
                })
                .collect(),
            DatabaseBackend::Pgn(p) => slice
                .iter()
                .map(|m| {
                    let mut summ = p.get_summary(m.game_id);
                    summ.matching_plies = Some(m.match_details.matching_plies.clone());
                    summ.match_count = Some(m.match_details.match_count);
                    serde_json::to_value(&summ).unwrap_or_default()
                })
                .collect(),
        };
        Some(json!(summaries))
    } else {
        None
    };

    let total_elapsed = start_time.elapsed().as_millis() as u64;

    ResponseMessage {
        id,
        status: "ok".to_string(),
        data: Some(json!({
            "fen": fen_str,
            "total_games": matched_total,
            "search_id": search_id,
            "tree": tree_data,
            "continuations": continuations_data,
            "endgames": endgames_data,
            "games": games_data,
            "page": page,
            "page_size": page_size,
            "duration_ms": total_elapsed,
        })),
        error: None,
    }
}

/// Evaluates matching games for a position, utilizing .boost.idx or direct database scan
fn evaluate_position_matches(
    fen_str: &str,
    db: &DatabaseBackend,
    db_path: &std::path::Path,
    filter_opt: Option<&GameFilter>,
) -> Vec<ScidMatchResult> {
    let mut matching_ids = Vec::new();

    let booster_path = crate::search_booster::resolve_companion_booster_path(db_path);
    if booster_path.exists() {
        if let Ok(boost_idx) = crate::search_booster::MmapBoostIndex::open(&booster_path) {
            let evaluator = crate::search_booster::BoostSearchEvaluator::new(&boost_idx);
            if let Ok(boost_res) = evaluator.search_position(fen_str, None) {
                matching_ids = boost_res.into_iter().map(|m| m.game_id).collect();
            }
        }
    } else {
        match db {
            DatabaseBackend::Scid(s) => {
                if let Ok(res) = s.search_position(fen_str, None, None, None) {
                    matching_ids = res.matches.into_iter().map(|m| m.game_id).collect();
                }
            }
            DatabaseBackend::Pgn(p) => {
                if let Ok(res) = p.search_position(fen_str, None, None, None, |_, _, _| {}) {
                    matching_ids = res.matches.into_iter().map(|m| m.game_id).collect();
                }
            }
        }
    }

    // Apply metadata filter if present
    if let Some(filt) = filter_opt {
        if !filt.is_empty() {
            let filter_set: HashSet<usize> = match db {
                DatabaseBackend::Scid(s) => {
                    let _ = s.query_games(filt, 0, 0);
                    s.get_cached_query_indices()
                        .unwrap_or_default()
                        .into_iter()
                        .collect()
                }
                DatabaseBackend::Pgn(p) => {
                    let _ = p.query_games(filt, 0, 0);
                    p.get_cached_query_indices()
                        .unwrap_or_default()
                        .into_iter()
                        .collect()
                }
            };
            matching_ids.retain(|gid| filter_set.contains(gid));
        }
    }

    matching_ids
        .into_iter()
        .map(|gid| ScidMatchResult {
            game_id: gid,
            match_details: QueryMatchResult {
                is_match: true,
                matching_plies: vec![],
                match_count: 1,
            },
        })
        .collect()
}

/// Calculates opening tree move stats and continuation lines in a single pass
#[allow(clippy::too_many_arguments)]
fn calculate_tree_and_continuations(
    fen_str: &str,
    target_pos: &Chess,
    db: &DatabaseBackend,
    db_path: &std::path::Path,
    current_tree_index: &mut Option<TreeIndex>,
    _current_pos_index: &mut Option<PositionIndex>,
    target_game_ids: Option<&[usize]>,
    max_sample_ids: Option<usize>,
    continuation_config: Option<&ContinuationQuery>,
    _cancel_token: &Arc<AtomicBool>,
) -> Option<OpeningTreeReport> {
    // 0. Instant lookup from .hot.idx (< 1 ms) if unfiltered
    if target_game_ids.is_none() {
        let hot_path = crate::continuation_index::resolve_companion_hot_path(db_path);
        if hot_path.exists() {
            if let Ok(mmap_hot) = crate::continuation_index::MmapHotGraph::open(&hot_path) {
                if let Some(rep) =
                    mmap_hot.query_opening_tree(target_pos, fen_str, continuation_config)
                {
                    if rep.total_games > 0 {
                        return Some(rep);
                    }
                }
            }
        }
    }

    // 1. Dynamic calculation from .boost.idx
    let booster_path = crate::search_booster::resolve_companion_booster_path(db_path);
    if booster_path.exists() {
        if let Ok(boost_idx) = crate::search_booster::MmapBoostIndex::open(&booster_path) {
            let evaluator = crate::search_booster::BoostSearchEvaluator::new(&boost_idx);
            match db {
                DatabaseBackend::Scid(s) => {
                    let entries = s.entries();
                    let meta_lookup = |gid: usize| -> Option<crate::search_booster::BoostGameMeta> {
                        entries.get(gid).map(|e| {
                            let year = if e.date > 0 {
                                Some((e.date / 10000) as u16)
                            } else {
                                None
                            };
                            crate::search_booster::BoostGameMeta::new(
                                e.result,
                                e.white_elo,
                                e.black_elo,
                                year,
                            )
                        })
                    };
                    if let Ok(Some(rep)) = evaluator.calculate_opening_tree(
                        fen_str,
                        target_game_ids,
                        max_sample_ids,
                        Some(meta_lookup),
                        continuation_config,
                    ) {
                        return Some(rep);
                    }
                }
                DatabaseBackend::Pgn(p) => {
                    let entries = &p.entries;
                    let meta_lookup = |gid: usize| -> Option<crate::search_booster::BoostGameMeta> {
                        entries.get(gid).map(|e| {
                            let res = match e.result {
                                1 => 1,
                                2 => 2,
                                3 => 3,
                                _ => 0,
                            };
                            let year = if e.date > 0 {
                                Some((e.date / 10000) as u16)
                            } else {
                                None
                            };
                            crate::search_booster::BoostGameMeta::new(
                                res,
                                e.white_elo,
                                e.black_elo,
                                year,
                            )
                        })
                    };
                    if let Ok(Some(rep)) = evaluator.calculate_opening_tree(
                        fen_str,
                        target_game_ids,
                        max_sample_ids,
                        Some(meta_lookup),
                        continuation_config,
                    ) {
                        return Some(rep);
                    }
                }
            }
        }
    }

    // 2. Pre-calculated .tree.idx lookup if unfiltered
    if current_tree_index.is_none() {
        *current_tree_index = TreeIndex::load(db_path).ok();
    }
    if let Some(ref tree_idx) = current_tree_index {
        if let Some(rep) =
            tree_idx.query_tree_with_options(fen_str, target_game_ids, max_sample_ids)
        {
            return Some(rep);
        }
    }

    None
}

/// Calculates endgame distribution for a position using .feat.idx
fn calculate_endgame_report(
    fen_str: &str,
    db: &DatabaseBackend,
    db_path: &std::path::Path,
    candidate_ids: Option<&[usize]>,
) -> Option<serde_json::Value> {
    let feat_path = resolve_companion_feat_path(db_path);
    if !feat_path.exists() {
        return None;
    }

    let mmap_idx = MmapFeatureIndex::open(&feat_path).ok()?;
    let catalog = EndgameCatalog::default_catalog();

    let candidate_u32: Option<Vec<u32>> =
        candidate_ids.map(|ids| ids.iter().map(|&id| id as u32).collect());

    let get_result = |gid: u32| -> u8 {
        match db {
            DatabaseBackend::Scid(s) => {
                s.entries().get(gid as usize).map(|e| e.result).unwrap_or(0)
            }
            DatabaseBackend::Pgn(p) => p.entries.get(gid as usize).map(|e| e.result).unwrap_or(0),
        }
    };

    let report = EndgameQueryEngine::calculate_popularity(
        &mmap_idx,
        &catalog,
        &db_path.to_string_lossy(),
        candidate_u32.as_deref(),
        Some(fen_str),
        Some(&get_result),
    )
    .ok()?;

    serde_json::to_value(&report).ok()
}
