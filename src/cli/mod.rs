pub mod commands;
pub mod formatters;

use crate::server;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "scid-mgr")]
#[command(about = "SCID (si4 / si5) Chess Database Manager & Interactive Server")]
pub struct Cli {
    /// Run as interactive JSON-RPC server on stdin/stdout
    #[arg(short, long)]
    pub interactive: bool,

    /// Max CPU worker threads for search and indexing
    #[arg(short, long)]
    pub threads: Option<usize>,

    /// Optional database path to auto-open in interactive mode
    #[arg(value_name = "DB_PATH")]
    pub db_path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum BuildCommands {
    /// Build companion .pos.idx position index for ultra-fast candidate searches
    #[command(name = "pos-idx", alias = "pos")]
    PosIdx {
        /// Path to .si5, .si4, or .pgn database
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Maximum ply depth to index (default: 24, i.e. 12 full moves)
        #[arg(long, default_value = "24")]
        max_ply: usize,

        /// Max game IDs to store per move (default: 0 for all games / unlimited)
        #[arg(long, default_value = "0")]
        max_games: usize,

        /// Minimum games reaching a position to include it in the index (default: 1, i.e. all positions)
        #[arg(long, default_value = "1")]
        min_games: usize,

        /// Number of worker threads (default: all available CPU cores)
        #[arg(long)]
        threads: Option<usize>,
    },

    /// Build companion .tree.idx opening tree statistics index
    #[command(name = "tree", alias = "tree-idx")]
    Tree {
        /// Path to .si5, .si4, or .pgn database
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Maximum ply depth to index (default: 24, i.e. 12 full moves)
        #[arg(long, default_value = "24")]
        max_ply: usize,

        /// Minimum games reaching a position to include it in the index (default: 1, i.e. all positions)
        #[arg(long, default_value = "1")]
        min_games: usize,

        /// Number of worker threads (default: all available CPU cores)
        #[arg(long)]
        threads: Option<usize>,
    },

    /// Build companion .hot.idx common continuations graph index
    #[command(name = "continuations", aliases = ["hot", "cont", "hot-idx", "cont-idx"])]
    Continuations {
        /// Path to .si5, .si4, or .pgn database
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Maximum ply depth to index (default: 24, i.e. 12 full moves)
        #[arg(long, default_value = "24")]
        max_ply: usize,

        /// Minimum games reaching a position to include it in the index (default: 1)
        #[arg(long, default_value = "1")]
        min_games: usize,
    },

    /// Build companion .feat.idx endgame taxonomy and feature index
    #[command(name = "endgames", aliases = ["feat", "feat-idx", "endgame"])]
    Endgames {
        /// Path to .si5, .si4, or .pgn database
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Custom catalog YAML path (optional)
        #[arg(long)]
        catalog: Option<PathBuf>,
    },

    /// Build both companion indexes (.pos.idx and .tree.idx)
    #[command(name = "all")]
    All {
        /// Path to .si5, .si4, or .pgn database
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Maximum ply depth to index (default: 24, i.e. 12 full moves)
        #[arg(long, default_value = "24")]
        max_ply: usize,

        /// Minimum games reaching a position to include it in the index (default: 1, i.e. all positions)
        #[arg(long, default_value = "1")]
        min_games: usize,

        /// Number of worker threads (default: all available CPU cores)
        #[arg(long)]
        threads: Option<usize>,
    },
}

#[derive(Subcommand)]
pub enum Commands {
    /// Check database integrity and companion index synchronization status (.pos.idx, .tree.idx)
    Check {
        /// Path to .si5, .si4, or .pgn database
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Include detailed posting and tree distribution diagnostics
        #[arg(long, short)]
        detailed: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

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

    /// Search games using unified CQLite text query or launch interactive search REPL
    #[command(alias = "cql")]
    Search {
        /// Path to .si4, .si5, or .pgn file
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Search query expression (e.g. "player 'Kasparov' and [Qq]==0 and move A--")
        #[arg(value_name = "QUERY")]
        query: Option<String>,

        /// Launch interactive search REPL shell (default if no query is provided)
        #[arg(short, long)]
        interactive: bool,

        /// Maximum number of matching games to return (default: 50)
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Start game index (0-based) for sub-range search
        #[arg(long)]
        start_game: Option<usize>,

        /// End game index (0-based, exclusive) for sub-range search
        #[arg(long)]
        end_game: Option<usize>,

        /// Output results as formatted JSON
        #[arg(long)]
        json: bool,

        /// Only output the total count of matching games
        #[arg(long)]
        count_only: bool,
    },

    /// Launch interactive CQLite search REPL shell
    #[command(alias = "repl")]
    Shell {
        /// Path to .si4, .si5, or .pgn file
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Result display limit
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Explain, normalize, and inspect CQL queries (parsed AST, canonical DSL, and symmetry branch breakdowns)
    Explain {
        /// Query expression to parse and explain
        #[arg(value_name = "QUERY")]
        query: String,

        /// Output explanation as formatted JSON
        #[arg(long)]
        json: bool,
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

    /// Build companion acceleration indexes (.pos.idx position index, .tree.idx opening tree)
    Build {
        #[command(subcommand)]
        target: BuildCommands,
    },

    /// Query the instant opening tree for any board position (FEN or starting board)
    Tree {
        /// Path to .si5, .si4, or .pgn database
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Optional FEN position (defaults to initial board)
        #[arg(long)]
        fen: Option<String>,

        /// Max sample game IDs to retrieve (defaults to 20; 0 for stats only)
        #[arg(long, default_value = "20")]
        sample_games: usize,

        /// Retrieve all game IDs from posting list
        #[arg(long)]
        all_game_ids: bool,
    },

    /// Query common multi-ply continuations from a board position (FEN or starting board)
    #[command(name = "continuations", aliases = ["continuation", "hot", "cont"])]
    Continuations {
        /// Path to .si5, .si4, or .pgn database
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Optional FEN position (defaults to initial board)
        #[arg(long)]
        fen: Option<String>,

        /// Maximum depth in plies (half-moves) after the position (1..=20, default: 8)
        #[arg(long, default_value = "8")]
        max_depth: usize,

        /// Maximum number of continuation lines returned (default: 10)
        #[arg(long, default_value = "10")]
        max_lines: usize,

        /// Minimum number of games in which a continuation must occur (default: 1)
        #[arg(long, default_value = "1")]
        min_games: u64,

        /// Minimum percentage of games reaching starting position (e.g. 0.5 for 0.5%)
        #[arg(long, default_value = "0.0")]
        min_percentage: f64,
    },

    /// Show endgame popularity breakdown or query games matching an endgame feature
    #[command(name = "endgames", aliases = ["endgame", "feat", "features"])]
    Endgames {
        /// Path to .si4, .si5, or .pgn file
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Filter endgame popularity from a specific opening/middlegame position FEN
        #[arg(long)]
        fen: Option<String>,

        /// Filter to a specific category (e.g. PAWN, ROOK, BISHOP, KNIGHT, MINOR_MIXED, QUEEN, ROOK_VS_MINOR, QUEEN_VS_PIECES)
        #[arg(long)]
        category: Option<String>,

        /// Query games matching a specific endgame feature ID (e.g. END_ROOK_RP_R) or bit number
        #[arg(long)]
        feature: Option<String>,

        /// Output results as formatted JSON
        #[arg(long)]
        json: bool,

        /// Maximum sample game IDs to display for a feature query (default: 20)
        #[arg(long, default_value = "20")]
        limit: usize,
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

    /// Sort games in a PGN file and export to a new PGN file
    SortPgn {
        /// Source input PGN file
        #[arg(value_name = "INPUT_PGN")]
        input_pgn: PathBuf,

        /// Destination sorted output PGN file
        #[arg(value_name = "OUTPUT_PGN")]
        output_pgn: PathBuf,

        /// Sort field (date, white_elo, black_elo, white, black, eco, result, event, site)
        #[arg(long, default_value = "date")]
        sort_by: String,

        /// Sort descending
        #[arg(long)]
        desc: bool,
    },

    /// Sort and compact games in a SCID database (.si5 or .si4)
    SortDb {
        /// Source SCID database path
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Optional destination SCID database path (if omitted, sorts in-place)
        #[arg(value_name = "OUTPUT_PATH")]
        output_path: Option<PathBuf>,

        /// Sort field (date, white_elo, black_elo, white, black, eco, result, event, site)
        #[arg(long, default_value = "date")]
        sort_by: String,

        /// Sort descending
        #[arg(long)]
        desc: bool,

        /// Keep deleted games (default: false, deleted games are purged during sort/compaction)
        #[arg(long)]
        keep_deleted: bool,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.interactive {
        return server::run_interactive_server(cli.db_path, cli.threads);
    }

    match cli.command {
        Some(Commands::Interactive { db_path, threads }) => {
            server::run_interactive_server(db_path, threads.or(cli.threads))?;
        }
        Some(Commands::Check {
            db_path,
            detailed,
            json,
        }) => {
            commands::check::handle_check(&db_path, detailed, json)?;
        }
        Some(Commands::Info { db_path }) => {
            commands::info::handle_info(&db_path)?;
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
            commands::list::handle_list(&db_path, page, page_size, player, eco, sort_by, desc)?;
        }
        Some(Commands::Search {
            db_path,
            query,
            interactive,
            limit,
            start_game,
            end_game,
            json,
            count_only,
        }) => {
            if interactive || query.is_none() {
                commands::repl::handle_repl(&db_path, limit)?;
            } else if let Some(q) = query {
                commands::search::handle_search(
                    &db_path, &q, limit, start_game, end_game, json, count_only,
                )?;
            }
        }
        Some(Commands::Shell { db_path, limit }) => {
            commands::repl::handle_repl(&db_path, limit)?;
        }
        Some(Commands::Explain { query, json }) => {
            commands::search::handle_explain(&query, json)?;
        }
        Some(Commands::SearchPos {
            db_path,
            fen,
            max_ply,
        }) => {
            commands::search::handle_search_pos(&db_path, &fen, max_ply)?;
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
            commands::search::handle_search_mat(
                &db_path,
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
            )?;
        }
        Some(Commands::Get { db_path, index }) => {
            commands::search::handle_get(&db_path, index)?;
        }
        Some(Commands::Import {
            db_path,
            pgn_path,
            format,
        }) => {
            commands::import_export::handle_import(&db_path, &pgn_path, &format)?;
        }
        Some(Commands::Export {
            db_path,
            output_pgn,
        }) => {
            commands::import_export::handle_export(&db_path, &output_pgn)?;
        }
        Some(Commands::Create { db_path, format }) => {
            commands::import_export::handle_create(&db_path, &format)?;
        }
        Some(Commands::Bench { db_path, heavy }) => {
            commands::bench::handle_bench(&db_path, heavy)?;
        }
        Some(Commands::Build { target }) => match target {
            BuildCommands::PosIdx {
                db_path,
                max_ply,
                max_games,
                min_games,
                threads,
            } => {
                commands::index::handle_build_pos_idx(
                    &db_path, max_ply, max_games, min_games, threads,
                )?;
            }
            BuildCommands::Tree {
                db_path,
                max_ply,
                min_games,
                threads,
            } => {
                commands::tree::handle_build_tree(&db_path, max_ply, min_games, threads)?;
            }
            BuildCommands::Continuations {
                db_path,
                max_ply,
                min_games,
            } => {
                commands::continuations::handle_build_continuations(&db_path, max_ply, min_games)?;
            }
            BuildCommands::Endgames { db_path, catalog } => {
                commands::endgames::handle_build_endgames(&db_path, catalog.as_deref())?;
            }
            BuildCommands::All {
                db_path,
                max_ply,
                min_games,
                threads,
            } => {
                commands::index::handle_build_all(&db_path, max_ply, min_games, threads)?;
            }
        },
        Some(Commands::Tree {
            db_path,
            fen,
            sample_games,
            all_game_ids,
        }) => {
            commands::tree::handle_tree(&db_path, fen, sample_games, all_game_ids)?;
        }
        Some(Commands::Continuations {
            db_path,
            fen,
            max_depth,
            max_lines,
            min_games,
            min_percentage,
        }) => {
            commands::continuations::handle_continuations(
                &db_path,
                fen,
                max_depth,
                max_lines,
                min_games,
                min_percentage,
            )?;
        }
        Some(Commands::Endgames {
            db_path,
            fen,
            category,
            feature,
            json,
            limit,
        }) => {
            commands::endgames::handle_endgames(&db_path, fen, category, feature, json, limit)?;
        }
        Some(Commands::SortPgn {
            input_pgn,
            output_pgn,
            sort_by,
            desc,
        }) => {
            commands::sort::handle_sort_pgn(&input_pgn, &output_pgn, &sort_by, desc)?;
        }
        Some(Commands::SortDb {
            db_path,
            output_path,
            sort_by,
            desc,
            keep_deleted,
        }) => {
            commands::sort::handle_sort_db(
                &db_path,
                output_path.as_deref(),
                &sort_by,
                desc,
                keep_deleted,
            )?;
        }
        None => {
            if let Some(path) = cli.db_path {
                server::run_interactive_server(Some(path), cli.threads)?;
            } else {
                println!("Run 'scid-mgr --help' for CLI options or 'scid-mgr check <DB_PATH>' to verify a database.");
            }
        }
    }

    Ok(())
}
