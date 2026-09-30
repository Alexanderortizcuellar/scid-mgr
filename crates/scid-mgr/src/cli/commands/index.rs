use crate::db::ScidDatabaseWrapper;
use crate::pgn_db::PgnDatabaseWrapper;
use crate::position_index::PositionIndex;
use anyhow::Result;
use std::path::Path;

pub fn handle_build_pos_idx(
    db_path: &Path,
    max_ply: usize,
    max_games: usize,
    min_games: usize,
    threads: Option<usize>,
) -> Result<()> {
    let start = std::time::Instant::now();
    let path_str = db_path.to_string_lossy();
    let max_games_opt = if max_games > 0 { Some(max_games) } else { None };
    let min_games_opt = if min_games > 1 { Some(min_games) } else { None };
    let idx = if path_str.ends_with(".pgn") {
        let pgn_db = PgnDatabaseWrapper::open(db_path)?;
        PositionIndex::build_for_pgn(
            db_path,
            &pgn_db.entries,
            pgn_db.mmap_ref(),
            max_ply,
            max_games_opt,
            min_games_opt,
            threads,
            |scanned, total, positions| {
                print!(
                    "\r  Indexing games: {} / {} ({:.1}%) | Unique positions: {}",
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
        PositionIndex::build_for_scid(
            &db_path_buf,
            entries,
            &games_path,
            max_ply,
            max_games_opt,
            min_games_opt,
            threads,
            |scanned, total, positions| {
                print!(
                    "\r  Indexing games: {} / {} ({:.1}%) | Unique positions: {}",
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
        "\n[OK] Built {} in {:.2} ms ({} unique positions).",
        idx.path.display(),
        elapsed_ms,
        idx.header.unique_positions
    );
    Ok(())
}

pub fn handle_build_all(
    db_path: &Path,
    max_ply: usize,
    min_games: usize,
    threads: Option<usize>,
) -> Result<()> {
    println!(">>> 1/2 Building Position Index (.pos.idx)...");
    handle_build_pos_idx(db_path, max_ply, 0, min_games, threads)?;
    println!("\n>>> 2/2 Building Opening Tree Index (.tree.idx)...");
    crate::cli::commands::tree::handle_build_tree(db_path, max_ply, min_games, threads)?;
    println!(
        "\n[OK] Successfully built all companion indexes for {}",
        db_path.display()
    );
    Ok(())
}
