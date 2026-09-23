use crate::db::ScidDatabaseWrapper;
use crate::pgn_db;
use anyhow::Result;
use std::path::Path;

pub fn handle_sort_pgn(
    input_pgn: &Path,
    output_pgn: &Path,
    sort_by: &str,
    desc: bool,
) -> Result<()> {
    println!(
        "Sorting PGN {} by {} ({})...",
        input_pgn.display(),
        sort_by,
        if desc { "descending" } else { "ascending" }
    );
    let start = std::time::Instant::now();
    let count = pgn_db::sort_pgn_file(input_pgn, output_pgn, Some(sort_by), !desc)?;
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "Successfully sorted {} games into {} in {:.2} ms.",
        count,
        output_pgn.display(),
        elapsed_ms
    );
    Ok(())
}

pub fn handle_sort_db(
    db_path: &Path,
    output_path: Option<&Path>,
    sort_by: &str,
    desc: bool,
    keep_deleted: bool,
) -> Result<()> {
    let delete_removed = !keep_deleted;
    let start = std::time::Instant::now();
    let mut db = ScidDatabaseWrapper::open(db_path)?;
    let count = if let Some(out_p) = output_path {
        println!(
            "Sorting SCID DB {} by {} to new database {}...",
            db_path.display(),
            sort_by,
            out_p.display()
        );
        db.sort_database_to(out_p, sort_by, !desc, delete_removed)?
    } else {
        println!(
            "Sorting SCID DB {} in-place by {}...",
            db_path.display(),
            sort_by
        );
        db.sort_database(sort_by, !desc, delete_removed)?
    };
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "Successfully sorted {} games in {:.2} ms.",
        count, elapsed_ms
    );
    Ok(())
}
