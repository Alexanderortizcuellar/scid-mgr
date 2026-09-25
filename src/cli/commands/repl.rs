use crate::cli::formatters::truncate_str;
use crate::db::ScidDatabaseWrapper;
use crate::pgn_db::PgnDatabaseWrapper;
use crate::search::parser::QueryParser;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

type GameSummaryTuple = (String, String, String, String);
type SearchSessionResult = (
    Vec<crate::search::ScidMatchResult>,
    usize,
    HashMap<usize, GameSummaryTuple>,
);

enum DatabaseSession {
    Scid(ScidDatabaseWrapper),
    Pgn(PgnDatabaseWrapper),
}

impl DatabaseSession {
    fn open(path: &Path) -> Result<Self> {
        let path_str = path.to_string_lossy().to_lowercase();
        if path_str.ends_with(".pgn") {
            let pgn = PgnDatabaseWrapper::open(path)
                .with_context(|| format!("Failed to open PGN database at {}", path.display()))?;
            Ok(DatabaseSession::Pgn(pgn))
        } else {
            let scid = ScidDatabaseWrapper::open(path)
                .with_context(|| format!("Failed to open SCID database at {}", path.display()))?;
            Ok(DatabaseSession::Scid(scid))
        }
    }

    fn game_count(&self) -> usize {
        match self {
            DatabaseSession::Scid(db) => db.game_count(),
            DatabaseSession::Pgn(db) => db.game_count(),
        }
    }

