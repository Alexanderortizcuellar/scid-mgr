use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

use crate::db::ScidDatabaseWrapper;
use crate::endgame_index::{
    resolve_companion_feat_path, EndgameCatalog, EndgameIndexBuilder, EndgamePopularityReport,
    EndgameQueryEngine, FeatureQueryReport, MmapFeatureIndex,
};
use crate::pgn_db::PgnDatabaseWrapper;
use crate::position_index::PositionIndex;
use shakmaty::fen::Fen;
use shakmaty::zobrist::ZobristHash;
use shakmaty::{CastlingMode, Chess};

pub fn handle_build_endgames(db_path: &Path, catalog_path: Option<&Path>) -> Result<()> {
    let path_str = db_path.to_string_lossy().to_lowercase();
    let catalog = EndgameCatalog::load_or_default(catalog_path);
    let builder = EndgameIndexBuilder::with_catalog(catalog);

    let (out_path, total_games, elapsed_ms) = if path_str.ends_with(".pgn") {
        let progress_cb = Arc::new(|done, total, pct| {
            print!(
                "\r  Indexing endgame features: {} / {} ({:.1}%)",
                done, total, pct
            );
            let _ = std::io::Write::flush(&mut std::io::stdout());
        });
        let res = builder.build_for_pgn(db_path, None, Some(progress_cb))?;
        println!();
        res
    } else {
        let scid_db = ScidDatabaseWrapper::open(db_path)
            .with_context(|| format!("Opening SCID database {:?}", db_path))?;
        let progress_cb = Arc::new(|done, total, pct| {
            print!(
                "\r  Indexing endgame features: {} / {} ({:.1}%)",
                done, total, pct
            );
            let _ = std::io::Write::flush(&mut std::io::stdout());
        });
        let res = builder.build_for_scid(&scid_db, None, Some(progress_cb))?;
        println!();
        res
    };

    println!(
        "[OK] Built Endgame Feature Index {} in {:.2} ms ({} games).",
        out_path.display(),
        elapsed_ms as f64,
        total_games
    );
    Ok(())
}

pub fn handle_endgames(
    db_path: &Path,
    fen: Option<String>,
    category: Option<String>,
    feature: Option<String>,
    json_output: bool,
    limit: usize,
) -> Result<()> {
    let catalog = EndgameCatalog::default_catalog();
    let feat_path = resolve_companion_feat_path(db_path);

    // Auto-build companion index if missing
    if !feat_path.exists() {
        if !json_output {
            println!(
                "[INFO] Companion feature index not found at {}. Building now...",
                feat_path.display()
            );
        }
        handle_build_endgames(db_path, None)?;
    }

    let mmap_idx = MmapFeatureIndex::open(&feat_path)
        .with_context(|| format!("Failed to open feature index at {:?}", feat_path))?;

    // 1. Feature Query Mode
    if let Some(feat_id) = feature {
        let report = EndgameQueryEngine::query_feature(&mmap_idx, &catalog, &feat_id, limit)?;
        if json_output {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            print_feature_report(&report);
        }
        return Ok(());
    }

    // 2. Position Filter Candidate Resolution
    let mut candidate_ids: Option<Vec<u32>> = None;
    if let Some(ref fen_str) = fen {
        let fen_parsed: Fen = fen_str
            .parse()
            .with_context(|| format!("Invalid FEN: {}", fen_str))?;
        let pos: Chess = fen_parsed
            .into_position(CastlingMode::Chess960)
            .map_err(|e| anyhow::anyhow!("Invalid position from FEN: {:?}", e))?;

        if let Ok(pos_idx) = PositionIndex::load(db_path) {
            let hash_val: shakmaty::zobrist::Zobrist64 =
                pos.zobrist_hash(shakmaty::EnPassantMode::Legal);
            if let Some(postings) = pos_idx.get_all_position_games(hash_val.0) {
                candidate_ids = Some(postings.into_iter().map(|id| id as u32).collect());
            } else {
                candidate_ids = Some(Vec::new());
            }
        }
    }

    // 3. Game Result Resolver Closure
    let path_str = db_path.to_string_lossy().to_lowercase();
    let (scid_db_opt, pgn_db_opt) = if path_str.ends_with(".pgn") {
        (None, PgnDatabaseWrapper::open(db_path).ok())
    } else {
        (ScidDatabaseWrapper::open(db_path).ok(), None)
    };

    let get_result = move |gid: u32| -> u8 {
        if let Some(ref scid) = scid_db_opt {
            let entries = scid.entries();
            if let Some(e) = entries.get(gid as usize) {
                return e.result;
            }
        } else if let Some(ref pgn) = pgn_db_opt {
            if let Some(e) = pgn.entries.get(gid as usize) {
                return e.result;
            }
        }
        0
    };

    let mut report = EndgameQueryEngine::calculate_popularity(
        &mmap_idx,
        &catalog,
        &db_path.to_string_lossy(),
        candidate_ids.as_deref(),
        fen.as_deref(),
        Some(&get_result),
    )?;

    // Filter by category if requested
    if let Some(ref cat_filter) = category {
        let cat_upper = cat_filter.to_uppercase();
        report
            .categories
            .retain(|c| c.category_id.to_uppercase() == cat_upper);
        report
            .features
            .retain(|f| f.category_id.to_uppercase() == cat_upper);
    }

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_popularity_report(&report);
    }

    Ok(())
}

