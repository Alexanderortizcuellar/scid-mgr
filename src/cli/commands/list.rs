use crate::cli::formatters::truncate_str;
use crate::db::{GameFilter, ScidDatabaseWrapper};
use crate::pgn_db::PgnDatabaseWrapper;
use anyhow::Result;
use std::path::Path;

pub fn handle_list(
    db_path: &Path,
    page: usize,
    page_size: usize,
    player: Option<String>,
    eco: Option<String>,
    sort_by: Option<String>,
    desc: bool,
) -> Result<()> {
    let filter = GameFilter {
        player,
        eco,
        sort_by,
        sort_asc: Some(!desc),
        ..Default::default()
    };
    let path_str = db_path.to_string_lossy().to_lowercase();
    let (games, total) = if path_str.ends_with(".pgn") {
        let pgn = PgnDatabaseWrapper::open(db_path)?;
        pgn.query_games(&filter, page, page_size)
    } else {
        let db = ScidDatabaseWrapper::open(db_path)?;
        db.query_games(&filter, page, page_size)
    };
    println!(
        "Displaying games {}-{} of {} total matching:\n",
        page * page_size,
        usize::min((page + 1) * page_size, total),
        total
    );

    println!(
        "{:<6} | {:<20} | {:<20} | {:<7} | {:<5} | {:<10} | {:<15}",
        "ID", "White", "Black", "Result", "ECO", "Date", "Event"
    );
    println!(
        "{:-<6}-+-{:-<20}-+-{:-<20}-+-{:-<7}-+-{:-<5}-+-{:-<10}-+-{:-<15}",
        "", "", "", "", "", "", ""
    );

    for g in games {
        let del_mark = if g.deleted { "[DEL] " } else { "" };
        println!(
            "{:<6} | {:<20} | {:<20} | {:<7} | {:<5} | {:<10} | {}{:<15}",
            g.id,
            truncate_str(&g.white, 20),
            truncate_str(&g.black, 20),
            g.result,
            g.eco,
            g.date,
            del_mark,
            truncate_str(&g.event, 15)
        );
    }
    Ok(())
}
