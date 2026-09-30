use crate::db::ScidDatabaseWrapper;
use crate::pgn_db::PgnDatabaseWrapper;
use anyhow::Result;
use std::path::Path;

pub fn handle_info(db_path: &Path) -> Result<()> {
    let path_str = db_path.to_string_lossy().to_lowercase();
    if path_str.ends_with(".pgn") {
        let pgn = PgnDatabaseWrapper::open(db_path)?;
        println!("File:        {}", pgn.pgn_path.display());
        println!("Format:      PGN (Plain Text Database)");
        println!("Total Games: {}", pgn.game_count());
    } else {
        let db = ScidDatabaseWrapper::open(db_path)?;
        let stats = db.stats();
        println!("Database: {}", stats.index_path);
        println!("Format:   {}", stats.format);
        println!("Total Games:    {}", stats.total_games);
        println!("Active Games:   {}", stats.active_games);
        println!("Deleted Games:  {}", stats.deleted_games);
        println!("Unique Players: {}", stats.players_count);
        println!("Unique Events:  {}", stats.events_count);
        println!("Unique Sites:   {}", stats.sites_count);
        println!("Unique Rounds:  {}", stats.rounds_count);
        println!("Index Size:     {} bytes", stats.index_file_size);
        println!("Names Size:     {} bytes", stats.namebase_file_size);
        println!("Games Size:     {} bytes", stats.games_file_size);
    }
    Ok(())
}
