use crate::cli::formatters::truncate_str;
use crate::db::ScidDatabaseWrapper;
use crate::pgn_db::PgnDatabaseWrapper;
use crate::position_search;
use anyhow::Result;
use std::path::Path;

pub fn handle_search(
    db_path: &Path,
    query: &str,
    limit: usize,
    start_game: Option<usize>,
    end_game: Option<usize>,
    json: bool,
    count_only: bool,
) -> Result<()> {
    let parsed_query = match crate::search::parser::QueryParser::parse_str(query) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("Query Parse Error: {}", e);
            if let Some(help) = &e.help {
                eprintln!("Help: {}", help);
            }
            std::process::exit(1);
        }
    };

    let path_str = db_path.to_string_lossy().to_lowercase();
    let start = std::time::Instant::now();

    let (matches, total_count, summaries) = if path_str.ends_with(".pgn") {
        let pgn_db = PgnDatabaseWrapper::open(db_path)?;
        let total = pgn_db.game_count();
        let matches = match (start_game, end_game) {
            (Some(s), Some(e)) => {
                pgn_db.search_query_range_with_progress(&parsed_query, s, e, |_, _, _| {})
            }
            (Some(s), None) => {
                pgn_db.search_query_range_with_progress(&parsed_query, s, total, |_, _, _| {})
            }
            (None, Some(e)) => {
                pgn_db.search_query_range_with_progress(&parsed_query, 0, e, |_, _, _| {})
            }
            (None, None) => pgn_db.search_query_with_progress(&parsed_query, |_, _, _| {}),
        };
        let mut summs = std::collections::HashMap::new();
        for m in matches.iter().take(limit) {
            let g = pgn_db.get_summary(m.game_id);
            summs.insert(m.game_id, (g.white, g.black, g.result, g.date));
        }
        (matches, total, summs)
    } else {
        let db = ScidDatabaseWrapper::open(db_path)?;
        let total = db.game_count();
        let matches = match (start_game, end_game) {
            (Some(s), Some(e)) => {
                db.search_query_range_with_progress(&parsed_query, s, e, |_, _, _| {})
            }
            (Some(s), None) => {
                db.search_query_range_with_progress(&parsed_query, s, total, |_, _, _| {})
            }
            (None, Some(e)) => {
                db.search_query_range_with_progress(&parsed_query, 0, e, |_, _, _| {})
            }
            (None, None) => db.search_query_with_progress(&parsed_query, |_, _, _| {}),
        };
        let mut summs = std::collections::HashMap::new();
        for m in matches.iter().take(limit) {
            if let Some(g) = db.get_game_summary(m.game_id) {
                summs.insert(m.game_id, (g.white, g.black, g.result, g.date));
            }
        }
        (matches, total, summs)
    };

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    if count_only {
        println!("{}", matches.len());
        return Ok(());
    }

    if json {
        let out_matches: Vec<serde_json::Value> = matches
            .iter()
            .take(limit)
            .map(|m| {
                let (w, b, r, d) = summaries.get(&m.game_id).cloned().unwrap_or_default();
                serde_json::json!({
                    "game_id": m.game_id,
                    "white": w,
                    "black": b,
                    "result": r,
                    "date": d,
                    "match_count": m.match_details.match_count,
                    "matching_plies": m.match_details.matching_plies,
                })
            })
            .collect();

        let output = serde_json::json!({
            "total_searched": total_count,
            "matched_count": matches.len(),
            "duration_ms": elapsed_ms,
            "matches": out_matches,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!(
        "Search completed in {:.2} ms across {} games:",
        elapsed_ms, total_count
    );
    println!("Query: {}\n", query);
    println!("Found {} matching games.\n", matches.len());

    println!(
        "{:<6} | {:<20} | {:<20} | {:<7} | {:<10} | {:<15}",
        "ID", "White", "Black", "Result", "Date", "Plies"
    );
    println!(
        "{:-<6}-+-{:-<20}-+-{:-<20}-+-{:-<7}-+-{:-<10}-+-{:-<15}",
        "", "", "", "", "", ""
    );

    for m in matches.iter().take(limit) {
        if let Some((w, b, r, d)) = summaries.get(&m.game_id) {
            let plies_str = if m.match_details.matching_plies.is_empty() {
                "-".to_string()
            } else if m.match_details.matching_plies.len() <= 4 {
                format!("{:?}", m.match_details.matching_plies)
            } else {
                format!("{:?}...", &m.match_details.matching_plies[..4])
            };
            println!(
                "{:<6} | {:<20} | {:<20} | {:<7} | {:<10} | {:<15}",
                m.game_id,
                truncate_str(w, 20),
                truncate_str(b, 20),
                r,
                d,
                truncate_str(&plies_str, 15)
            );
        }
    }

    if matches.len() > limit {
        println!("... (showing first {} of {} matches)", limit, matches.len());
    }

    Ok(())
}

pub fn handle_explain(query: &str, json: bool) -> Result<()> {
    let explanation = match crate::search::QueryParser::explain(query) {
        Ok(exp) => exp,
        Err(e) => {
            eprintln!("Query Parse Error: {}", e);
            if let Some(help) = &e.help {
                eprintln!("Help: {}", help);
            }
            std::process::exit(1);
        }
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&explanation)?);
        return Ok(());
    }

    println!("=== CQL Query Explanation ===");
    println!("Original Query:  {}", explanation.original_query);
    println!("Canonical DSL:   {}", explanation.canonical_dsl);
    println!("Header-Only:     {}", explanation.is_header_only);
    println!("Has Symmetries:  {}", explanation.has_symmetries);
    println!(
        "\nExpanded Search Branches ({}):",
        explanation.branches.len()
    );

    for (i, branch) in explanation.branches.iter().enumerate() {
        println!("\n  [Branch {}] Symmetry: {}", i + 1, branch.symmetry_name);
        println!("  DSL:      {}", branch.dsl);
        if !branch.transformed_fens.is_empty() {
            println!("  Transformed FENs:");
            for fen in &branch.transformed_fens {
                println!("    -> {}", fen);
            }
        }
    }
    Ok(())
}

