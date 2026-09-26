pub mod handlers;

use crate::db::ScidDatabaseWrapper;
use crate::pgn_db::PgnDatabaseWrapper;
use crate::position_index::PositionIndex;
use crate::tree_index::TreeIndex;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

#[allow(clippy::large_enum_variant)]
pub enum DatabaseBackend {
    Scid(ScidDatabaseWrapper),
    Pgn(PgnDatabaseWrapper),
}

#[derive(Debug, Deserialize)]
pub struct RequestMessage {
    pub id: Option<u64>,
    pub command: String,
    #[serde(flatten)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct ResponseMessage {
    pub id: Option<u64>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn run_interactive_server(
    initial_db_path: Option<PathBuf>,
    initial_threads: Option<usize>,
) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = stdin.lock();

    let max_system_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let mut current_thread_count = initial_threads.unwrap_or(max_system_threads).max(1);
    let mut thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(current_thread_count)
        .build()?;

    let mut current_db: Option<DatabaseBackend> = None;
    let mut current_pos_index: Option<PositionIndex> = None;
    let mut current_tree_index: Option<TreeIndex> = None;

    if let Some(path) = initial_db_path {
        if path.exists() {
            let path_str = path.to_string_lossy().to_lowercase();
            if path_str.ends_with(".pgn") {
                match PgnDatabaseWrapper::open(&path) {
                    Ok(pgn) => {
                        eprintln!("[Server] Auto-opened PGN database: {}", path.display());
                        current_db = Some(DatabaseBackend::Pgn(pgn));
                    }
                    Err(e) => {
                        eprintln!("[Server] Failed to auto-open {}: {}", path.display(), e);
                    }
                }
            } else {
                match ScidDatabaseWrapper::open(&path) {
                    Ok(db) => {
                        eprintln!("[Server] Auto-opened SCID database: {}", path.display());
                        current_db = Some(DatabaseBackend::Scid(db));
                    }
                    Err(e) => {
                        eprintln!("[Server] Failed to auto-open {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        let req: RequestMessage = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let resp = ResponseMessage {
                    id: None,
                    status: "error".to_string(),
                    data: None,
                    error: Some(format!("Invalid JSON request: {}", e)),
                };
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
                line.clear();
                continue;
            }
        };

        if req.command == "shutdown" || req.command == "exit" || req.command == "quit" {
            let resp = ResponseMessage {
                id: req.id,
                status: "ok".to_string(),
                data: Some(serde_json::json!({"message": "Shutting down"})),
                error: None,
            };
            writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            stdout.flush()?;
            break;
        }

        let resp = handle_command(
            &mut current_db,
            &mut current_pos_index,
            &mut current_tree_index,
            &mut thread_pool,
            &mut current_thread_count,
            max_system_threads,
            &req,
        );
        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
        stdout.flush()?;
        line.clear();
    }

    Ok(())
}

fn handle_command(
    current_db: &mut Option<DatabaseBackend>,
    current_pos_index: &mut Option<PositionIndex>,
    current_tree_index: &mut Option<TreeIndex>,
    thread_pool: &mut rayon::ThreadPool,
    current_thread_count: &mut usize,
    max_system_threads: usize,
    req: &RequestMessage,
) -> ResponseMessage {
    let id = req.id;
    let cmd = req.command.as_str();

    match cmd {
        // System / Configuration
        "set_threads" | "set_config" => handlers::config::handle_set_threads(
            req,
            thread_pool,
            current_thread_count,
            max_system_threads,
        ),
        "get_threads" | "get_config" => {
            handlers::config::handle_get_threads(req, *current_thread_count, max_system_threads)
        }

        // Database Lifecycle & Inspection
        "open" | "open_db" => {
            handlers::db::handle_open_db(req, current_db, current_pos_index, current_tree_index)
        }
        "create" => handlers::db::handle_create_db(req, current_db),
        "info" | "stats" => handlers::db::handle_info_stats(req, current_db),
        "query_games" | "get_games" => {
            handlers::db::handle_query_games(req, current_db, thread_pool)
        }
        "get_game_summaries" => handlers::db::handle_get_game_summaries(req, current_db),
        "get_pgn" | "get_game" | "get_game_pgn" => {
            handlers::db::handle_get_game_pgn(req, current_db)
        }

        // Position & Material Searches
        "search_position" | "position_search" => handlers::position::handle_search_position(
            req,
            current_db,
            current_pos_index,
            thread_pool,
        ),
        "search_material" | "material_search" => {
            handlers::position::handle_search_material(req, current_db)
        }

        // Opening Tree Operations
        "opening_tree" | "query_tree" => handlers::tree::handle_opening_tree(
            req,
            current_db,
            current_pos_index,
            current_tree_index,
        ),
        "tree_index_status" | "get_tree_index_status" | "tree_status" => {
            handlers::tree::handle_tree_index_status(req, current_db, current_tree_index)
        }
        "tree_index_diagnostics" | "get_tree_index_diagnostics" | "tree_diagnostics" => {
            handlers::tree::handle_tree_index_diagnostics(req, current_db, current_tree_index)
        }
        "build_tree" | "build_tree_index" | "rebuild_tree" => {
            handlers::tree::handle_build_tree_index(
                req,
                current_db,
                current_tree_index,
                *current_thread_count,
            )
        }

        // Common Continuations Operations
        "continuations" | "common_continuations" | "hot_continuations" | "get_continuations" => {
            handlers::continuations::handle_continuations(req, current_db, current_pos_index)
        }
        "build_continuations" | "build_hot_index" | "build_hot" | "rebuild_continuations" => {
            handlers::continuations::handle_build_continuations_index(
                req,
                current_db,
                *current_thread_count,
            )
        }

        // Position Index Operations
        "unload_pos_index" => handlers::index::handle_unload_pos_index(req, current_pos_index),
        "pos_index_status" | "get_pos_index_status" => {
            handlers::index::handle_pos_index_status(req, current_db, current_pos_index)
        }
        "pos_index_diagnostics" | "get_pos_index_diagnostics" => {
            handlers::index::handle_pos_index_diagnostics(req, current_db, current_pos_index)
        }
        "build_pos_index" | "rebuild_pos_index" => handlers::index::handle_build_pos_index(
            req,
            current_db,
            current_pos_index,
            *current_thread_count,
        ),

        // Database Editing & Mutation
        "add_game" => handlers::db::handle_add_game(req, current_db),
        "update_game" => handlers::db::handle_update_game(req, current_db),
        "delete_game" => handlers::db::handle_delete_game(req, current_db),
        "undelete_game" => handlers::db::handle_undelete_game(req, current_db),
        "compact" => handlers::db::handle_compact(req, current_db),
        "save" => handlers::db::handle_save(req, current_db),
        "sort_database" | "sort_db" => {
            handlers::db::handle_sort_database(req, current_db, current_pos_index)
        }
        "sort_pgn" => handlers::db::handle_sort_pgn(req, current_db),

        // Import, Export & Benchmark
        "import_pgn" => handlers::import_export::handle_import_pgn(req, current_db),
        "export_pgn" => handlers::import_export::handle_export_pgn(req, current_db),
        "benchmark" | "bench" => handlers::import_export::handle_benchmark(req, current_db),

        // CQLite Search & Query Analysis
        "validate_dsl" | "validate_cql" => handlers::search::handle_validate_dsl(req),
        "explain_dsl" | "explain_cql" | "explain_query" | "explain" => {
            handlers::search::handle_explain_dsl(req)
        }
        "search" | "search_query" | "query_search" | "dsl_search" | "cql_search" => {
            handlers::search::handle_cql_search(req, current_db, thread_pool)
        }

        unknown => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Unknown command: '{}'", unknown)),
        },
    }
}