fn print_feature_report(rep: &FeatureQueryReport) {
    println!("\n================================================================================");
    println!("♟️  ENDGAME FEATURE QUERY: {}", rep.name);
    println!("================================================================================");
    println!("Feature ID:     {}", rep.feature_id);
    println!("Short Name:     {}", rep.short_name);
    if let Some(ref gbr) = rep.gbr_code {
        println!("GBR Code:       {}", gbr);
    }
    println!("Category:       {}", rep.category_id);
    println!("Bit Position:   {}", rep.bit);
    println!(
        "Matching Games: {} / {} ({:.2}%)",
        rep.matching_games_count, rep.total_db_games, rep.percentage
    );

    if !rep.sample_game_ids.is_empty() {
        println!("\nSample Game IDs:");
        let ids_str = rep
            .sample_game_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        println!("  {}", ids_str);
    }
    println!("================================================================================\n");
}

fn print_popularity_report(rep: &EndgamePopularityReport) {
    println!("\n================================================================================");
    if rep.position_filtered {
        println!(
            "♟️  ENDGAME POPULARITY REPORT (Position Filtered: {} games)",
            rep.games_reaching_position
        );
        if let Some(ref fen) = rep.fen {
            println!("FEN: {}", fen);
        }
    } else {
        println!(
            "♟️  DATABASE ENDGAME POPULARITY REPORT ({} games)",
            rep.total_db_games
        );
    }
    println!("================================================================================");

    println!("\n--- CATEGORY BREAKDOWN ---");
    println!(
        "{:<20} {:>10} {:>8}  Distribution",
        "Category", "Games", "Share"
    );
    println!("{:-<70}", "");

    for cat in &rep.categories {
        let bar_len = ((cat.percentage / 100.0) * 25.0).round() as usize;
        let bar = "█".repeat(bar_len.min(25));
        println!(
            "{:<20} {:>10} {:>7.1}%  {}",
            cat.name, cat.total_games, cat.percentage, bar
        );
    }

    println!("\n--- TOP ENDGAME FEATURES ---");
    println!(
        "{:<22} {:<24} {:>8} {:>7} {:>7} {:>7} {:>7}",
        "Feature ID", "Name", "Games", "Share", "1-0", "1/2", "0-1"
    );
    println!("{:-<85}", "");

    for feat in &rep.features {
        if feat.game_count == 0 {
            continue;
        }
        let denom = feat.game_count.max(1) as f64;
        let w_pct = (feat.white_wins as f64 / denom) * 100.0;
        let d_pct = (feat.draws as f64 / denom) * 100.0;
        let b_pct = (feat.black_wins as f64 / denom) * 100.0;

        let name_trunc = if feat.short_name.len() > 23 {
            format!("{}...", &feat.short_name[..20])
        } else {
            feat.short_name.clone()
        };

        println!(
            "{:<22} {:<24} {:>8} {:>6.1}% {:>6.1}% {:>6.1}% {:>6.1}%",
            feat.id, name_trunc, feat.game_count, feat.percentage, w_pct, d_pct, b_pct
        );
    }
    println!("================================================================================\n");
}
