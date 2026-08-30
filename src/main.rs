mod db;
pub mod pgn_utils;
pub mod position_search;
mod server;
mod test_suite;
pub mod zero_copy_ingest;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use db::{GameFilter, ScidDatabaseWrapper, ScidFormat};
use pgn_utils::import_pgn_file_with_progress;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "scid-mgr")]
#[command(about = "SCID (si4 / si5) Chess Database Manager & Interactive Server")]
struct Cli {
    /// Run as interactive JSON-RPC server on stdin/stdout
    #[arg(short, long)]
    interactive: bool,

    /// Optional database path to auto-open in interactive mode
    #[arg(value_name = "DB_PATH")]
    db_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the test suite validating chess-scid-rw integration with SI4 and SI5
    Test,

    /// Display summary and statistics of a SCID database
    Info {
        /// Path to .si4 or .si5 file
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,
    },

    /// List game headers / index entries from a SCID database
    List {
        /// Path to .si4 or .si5 file
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Page number (0-based)
        #[arg(long, default_value = "0")]
        page: usize,

        /// Number of games per page
        #[arg(long, default_value = "20")]
        page_size: usize,

        /// Filter by player name
        #[arg(long)]
        player: Option<String>,

        /// Filter by ECO code prefix
        #[arg(long)]
        eco: Option<String>,

        /// Sort by field (date, white, black, white_elo, black_elo, eco, result, event, site, round, id)
        #[arg(long)]
        sort_by: Option<String>,

        /// Sort descending
        #[arg(long)]
        desc: bool,
    },

    /// Search for an exact board position by FEN across all games
    SearchPos {
        /// Path to .si4 or .si5 file
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// FEN string to search for
        #[arg(value_name = "FEN")]
        fen: String,

        /// Maximum ply depth to search in each game (default: 250)
        #[arg(long, default_value = "250")]
        max_ply: usize,
    },

    /// Search for games by piece material counts
    SearchMat {
        /// Path to .si4 or .si5 file
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// White Queens count
        #[arg(long)]
        wq: Option<u8>,

        /// White Rooks count
        #[arg(long)]
        wr: Option<u8>,

        /// White Bishops count
        #[arg(long)]
        wb: Option<u8>,

        /// White Knights count
        #[arg(long)]
        wn: Option<u8>,

        /// White Pawns count
        #[arg(long)]
        wp: Option<u8>,

        /// Black Queens count
        #[arg(long)]
        bq: Option<u8>,

        /// Black Rooks count
        #[arg(long)]
        br: Option<u8>,

        /// Black Bishops count
        #[arg(long)]
        bb: Option<u8>,

        /// Black Knights count
        #[arg(long)]
        bn: Option<u8>,

        /// Black Pawns count
        #[arg(long)]
        bp: Option<u8>,

        /// Match at any move (default: false, checks final position only)
        #[arg(long)]
        any_move: bool,

        /// Enforce opposite-colored bishops
        #[arg(long)]
        opposite_bishops: bool,

        /// Enforce same-colored bishops
        #[arg(long)]
        same_bishops: bool,
    },

    /// Extract and print reconstructed PGN for a specific game index
    Get {
        /// Path to .si4 or .si5 file
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// 0-based game index
        #[arg(value_name = "INDEX")]
        index: usize,
    },

    /// Import games from a PGN file into a SCID database
    Import {
        /// Target SCID database path (.si4 or .si5)
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Input PGN file path
        #[arg(value_name = "PGN_PATH")]
        pgn_path: PathBuf,

        /// Format if creating new database ("si4" or "si5")
        #[arg(long, default_value = "si5")]
        format: String,

        /// Optional path to official SCID binary for ultra-fast C++ ingestion (~5s for 300k games)
        #[arg(long)]
        scid_bin: Option<PathBuf>,
    },

