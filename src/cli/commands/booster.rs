use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::db::ScidDatabaseWrapper;
use crate::search_booster::BoostIndexBuilder;

pub fn handle_build_booster(db_path: &Path, output_path: Option<PathBuf>) -> Result<()> {
    let path_str = db_path.to_string_lossy();
    let is_pgn = path_str.to_lowercase().ends_with(".pgn");

    let progress_cb = Arc::new(|scanned: usize, total: usize, _| {
        print!(
            "\r  Building 16-bit Search Booster: {} / {} games ({:.1}%)",
            scanned,
            total,
            (scanned as f64 / total.max(1) as f64) * 100.0
        );
        let _ = std::io::Write::flush(&mut std::io::stdout());
    });

    let (out_file, total_games, total_plies, elapsed_ms) = if is_pgn {
        BoostIndexBuilder::build_for_pgn(db_path, output_path, Some(progress_cb))?
    } else {
        let scid_db = ScidDatabaseWrapper::open(db_path)?;
        BoostIndexBuilder::build_for_scid(&scid_db, output_path, Some(progress_cb))?
    };

    println!();
    println!(
        "[OK] Built Search Booster ({:?}) for {} games ({} plies) in {:.2} ms.",
        out_file, total_games, total_plies, elapsed_ms as f64
    );

    Ok(())
}