    fn file_name(&self) -> String {
        match self {
            DatabaseSession::Scid(db) => db
                .index_path()
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "scid_db".into()),
            DatabaseSession::Pgn(db) => db
                .pgn_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "pgn_db".into()),
        }
    }

    fn format_name(&self) -> &'static str {
        match self {
            DatabaseSession::Scid(_) => "SCID (.si4/.si5)",
            DatabaseSession::Pgn(_) => "PGN (text)",
        }
    }

    fn search(&self, query: &crate::search::SearchQuery, limit: usize) -> SearchSessionResult {
        match self {
            DatabaseSession::Pgn(db) => {
                let total = db.game_count();
                let matches = db.search_query_with_progress(query, |_, _, _| {});
                let mut summs = HashMap::new();
                for m in matches.iter().take(limit) {
                    let g = db.get_summary(m.game_id);
                    summs.insert(m.game_id, (g.white, g.black, g.result, g.date));
                }
                (matches, total, summs)
            }
            DatabaseSession::Scid(db) => {
                let total = db.game_count();
                let matches = db.search_query_with_progress(query, |_, _, _| {});
                let mut summs = HashMap::new();
                for m in matches.iter().take(limit) {
                    if let Some(g) = db.get_game_summary(m.game_id) {
                        summs.insert(m.game_id, (g.white, g.black, g.result, g.date));
                    }
                }
                (matches, total, summs)
            }
        }
    }

    fn get_pgn(&self, game_id: usize) -> Result<String> {
        match self {
            DatabaseSession::Pgn(db) => db.get_game_pgn(game_id),
            DatabaseSession::Scid(db) => db.game_pgn(game_id),
        }
    }

    fn print_info(&self) {
        match self {
            DatabaseSession::Pgn(pgn) => {
                println!("Database:     {}", pgn.pgn_path.display());
                println!("Format:       PGN (Plain Text Database)");
                println!("Total Games:  {}", pgn.game_count());
            }
            DatabaseSession::Scid(db) => {
                let stats = db.stats();
                println!("Database:       {}", stats.index_path);
                println!("Format:         {}", stats.format);
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
    }
}

pub fn handle_repl(initial_db_path: &Path, default_limit: usize) -> Result<()> {
    let mut current_path = initial_db_path.to_path_buf();
    let mut session = DatabaseSession::open(&current_path)?;
    let mut limit = default_limit.max(1);

    println!("================================================================================");
    println!(" CQLite Interactive Shell (scid-mgr)");
    println!(
        " Connected to: {} [{}] ({} games)",
        session.file_name(),
        session.format_name(),
        session.game_count()
    );
    println!(" Type '.help' for command reference, '.schema' for CQL syntax, 'exit' to quit.");
    println!("================================================================================");

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    loop {
        let prompt_name = session.file_name();
        print!("cqlite [{}]> ", prompt_name);
        io::stdout().flush().ok();

        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            // EOF reached
            println!("\nGoodbye!");
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Meta commands handling
        if trimmed.starts_with('.')
            || trimmed.eq_ignore_ascii_case("exit")
            || trimmed.eq_ignore_ascii_case("quit")
            || trimmed.eq_ignore_ascii_case("help")
            || trimmed.eq_ignore_ascii_case("clear")
            || trimmed.eq_ignore_ascii_case("cls")
        {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            let cmd = parts[0].to_lowercase();
            let arg = if parts.len() > 1 {
                trimmed[parts[0].len()..].trim()
            } else {
                ""
            };

            match cmd.as_str() {
                ".exit" | ".quit" | "exit" | "quit" => {
                    println!("Goodbye!");
                    break;
                }
                ".clear" | "clear" | ".cls" | "cls" => {
                    print!("\x1B[2J\x1B[1;1H");
                    io::stdout().flush().ok();
                }
                ".help" | "help" => {
                    print_help();
                }
                ".schema" => {
                    print_schema();
                }
                ".info" => {
                    session.print_info();
                }
                ".limit" => {
                    if arg.is_empty() {
                        println!("Current result limit: {}", limit);
                    } else if let Ok(new_limit) = arg.parse::<usize>() {
                        if new_limit > 0 {
                            limit = new_limit;
                            println!("Result limit set to: {}", limit);
                        } else {
                            eprintln!("Limit must be greater than 0");
                        }
                    } else {
                        eprintln!("Invalid limit: '{}'. Expected integer.", arg);
                    }
                }
                ".open" => {
                    if arg.is_empty() {
                        eprintln!("Usage: .open <DB_PATH>");
                    } else {
                        let new_path = PathBuf::from(arg.trim_matches('"').trim_matches('\''));
                        match DatabaseSession::open(&new_path) {
                            Ok(new_session) => {
                                current_path = new_path;
                                session = new_session;
                                println!(
                                    "Opened {} [{}] ({} games)",
                                    session.file_name(),
                                    session.format_name(),
                                    session.game_count()
                                );
                            }
                            Err(e) => {
                                eprintln!("Error opening database: {:#}", e);
                            }
                        }
                    }
                }
                ".get" => {
                    if arg.is_empty() {
                        eprintln!("Usage: .get <GAME_ID>");
                    } else if let Ok(id) = arg.parse::<usize>() {
                        match session.get_pgn(id) {
                            Ok(pgn) => println!("\n{}\n", pgn),
                            Err(e) => eprintln!("Failed to retrieve game {}: {:#}", id, e),
                        }
                    } else {
                        eprintln!("Invalid game ID: '{}'. Expected non-negative integer.", arg);
                    }
                }
                ".explain" => {
                    if arg.is_empty() {
                        eprintln!("Usage: .explain <CQL_QUERY>");
                    } else if let Err(e) = crate::cli::commands::search::handle_explain(arg, false)
                    {
                        eprintln!("Explain error: {:#}", e);
                    }
                }
                ".tree" => {
                    let fen_opt = if arg.is_empty() {
                        None
                    } else {
                        Some(arg.to_string())
                    };
                    if let Err(e) =
                        crate::cli::commands::tree::handle_tree(&current_path, fen_opt, 20, false)
                    {
                        eprintln!("Tree error: {:#}", e);
                    }
                }
                _ => {
                    eprintln!(
                        "Unknown shell command: '{}'. Type '.help' for available commands.",
                        cmd
                    );
                }
            }
            continue;
        }

        // Execute CQL query
        let parsed_query = match QueryParser::parse_str(trimmed) {
            Ok(q) => q,
            Err(e) => {
                eprintln!("\nQuery Parse Error: {}", e);
                if let Some(help) = &e.help {
                    eprintln!("Help: {}", help);
                }
                println!();
                continue;
            }
        };

        let start_time = Instant::now();
        let (matches, total_games, summaries) = session.search(&parsed_query, limit);
        let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;

        if matches.is_empty() {
            println!(
                "0 matches found ({:.2} ms across {} games)\n",
                elapsed_ms, total_games
            );
            continue;
        }

        println!();
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

        let displayed = matches.len().min(limit);
        println!(
            "\n{} matches found in {:.2} ms across {} games (displaying 1..{}).",
            matches.len(),
            elapsed_ms,
            total_games,
            displayed
        );
        if matches.len() > limit {
            println!("Use '.limit <N>' to increase display size.");
        }
        println!("Use '.get <id>' to inspect full game PGN.\n");
    }

    Ok(())
}

fn print_help() {
    println!(
        r#"
=== CQLite Interactive Shell Commands ===
  .help                 Show this help overview
  .schema               Display CQL grammar cheatsheet and field reference
  .info                 Show database metadata, sizes, and game counts
  .limit [N]            Get or set display result limit (default: 20)
  .open <PATH>          Open a different database file (.si5, .si4, .pgn)
  .get <GAME_ID>        Print reconstructed PGN for a matching game
  .explain <QUERY>      Inspect AST, canonical DSL, and symmetry expansions
  .tree [FEN]           Inspect opening tree statistics for position
  .clear / clear        Clear terminal screen
  .exit / .quit         Exit the shell

=== Quick CQL Examples ===
  player 'Kasparov' and result 1-0
  white 'Carlsen' and eco B90..B99 and elo >= 2800
  [Qq]==0 and [Rr]>=3
  pos [K.k] and move A--
  year >= 2020 and result 1/2-1/2
"#
    );
}

fn print_schema() {
    println!(
        r#"
=== CQLite Query Language Specification ===

1. Header Filters:
   player <NAME>             Case-insensitive player name substring
   white <NAME>              White player name substring
   black <NAME>              Black player name substring
   event <EVENT>             Event name substring
   site <SITE>               Site name substring
   round <ROUND>             Round string
   result <RES>              Game result: 1-0, 0-1, 1/2-1/2, *
   eco <CODE>                ECO code prefix or range (e.g. B90, B90..B99)
   elo <OP> <NUM>            Player Elo rating (e.g. elo >= 2700, white_elo > 2800)
   year / date <OP> <NUM>    Game date or year (e.g. year >= 2020, date >= 2015.01.01)

2. Piece Counts & Material:
   [<PIECE_CHARS>] <OP> <N>  Count pieces on board (e.g. [Qq]==0, [Rr]>=3, [Nn]==2)
                             Pieces: K, Q, R, B, N, P (uppercase=White, lowercase=Black)
   material <OP> <N>         Count total material on board

3. Positional & Move Patterns:
   pos <FEN_OR_PIECES>       Board layout / partial piece placement
   move <SPEC>               Move pattern (e.g. move A--, move e4, move Nf3)
   square <SQ>               Piece occupancy on square (e.g. square e4 P)
   ply <OP> <N>              Ply depth constraint (e.g. ply <= 20)

4. Logical Operators:
   AND, OR, NOT, ( ... )     Combine expressions with boolean precedence
"#
    );
}
