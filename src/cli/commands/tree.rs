use crate::db::ScidDatabaseWrapper;
use crate::pgn_db::PgnDatabaseWrapper;
use crate::tree_index::TreeIndex;
use anyhow::Result;
use std::path::Path;

pub fn handle_build_tree(
    db_path: &Path,
    max_ply: usize,
    min_games: usize,
    threads: Option<usize>,
) -> Result<()> {
    let start = std::time::Instant::now();
    let path_str = db_path.to_string_lossy();
    let min_games_opt = if min_games > 1 { Some(min_games) } else { None };
    let idx = if path_str.ends_with(".pgn") {
        let pgn_db = PgnDatabaseWrapper::open(db_path)?;
        TreeIndex::build_for_pgn(
            db_path,
            &pgn_db.entries,
            pgn_db.mmap_ref(),
            max_ply,
            None,
            min_games_opt,
            threads,
            |scanned, total, positions| {
                print!(
                    "\r  Indexing tree stats: {} / {} ({:.1}%) | Unique positions: {}",
                    scanned,
                    total,
                    (scanned as f64 / total as f64) * 100.0,
                    positions
                );
                let _ = std::io::Write::flush(&mut std::io::stdout());
            },
        )?
    } else {
        let db = ScidDatabaseWrapper::open(db_path)?;
        let games_path = db.games_path().to_path_buf();
        let entries = db.entries();
        let db_path_buf = db.index_path().to_path_buf();
        TreeIndex::build_for_scid(
            &db_path_buf,
            entries,
            &games_path,
            max_ply,
            None,
            min_games_opt,
            threads,
            |scanned, total, positions| {
                print!(
                    "\r  Indexing tree stats: {} / {} ({:.1}%) | Unique positions: {}",
                    scanned,
                    total,
                    (scanned as f64 / total as f64) * 100.0,
                    positions
                );
                let _ = std::io::Write::flush(&mut std::io::stdout());
            },
        )?
    };
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "\n[OK] Built Opening Tree Index {} in {:.2} ms ({} unique positions).",
        idx.path.display(),
        elapsed_ms,
        idx.header.unique_positions
    );
    Ok(())
}

pub fn handle_tree(
    db_path: &Path,
    fen: Option<String>,
    _sample_games: usize,
    _all_game_ids: bool,
) -> Result<()> {
    let fen_str = fen.as_deref().unwrap_or("");
    let mut tree_report = TreeIndex::load(db_path)
        .ok()
        .and_then(|idx| idx.query_tree(fen_str));

    if tree_report.is_none() {
        let lower = db_path.to_string_lossy().to_lowercase();
        if lower.ends_with(".pgn") {
            if let Ok(pgn_db) = PgnDatabaseWrapper::open(db_path) {
                tree_report = TreeIndex::calculate_tree_for_pgn(
                    &pgn_db.entries,
                    pgn_db.mmap_ref(),
                    fen_str,
                    None,
                    Some(500),
                );
            }
        } else if lower.ends_with(".si5")
            || lower.ends_with(".si4")
            || lower.ends_with(".sg5")
            || lower.ends_with(".sg4")
            || lower.ends_with(".sn5")
            || lower.ends_with(".sn4")
        {
            if let Ok(scid_db) = ScidDatabaseWrapper::open(db_path) {
                tree_report = TreeIndex::calculate_tree_for_scid(
                    scid_db.entries(),
                    scid_db.games_path(),
                    fen_str,
                    None,
                    Some(500),
                );
            }
        }
    }

    if let Some(tree) = tree_report {
        println!(
            "Opening Tree for position (Total Games: {} | +{:.1}% / ={:.1}% / -{:.1}%):",
            tree.total_games, tree.white_pct, tree.draw_pct, tree.black_pct
        );
        println!(
            "{:<6} | {:<8} | {:<10} | {:<7} | {:<7} | {:<7} | {:<8}",
            "Move", "UCI", "Games", "1-0 %", "1/2 %", "0-1 %", "Avg Elo"
        );
        println!(
            "{:-<6}-+-{:-<8}-+-{:-<10}-+-{:-<7}-+-{:-<7}-+-{:-<7}-+-{:-<8}",
            "", "", "", "", "", "", ""
        );
        for m in tree.moves {
            let avg_elo_str = match (m.avg_white_elo, m.avg_black_elo) {
                (Some(w), Some(b)) => format!("{}/{}", w, b),
                (Some(w), None) => format!("{}/-", w),
                _ => "-".to_string(),
            };
            println!(
                "{:<6} | {:<8} | {:<10} | {:<6.1}% | {:<6.1}% | {:<6.1}% | {:<8}",
                m.san, m.uci, m.total_games, m.white_pct, m.draw_pct, m.black_pct, avg_elo_str
            );
        }
    } else {
        println!("No games found reaching this position.");
    }
    Ok(())
}
