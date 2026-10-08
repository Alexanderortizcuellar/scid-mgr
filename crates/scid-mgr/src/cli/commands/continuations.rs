use anyhow::{Context, Result};
use std::path::Path;

use crate::continuation_index::{
    build_for_pgn, build_for_scid, calculate_continuations_for_pgn,
    calculate_continuations_for_scid, resolve_companion_hot_path, ContinuationQuery,
    ContinuationResult, HotGraphBuildConfig, HotGraphQueryable, MmapHotGraph,
};
use crate::db::ScidDatabaseWrapper;
use crate::position_index::PositionIndex;
use shakmaty::zobrist::ZobristHash;

pub fn handle_build_continuations(db_path: &Path, max_ply: usize, min_games: usize) -> Result<()> {
    let start = std::time::Instant::now();
    let path_str = db_path.to_string_lossy().to_lowercase();
    let dest_path = resolve_companion_hot_path(db_path);
    let config = HotGraphBuildConfig { max_ply, min_games };

    let meta = if path_str.ends_with(".pgn") {
        build_for_pgn(db_path, &dest_path, config)?
    } else {
        build_for_scid(db_path, &dest_path, config)?
    };

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "[OK] Built Common Continuations Hot Graph {} in {:.2} ms ({} nodes, {} edges, {} indexed positions).",
        dest_path.display(),
        elapsed_ms,
        meta.node_count,
        meta.edge_count,
        meta.hash_count
    );
    Ok(())
}

pub fn handle_continuations(
    db_path: &Path,
    fen: Option<String>,
    max_depth: usize,
    max_lines: usize,
    min_games: u64,
    min_percentage: f64,
) -> Result<()> {
    let fen_str = fen
        .as_deref()
        .unwrap_or("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let query = ContinuationQuery {
        position: fen_str.to_string(),
        max_depth,
        max_lines,
        min_games,
        min_percentage,
        hot_idx: None,
        pos_idx: None,
    };

    let target_pos = query.validate()?;

    // 1. Ultra-fast dynamic calculation via Search Booster (.boost.idx) if present
    let booster_path = crate::search_booster::resolve_companion_booster_path(db_path);
    if booster_path.exists() {
        if let Ok(boost_idx) = crate::search_booster::MmapBoostIndex::open(&booster_path) {
            let evaluator = crate::search_booster::BoostSearchEvaluator::new(&boost_idx);
            let lower = db_path.to_string_lossy().to_lowercase();
            let res = if lower.ends_with(".pgn") {
                if let Ok(pgn_db) = crate::pgn_db::PgnDatabaseWrapper::open(db_path) {
                    let meta_lookup = |gid: usize| -> Option<crate::search_booster::BoostGameMeta> {
                        pgn_db.entries.get(gid).map(|e| {
                            let res = match e.result {
                                1 => 1,
                                2 => 2,
                                3 => 3,
                                _ => 0,
                            };
                            let year = {
                                let y = (e.date >> 9) as u16;
                                if y > 0 {
                                    Some(y)
                                } else {
                                    None
                                }
                            };
                            let month = {
                                let m = ((e.date >> 5) & 0x0F) as u8;
                                if (1..=12).contains(&m) {
                                    Some(m)
                                } else {
                                    None
                                }
                            };
                            crate::search_booster::BoostGameMeta::new(
                                res,
                                e.white_elo,
                                e.black_elo,
                                year,
                                month,
                            )
                        })
                    };
                    evaluator
                        .calculate_continuations(&query, None, Some(meta_lookup))
                        .ok()
                        .flatten()
                } else {
                    None
                }
            } else if let Ok(scid_db) = ScidDatabaseWrapper::open(db_path) {
                let entries = scid_db.entries();
                let meta_lookup = |gid: usize| -> Option<crate::search_booster::BoostGameMeta> {
                    entries.get(gid).map(|e| {
                        let year = {
                            let y = (e.date >> 9) as u16;
                            if y > 0 {
                                Some(y)
                            } else {
                                None
                            }
                        };
                        let month = {
                            let m = ((e.date >> 5) & 0x0F) as u8;
                            if (1..=12).contains(&m) {
                                Some(m)
                            } else {
                                None
                            }
                        };
                        crate::search_booster::BoostGameMeta::new(
                            e.result,
                            e.white_elo,
                            e.black_elo,
                            year,
                            month,
                        )
                    })
                };
                evaluator
                    .calculate_continuations(&query, None, Some(meta_lookup))
                    .ok()
                    .flatten()
            } else {
                None
            };

            if let Some(res) = res {
                print_continuation_report(&res);
                return Ok(());
            }
        }
    }

    let hot_path = resolve_companion_hot_path(db_path);

    // 2. Fast Memory-Mapped Hot Graph path if companion index exists
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
            print_continuation_report(&res);
            return Ok(());
        }
    }

    // 2. Candidate acceleration via companion .pos.idx if present
    let mut candidate_ids: Option<Vec<usize>> = None;
    if let Ok(pos_idx) = PositionIndex::load(db_path) {
        let target_hash_val: shakmaty::zobrist::Zobrist64 =
            target_pos.zobrist_hash(shakmaty::EnPassantMode::Legal);
        let target_hash = target_hash_val.0;
        if let Some(postings) = pos_idx.get_all_position_games(target_hash) {
            candidate_ids = Some(postings.into_iter().collect());
        } else {
            // Position does not occur in database
            let empty_res = ContinuationResult {
                starting_fen: fen_str.to_string(),
                total_games_processed: pos_idx.header.db_game_count,
                games_reaching_position: 0,
                lines: Vec::new(),
                tree: None,
            };
            print_continuation_report(&empty_res);
            return Ok(());
        }
    }

    // 3. Dynamic on-the-fly calculation for PGN or SCID
    let path_str = db_path.to_string_lossy().to_lowercase();
    let res = if path_str.ends_with(".pgn") {
        calculate_continuations_for_pgn(db_path, &query, candidate_ids.as_deref())
            .context("Failed to calculate continuations for PGN")?
    } else {
        let db = ScidDatabaseWrapper::open(db_path)?;
        calculate_continuations_for_scid(
            db.entries(),
            db.games_path(),
            &query,
            candidate_ids.as_deref(),
        )
        .context("Failed to calculate continuations for SCID")?
    };

    print_continuation_report(&res);
    Ok(())
}