    /// Export games from a SCID database into a PGN file
    Export {
        /// Source SCID database path
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Output PGN file path
        #[arg(value_name = "OUTPUT_PGN")]
        output_pgn: PathBuf,
    },

    /// Create a new empty SCID database
    Create {
        /// Destination database path
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Format: "si4" or "si5"
        #[arg(long, default_value = "si5")]
        format: String,
    },

    /// Run the interactive JSON-RPC server
    Interactive {
        /// Optional database path to auto-open
        #[arg(value_name = "DB_PATH")]
        db_path: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.interactive {
        return server::run_interactive_server(cli.db_path);
    }

    match cli.command {
        Some(Commands::Test) => {
            test_suite::run_full_test_suite()?;
        }
        Some(Commands::Info { db_path }) => {
            let db = ScidDatabaseWrapper::open(&db_path)?;
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
        Some(Commands::List {
            db_path,
            page,
            page_size,
            player,
            eco,
            sort_by,
            desc,
        }) => {
            let db = ScidDatabaseWrapper::open(&db_path)?;
            let filter = GameFilter {
                player,
                eco,
                sort_by,
                sort_asc: Some(!desc),
                ..Default::default()
            };
            let (games, total) = db.query_games(&filter, page, page_size);
            println!(
                "Displaying games {}-{} of {} total matching:\n",
                page * page_size,
                usize::min((page + 1) * page_size, total),
                total
            );

            println!("{:<6} | {:<20} | {:<20} | {:<7} | {:<5} | {:<10} | {:<15}",
                "ID", "White", "Black", "Result", "ECO", "Date", "Event");
            println!("{:-<6}-+-{:-<20}-+-{:-<20}-+-{:-<7}-+-{:-<5}-+-{:-<10}-+-{:-<15}",
                "", "", "", "", "", "", "");

            for g in games {
                let del_mark = if g.deleted { "[DEL] " } else { "" };
                println!("{:<6} | {:<20} | {:<20} | {:<7} | {:<5} | {:<10} | {}{:<15}",
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
        }
        Some(Commands::SearchPos {
            db_path,
            fen,
            max_ply,
        }) => {
            let db = ScidDatabaseWrapper::open(&db_path)?;
            let result = db.search_position(&fen, Some(max_ply))?;
            println!(
                "Position search completed in {:.2} ms across {} games:",
                result.elapsed_ms, result.total_games_searched
            );
            println!("Found {} matching games.\n", result.matches.len());

            println!("{:<6} | {:<5} | {:<20} | {:<20} | {:<7} | {:<10}",
                "ID", "Ply", "White", "Black", "Result", "Date");
            println!("{:-<6}-+-{:-<5}-+-{:-<20}-+-{:-<20}-+-{:-<7}-+-{:-<10}",
                "", "", "", "", "", "");

            for m in result.matches.iter().take(50) {
                if let Some(g) = db.get_game_summary(m.game_id) {
                    println!("{:<6} | {:<5} | {:<20} | {:<20} | {:<7} | {:<10}",
                        m.game_id,
                        m.ply,
                        truncate_str(&g.white, 20),
                        truncate_str(&g.black, 20),
                        g.result,
                        g.date
                    );
                }
            }
            if result.matches.len() > 50 {
                println!("... (showing first 50 of {} matches)", result.matches.len());
            }
        }
        Some(Commands::SearchMat {
            db_path,
            wq,
            wr,
            wb,
            wn,
            wp,
            bq,
            br,
            bb,
            bn,
            bp,
            any_move,
            opposite_bishops,
            same_bishops,
        }) => {
            let db = ScidDatabaseWrapper::open(&db_path)?;
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

            let start = std::time::Instant::now();
            let matches = db.search_material(&mat_filter)?;
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

            println!(
                "Material search completed in {:.2} ms across {} games (mode: {}):",
                elapsed_ms,
                db.game_count(),
                if any_move { "any move" } else { "final position" }
            );
            println!("Found {} matching games.\n", matches.len());

            println!("{:<6} | {:<20} | {:<20} | {:<7} | {:<10}",
                "ID", "White", "Black", "Result", "Date");
            println!("{:-<6}-+-{:-<20}-+-{:-<20}-+-{:-<7}-+-{:-<10}",
                "", "", "", "", "");

            for &game_id in matches.iter().take(50) {
                if let Some(g) = db.get_game_summary(game_id) {
                    println!("{:<6} | {:<20} | {:<20} | {:<7} | {:<10}",
                        game_id,
                        truncate_str(&g.white, 20),
                        truncate_str(&g.black, 20),
                        g.result,
                        g.date
                    );
                }
            }
            if matches.len() > 50 {
                println!("... (showing first 50 of {} matches)", matches.len());
            }
        }
        Some(Commands::Get { db_path, index }) => {
            let db = ScidDatabaseWrapper::open(&db_path)?;
            let pgn = db.game_pgn(index)?;
            println!("{}", pgn);
        }
        Some(Commands::Import {
            db_path,
            pgn_path,
            format,
            scid_bin,
        }) => {
            let fmt = if format.eq_ignore_ascii_case("si4") {
                ScidFormat::Si4
            } else {
                ScidFormat::Si5
            };

            let mut db = if db_path.exists() {
                ScidDatabaseWrapper::open(&db_path)?
            } else {
                println!("Creating new {} database at {}...", fmt, db_path.display());
                ScidDatabaseWrapper::create(&db_path, fmt)?
            };

            if let Some(ref scid_exe) = scid_bin {
                if scid_exe.exists() {
                    println!(
                        "Using SCID C++ engine ({}) for ultra-fast import...",
                        scid_exe.display()
                    );
                    let (imported, errors) =
                        pgn_utils::import_pgn_with_scid_cli(&mut db, &pgn_path, scid_exe)?;
                    println!(
                        "Successfully imported {} games ({} errors). Total games: {}",
                        imported,
                        errors,
                        db.game_count()
                    );
                    return Ok(());
                }
            }

            println!("Importing games from {}...", pgn_path.display());
            let (imported, errors) = import_pgn_file_with_progress(&mut db, &pgn_path, |prog| {
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
        }
        Some(Commands::Export {
            db_path,
            output_pgn,
        }) => {
            use std::io::Write;
            let db = ScidDatabaseWrapper::open(&db_path)?;
            println!("Exporting {} games to {}...", db.game_count(), output_pgn.display());
            let count = pgn_utils::export_pgn_ultra_fast(&db, &output_pgn, |p| {
                print!(
                    "\r[Export: {:>5.1}%] | Games: {:>8} / {:>8} | Speed: {:>7.0} games/s | ETA: {:>3}s   ",
                    p.percent,
                    p.exported_games,
                    p.total_games,
                    p.speed_gps,
                    p.eta_seconds
                );
                let _ = std::io::stdout().flush();
            })?;
            println!("\nSuccessfully exported {} games to {}.", count, output_pgn.display());
        }
        Some(Commands::Create { db_path, format }) => {
            let fmt = if format.eq_ignore_ascii_case("si4") {
                ScidFormat::Si4
            } else {
                ScidFormat::Si5
            };
            let mut db = ScidDatabaseWrapper::create(&db_path, fmt)?;
            db.save().context("Saving empty database")?;
            println!("Created new empty {} database at {}", fmt, db.index_path().display());
        }
        Some(Commands::Interactive { db_path }) => {
            server::run_interactive_server(db_path)?;
        }
        None => {
            // Default to interactive mode if a db path was provided, or print help
            if let Some(path) = cli.db_path {
                server::run_interactive_server(Some(path))?;
            } else {
                println!("Run 'scid-mgr --help' for CLI options or 'scid-mgr test' to test.");
            }
        }
    }

    Ok(())
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        s.chars().take(max_chars - 1).collect::<String>() + "…"
    } else {
        s.to_string()
    }
}
