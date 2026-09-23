use crate::db::{ScidDatabaseWrapper, ScidFormat};
use crate::pgn_utils;
use anyhow::{Context, Result};
use std::path::Path;

pub fn handle_import(db_path: &Path, pgn_path: &Path, format: &str) -> Result<()> {
    let fmt = if format.eq_ignore_ascii_case("si4") {
        ScidFormat::Si4
    } else {
        ScidFormat::Si5
    };

    let mut db = if db_path.exists() {
        ScidDatabaseWrapper::open(db_path)?
    } else {
        println!("Creating new {} database at {}...", fmt, db_path.display());
        ScidDatabaseWrapper::create(db_path, fmt)?
    };

    println!("Importing games from {}...", pgn_path.display());
    let (imported, errors) = pgn_utils::import_pgn_file_with_progress(&mut db, pgn_path, |prog| {
        let mb_processed = prog.processed_bytes as f64 / (1024.0 * 1024.0);
        let mb_total = prog.total_bytes as f64 / (1024.0 * 1024.0);
        print!(
            "\r[Progress: {:>5.1}%] ({:>6.1}/{:>6.1} MB) | Games: {:>7} | Speed: {:>6.0} games/s | ETA: {:>3}s   ",
            prog.percent,
            mb_processed,
            mb_total,
            prog.imported_games,
            prog.speed_gps,
            prog.eta_seconds
        );
        use std::io::Write;
        let _ = std::io::stdout().flush();
    })?;
    println!();
    println!(
        "Successfully imported {} games ({} errors). Total games: {}",
        imported,
        errors,
        db.game_count()
    );
    Ok(())
}

pub fn handle_export(db_path: &Path, output_pgn: &Path) -> Result<()> {
    use std::io::Write;
    let db = ScidDatabaseWrapper::open(db_path)?;
    println!(
        "Exporting {} games to {}...",
        db.game_count(),
        output_pgn.display()
    );
    let count = pgn_utils::export_pgn_ultra_fast(&db, output_pgn, |p| {
        print!(
            "\r[Export: {:>5.1}%] | Games: {:>8} / {:>8} | Speed: {:>7.0} games/s | ETA: {:>3}s   ",
            p.percent, p.exported_games, p.total_games, p.speed_gps, p.eta_seconds
        );
        let _ = std::io::stdout().flush();
    })?;
    println!(
        "\nSuccessfully exported {} games to {}.",
        count,
        output_pgn.display()
    );
    Ok(())
}

pub fn handle_create(db_path: &Path, format: &str) -> Result<()> {
    let fmt = if format.eq_ignore_ascii_case("si4") {
        ScidFormat::Si4
    } else {
        ScidFormat::Si5
    };
    let mut db = ScidDatabaseWrapper::create(db_path, fmt)?;
    db.save().context("Saving empty database")?;
    println!(
        "Created new empty {} database at {}",
        fmt,
        db.index_path().display()
    );
    Ok(())
}