pub fn handle_search_pos(db_path: &Path, fen: &str, max_ply: usize) -> Result<()> {
    let path_str = db_path.to_string_lossy().to_lowercase();
    let (result, summaries) = if path_str.ends_with(".pgn") {
        let pgn_db = PgnDatabaseWrapper::open(db_path)?;
        let res = pgn_db.search_position(fen, None, None, Some(max_ply), |_, _, _| {})?;
        let mut summs = std::collections::HashMap::new();
        for m in res.matches.iter().take(50) {
            let g = pgn_db.get_summary(m.game_id);
            summs.insert(m.game_id, (g.white, g.black, g.result, g.date));
        }
        (res, summs)
    } else {
        let db = ScidDatabaseWrapper::open(db_path)?;
        let res = db.search_position(fen, None, None, Some(max_ply))?;
        let mut summs = std::collections::HashMap::new();
        for m in res.matches.iter().take(50) {
            if let Some(g) = db.get_game_summary(m.game_id) {
                summs.insert(m.game_id, (g.white, g.black, g.result, g.date));
            }
        }
        (res, summs)
    };

    println!(
        "Position search completed in {:.2} ms across {} games:",
        result.elapsed_ms, result.total_games_searched
    );
    println!("Found {} matching games.\n", result.matches.len());

    println!(
        "{:<6} | {:<5} | {:<20} | {:<20} | {:<7} | {:<10}",
        "ID", "Ply", "White", "Black", "Result", "Date"
    );
    println!(
        "{:-<6}-+-{:-<5}-+-{:-<20}-+-{:-<20}-+-{:-<7}-+-{:-<10}",
        "", "", "", "", "", ""
    );

    for m in result.matches.iter().take(50) {
        if let Some((w, b, r, d)) = summaries.get(&m.game_id) {
            println!(
                "{:<6} | {:<5} | {:<20} | {:<20} | {:<7} | {:<10}",
                m.game_id,
                m.ply,
                truncate_str(w, 20),
                truncate_str(b, 20),
                r,
                d
            );
        }
    }
    if result.matches.len() > 50 {
        println!("... (showing first 50 of {} matches)", result.matches.len());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handle_search_mat(
    db_path: &Path,
    wq: Option<u8>,
    wr: Option<u8>,
    wb: Option<u8>,
    wn: Option<u8>,
    wp: Option<u8>,
    bq: Option<u8>,
    br: Option<u8>,
    bb: Option<u8>,
    bn: Option<u8>,
    bp: Option<u8>,
    any_move: bool,
    opposite_bishops: bool,
    same_bishops: bool,
) -> Result<()> {
    let mat_filter = position_search::MaterialFilter {
        white_queens: wq,
        white_rooks: wr,
        white_bishops: wb,
        white_knights: wn,
        white_pawns: wp,
        black_queens: bq,
        black_rooks: br,
        black_bishops: bb,
        black_knights: bn,
        black_pawns: bp,
        opposite_bishops: if opposite_bishops { Some(true) } else { None },
        same_bishops: if same_bishops { Some(true) } else { None },
        match_any_ply: any_move,
        max_ply: None,
    };

    let path_str = db_path.to_string_lossy().to_lowercase();
    let start = std::time::Instant::now();

    let (matches, total_count, summaries) = if path_str.ends_with(".pgn") {
        let pgn_db = PgnDatabaseWrapper::open(db_path)?;
        let matches = pgn_db.search_material(&mat_filter, |_, _, _| {})?;
        let mut summs = std::collections::HashMap::new();
        for &gid in matches.iter().take(50) {
            let g = pgn_db.get_summary(gid);
            summs.insert(gid, (g.white, g.black, g.result, g.date));
        }
        (matches, pgn_db.game_count(), summs)
    } else {
        let db = ScidDatabaseWrapper::open(db_path)?;
        let matches = db.search_material(&mat_filter)?;
        let mut summs = std::collections::HashMap::new();
        for &gid in matches.iter().take(50) {
            if let Some(g) = db.get_game_summary(gid) {
                summs.insert(gid, (g.white, g.black, g.result, g.date));
            }
        }
        (matches, db.game_count(), summs)
    };
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    println!(
        "Material search completed in {:.2} ms across {} games (mode: {}):",
        elapsed_ms,
        total_count,
        if any_move {
            "any move"
        } else {
            "final position"
        }
    );
    println!("Found {} matching games.\n", matches.len());

    println!(
        "{:<6} | {:<20} | {:<20} | {:<7} | {:<10}",
        "ID", "White", "Black", "Result", "Date"
    );
    println!(
        "{:-<6}-+-{:-<20}-+-{:-<20}-+-{:-<7}-+-{:-<10}",
        "", "", "", "", ""
    );

    for &game_id in matches.iter().take(50) {
        if let Some((w, b, r, d)) = summaries.get(&game_id) {
            println!(
                "{:<6} | {:<20} | {:<20} | {:<7} | {:<10}",
                game_id,
                truncate_str(w, 20),
                truncate_str(b, 20),
                r,
                d
            );
        }
    }
    if matches.len() > 50 {
        println!("... (showing first 50 of {} matches)", matches.len());
    }
    Ok(())
}

pub fn handle_get(db_path: &Path, index: usize) -> Result<()> {
    let path_str = db_path.to_string_lossy().to_lowercase();
    let pgn = if path_str.ends_with(".pgn") {
        let pgn_db = PgnDatabaseWrapper::open(db_path)?;
        pgn_db.get_game_pgn(index)?
    } else {
        let db = ScidDatabaseWrapper::open(db_path)?;
        db.game_pgn(index)?
    };
    println!("{}", pgn);
    Ok(())
}