fn print_continuation_report(res: &ContinuationResult) {
    println!("\n================================================================================");
    println!("  COMMON CONTINUATIONS REPORT");
    println!("================================================================================");
    println!("  Position:    {}", res.starting_fen);
    println!(
        "  Frequency:   {} / {} games reached position ({:.1}%)",
        res.games_reaching_position,
        res.total_games_processed,
        if res.total_games_processed > 0 {
            (res.games_reaching_position as f64 / res.total_games_processed as f64) * 100.0
        } else {
            0.0
        }
    );
    println!("--------------------------------------------------------------------------------");
    println!(
        "  {:<4}  {:<45}  {:<8}  {:<8}  {:<16}",
        "#", "Continuation Line", "Games", "Freq %", "Score (W/D/L)"
    );
    println!("--------------------------------------------------------------------------------");

    if res.lines.is_empty() {
        println!("  (No continuation lines found matching criteria)");
    } else {
        for (i, line) in res.lines.iter().enumerate() {
            let score_str = if line.games > 0 {
                format!(
                    "{:.1}% / {:.1}% / {:.1}%",
                    (line.white_wins as f64 / line.games as f64) * 100.0,
                    (line.draws as f64 / line.games as f64) * 100.0,
                    (line.black_wins as f64 / line.games as f64) * 100.0
                )
            } else {
                "-".to_string()
            };

            println!(
                "  {:<4}  {:<45}  {:<8}  {:<7.1}%  {:<16}",
                i + 1,
                line.formatted,
                line.games,
                line.percentage,
                score_str
            );
        }
    }
    println!("================================================================================\n");
}
