pub mod benchmark;
mod db;
pub mod pgn_db;
pub mod pgn_utils;
pub mod position_index;
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

    /// Max CPU worker threads for search and indexing
    #[arg(short, long)]
    threads: Option<usize>,

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

    /// Run comprehensive performance benchmarks (load, sort, filter, search, seek)
    Bench {
        /// Path to database file (.si5, .si4, or .pgn)
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Include heavy search operations (e.g. full position search on large databases)
        #[arg(long)]
        heavy: bool,
    },

    /// Build companion .pos.idx position index for ultra-fast searches and opening tree
    BuildPosIdx {
        /// Path to .si5, .si4, or .pgn database
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Maximum ply depth to index (default: 24, i.e. 12 full moves)
        #[arg(long, default_value = "24")]
        max_ply: usize,

        /// Number of worker threads (default: all available CPU cores)
        #[arg(long)]
        threads: Option<usize>,
    },

    /// Query the instant opening tree for any board position (FEN or starting board)
    Tree {
        /// Path to .si5, .si4, or .pgn database
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Optional FEN position (defaults to initial board)
        #[arg(long)]
        fen: Option<String>,
    },

    /// Run the interactive JSON-RPC server
    Interactive {
        /// Optional database path to auto-open
        #[arg(value_name = "DB_PATH")]
        db_path: Option<PathBuf>,

        /// Max CPU worker threads for search and indexing
        #[arg(short, long)]
        threads: Option<usize>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.interactive {
        return server::run_interactive_server(cli.db_path, cli.threads);
    }

    match cli.command {
        Some(Commands::Interactive { db_path, threads }) => {
            server::run_interactive_server(db_path, threads.or(cli.threads))?;
        }
        Some(Commands::Test) => {
            test_suite::run_full_test_suite()?;
        }
        Some(Commands::Info { db_path }) => {
            let path_str = db_path.to_string_lossy().to_lowercase();
            if path_str.ends_with(".pgn") {
                let pgn = pgn_db::PgnDatabaseWrapper::open(&db_path)?;
                println!("File:        {}", pgn.pgn_path.display());
                println!("Format:      PGN (Plain Text Database)");
                println!("Total Games: {}", pgn.game_count());
            } else {
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
            let filter = GameFilter {
                player,
                eco,
                sort_by,
                sort_asc: Some(!desc),
                ..Default::default()
            };
            let path_str = db_path.to_string_lossy().to_lowercase();
            let (games, total) = if path_str.ends_with(".pgn") {
                let pgn = pgn_db::PgnDatabaseWrapper::open(&db_path)?;
                pgn.query_games(&filter, page, page_size)
            } else {
                let db = ScidDatabaseWrapper::open(&db_path)?;
                db.query_games(&filter, page, page_size)
            };
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
            let path_str = db_path.to_string_lossy().to_lowercase();
            let (result, summaries) = if path_str.ends_with(".pgn") {
                let pgn_db = pgn_db::PgnDatabaseWrapper::open(&db_path)?;
                let res = pgn_db.search_position(&fen, Some(max_ply), |_, _, _| {})?;
                let mut summs = std::collections::HashMap::new();
                for m in res.matches.iter().take(50) {
                    if let Some(e) = pgn_db.entries.get(m.game_id) {
                        summs.insert(m.game_id, (e.white.clone(), e.black.clone(), e.result.clone(), e.date.clone()));
                    }
                }
                (res, summs)
            } else {
                let db = ScidDatabaseWrapper::open(&db_path)?;
                let res = db.search_position(&fen, Some(max_ply))?;
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

            println!("{:<6} | {:<5} | {:<20} | {:<20} | {:<7} | {:<10}",
                "ID", "Ply", "White", "Black", "Result", "Date");
            println!("{:-<6}-+-{:-<5}-+-{:-<20}-+-{:-<20}-+-{:-<7}-+-{:-<10}",
                "", "", "", "", "", "");

            for m in result.matches.iter().take(50) {
                if let Some((w, b, r, d)) = summaries.get(&m.game_id) {
                    println!("{:<6} | {:<5} | {:<20} | {:<20} | {:<7} | {:<10}",
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
                let pgn_db = pgn_db::PgnDatabaseWrapper::open(&db_path)?;
                let matches = pgn_db.search_material(&mat_filter, |_, _, _| {})?;
                let mut summs = std::collections::HashMap::new();
                for &gid in matches.iter().take(50) {
                    if let Some(e) = pgn_db.entries.get(gid) {
                        summs.insert(gid, (e.white.clone(), e.black.clone(), e.result.clone(), e.date.clone()));
                    }
                }
                (matches, pgn_db.game_count(), summs)
            } else {
                let db = ScidDatabaseWrapper::open(&db_path)?;
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
                if any_move { "any move" } else { "final position" }
            );
            println!("Found {} matching games.\n", matches.len());

            println!("{:<6} | {:<20} | {:<20} | {:<7} | {:<10}",
                "ID", "White", "Black", "Result", "Date");
            println!("{:-<6}-+-{:-<20}-+-{:-<20}-+-{:-<7}-+-{:-<10}",
                "", "", "", "", "");

            for &game_id in matches.iter().take(50) {
                if let Some((w, b, r, d)) = summaries.get(&game_id) {
                    println!("{:<6} | {:<20} | {:<20} | {:<7} | {:<10}",
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
        }
        Some(Commands::Get { db_path, index }) => {
            let path_str = db_path.to_string_lossy().to_lowercase();
            let pgn = if path_str.ends_with(".pgn") {
                let pgn_db = pgn_db::PgnDatabaseWrapper::open(&db_path)?;
                pgn_db.get_game_pgn(index)?
            } else {
                let db = ScidDatabaseWrapper::open(&db_path)?;
                db.game_pgn(index)?
            };
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
        Some(Commands::Bench { db_path, heavy }) => {
            println!("Running performance benchmarks on {}...", db_path.display());
            let report = benchmark::run_benchmark(&db_path, heavy)?;

            println!("\n==========================================================================================");
            println!("                      DATABASE PERFORMANCE BENCHMARK REPORT                              ");
            println!("==========================================================================================");
            println!("Database:     {}", report.db_path);
            println!("Format:       {}", report.format);
            println!("Total Games:  {}", report.total_games);
            if report.total_players > 0 {
                println!("Entities:     {} players, {} events, {} sites", report.total_players, report.total_events, report.total_sites);
            }
            println!("Disk Size:    {:.2} MB", report.file_size_mb);
            println!("------------------------------------------------------------------------------------------");
            println!("{:<22} | {:<42} | {:>10} | {:<20}", "Category", "Benchmark Operation", "Time (ms)", "Details / Speed");
            println!("{:-<22}-+-{:-<42}-+-{:-<10}-+-{:-<20}", "", "", "", "");

            for item in &report.results {
                println!("{:<22} | {:<42} | {:>10.2} | {:<20}", item.category, item.name, item.elapsed_ms, item.notes);
            }
            println!("==========================================================================================");
            println!("Overall Benchmark Duration: {:.2} ms ({:.2} s)\n", report.total_time_ms, report.total_time_ms / 1000.0);
        }
        Some(Commands::BuildPosIdx { db_path, max_ply, threads }) => {
            let start = std::time::Instant::now();
            let path_str = db_path.to_string_lossy();
            let idx = if path_str.ends_with(".pgn") {
                let pgn_db = pgn_db::PgnDatabaseWrapper::open(&db_path)?;
                position_index::PositionIndex::build_for_pgn(&db_path, &pgn_db.entries, pgn_db.mmap_ref(), max_ply, threads, |scanned, total, positions| {
                    print!("\r  Indexing games: {} / {} ({:.1}%) | Unique positions: {}", scanned, total, (scanned as f64 / total as f64) * 100.0, positions);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                })?
            } else {
                let db = ScidDatabaseWrapper::open(&db_path)?;
                let games_path = db.games_path().to_path_buf();
                let entries = db.entries();
                let db_path_buf = db.index_path().to_path_buf();
                position_index::PositionIndex::build_for_scid(&db_path_buf, entries, &games_path, max_ply, threads, |scanned, total, positions| {
                    print!("\r  Indexing games: {} / {} ({:.1}%) | Unique positions: {}", scanned, total, (scanned as f64 / total as f64) * 100.0, positions);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                })?
            };
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            println!("\n[OK] Built {} in {:.2} ms ({} unique positions).", idx.path.display(), elapsed_ms, idx.header.unique_positions);
        }
        Some(Commands::Tree { db_path, fen }) => {
            let idx = position_index::PositionIndex::load(&db_path)?;
            let fen_str = fen.as_deref().unwrap_or("");
            if let Some(tree) = idx.query_tree(fen_str) {
                println!("Opening Tree for position (Total Games: {} | +{:.1}% / ={:.1}% / -{:.1}%):", tree.total_games, tree.white_pct, tree.draw_pct, tree.black_pct);
                println!("{:<6} | {:<8} | {:<10} | {:<7} | {:<7} | {:<7} | {:<8}",
                    "Move", "UCI", "Games", "1-0 %", "1/2 %", "0-1 %", "Avg Elo");
                println!("{:-<6}-+-{:-<8}-+-{:-<10}-+-{:-<7}-+-{:-<7}-+-{:-<7}-+-{:-<8}",
                    "", "", "", "", "", "", "");
                for m in tree.moves {
                    let avg_elo_str = match (m.avg_white_elo, m.avg_black_elo) {
                        (Some(w), Some(b)) => format!("{}/{}", w, b),
                        (Some(w), None) => format!("{}/-", w),
                        _ => "-".to_string(),
                    };
                    println!("{:<6} | {:<8} | {:<10} | {:<6.1}% | {:<6.1}% | {:<6.1}% | {:<8}",
                        m.san, m.uci, m.total_games, m.white_pct, m.draw_pct, m.black_pct, avg_elo_str);
                }
            } else {
                println!("No games found reaching this position in the opening index.");
            }
        }
        None => {
            // Default to interactive mode if a db path was provided, or print help
            if let Some(path) = cli.db_path {
                server::run_interactive_server(Some(path), cli.threads)?;
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
