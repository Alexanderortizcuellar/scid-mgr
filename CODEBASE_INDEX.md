# scid-mgr Codebase Symbol & Architecture Index
> Auto-generated index mapping modules, files, classes, structs, traits, functions, and IPC commands.
> To update this index at any time, run: `python scripts/index_codebase.py`

## Table of Contents
- [1. Architectural Overview](#1-architectural-overview)
- [2. JSON-RPC Server IPC Commands](#2-json-rpc-server-ipc-commands)
- [3. Rust Backend (`src/`)](#3-rust-backend-src)
- [4. Python GUI Frontend (`scripts/gui/`)](#4-python-gui-frontend-scriptsgui)
- [5. Alphabetical Symbol Quick-Find](#5-alphabetical-symbol-quick-find)

## 1. Architectural Overview

| Layer | Module / Directory | Key Responsibilities |
|---|---|---|
| **Backend Core** | `src/db.rs` | SCID `.si4`/`.si5` database wrapper, record reading, in-place sorting, editing, headers |
| **Backend Core** | `src/pgn_db.rs` | Fast memory-mapped / indexed PGN wrapper with string deduplication tables |
| **Backend Core** | `src/pgn_utils.rs` | Streaming PGN parser, chunk-based PGN importer with progress callbacks |
| **Indexing Engine** | `src/position_index.rs` | Inverted position index (`.pos.idx`) with Delta-Varint posting lists for instant candidate filtering |
| **Indexing Engine** | `src/tree_index.rs` | Opening tree index (`.tree.idx`) calculating move frequencies, win/draw/loss stats, ECO trees |
| **Indexing Engine** | `src/zero_copy_ingest.rs` | High-speed zero-copy stream ingestion for massive PGN datasets directly into indices |
| **Search (CQLite)** | `src/search/` | Complete AST query parser, semantic validator, tacticals, piece paths, square sets, FEN transformations |
| **IPC Server** | `src/server.rs` | JSON-RPC server on stdin/stdout connecting Python GUI and Rust engine with multithreading |
| **CLI Binary** | `src/main.rs` | CLI commands (`info`, `list`, `search`, `tree`, `index`, `sort`, `import`, `bench`) |
| **GUI Frontend** | `scripts/gui/main_window.py` | PySide6 / PyQt application window, layout, and event wiring |
| **GUI Client** | `scripts/gui/backend_client.py` | Subprocess IPC client communicating asynchronously with `scid-mgr --interactive` |
| **GUI Widgets** | `scripts/gui/widgets/` | Chessboard, opening tree explorer, CQLite search query visualizer, filter panel, game list |

## 2. JSON-RPC Server IPC Commands

Defined in [`src/server.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server.rs):

| Command | Line | Description |
|---|---|---|

## 3. Rust Backend (`src/`)

### [`src/benchmark.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/benchmark.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `BenchmarkItem` | [`L9`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/benchmark.rs#L9) |
| `struct` | `BenchmarkReport` | [`L18`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/benchmark.rs#L18) |
| `fn` | `run_benchmark` | [`L30`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/benchmark.rs#L30) |

### [`src/lib.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/lib.rs)

_No public structs or functions found._

### [`src/main.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/main.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `main` | [`L3`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/main.rs#L3) |

### [`src/cli/formatters.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/formatters.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `truncate_str` | [`L1`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/formatters.rs#L1) |

### [`src/cli/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/mod.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `Cli` | [`L12`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/mod.rs#L12) |
| `enum` | `BuildCommands` | [`L30`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/mod.rs#L30) |
| `enum` | `Commands` | [`L97`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/mod.rs#L97) |
| `fn` | `run` | [`L422`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/mod.rs#L422) |

### [`src/cli/commands/bench.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/bench.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_bench` | [`L5`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/bench.rs#L5) |

### [`src/cli/commands/check.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/check.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `DatabaseCheckReport` | [`L10`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/check.rs#L10) |
| `struct` | `IndexCheckReport` | [`L25`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/check.rs#L25) |
| `fn` | `handle_check` | [`L39`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/check.rs#L39) |

### [`src/cli/commands/import_export.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/import_export.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_import` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/import_export.rs#L6) |
| `fn` | `handle_export` | [`L46`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/import_export.rs#L46) |
| `fn` | `handle_create` | [`L69`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/import_export.rs#L69) |

### [`src/cli/commands/index.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/index.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_build_pos_idx` | [`L7`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/index.rs#L7) |
| `fn` | `handle_build_all` | [`L74`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/index.rs#L74) |

### [`src/cli/commands/info.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/info.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_info` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/info.rs#L6) |

### [`src/cli/commands/list.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/list.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_list` | [`L7`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/list.rs#L7) |

### [`src/cli/commands/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/mod.rs)

_No public structs or functions found._

### [`src/cli/commands/repl.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs)

| Kind | Name | Line |
|---|---|---|
| `enum` | `DatabaseSession` | [`L11`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L11) |
| `fn` | `DatabaseSession::open` | [`L17`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L17) |
| `fn` | `DatabaseSession::game_count` | [`L30`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L30) |
| `fn` | `DatabaseSession::file_name` | [`L37`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L37) |
| `fn` | `DatabaseSession::format_name` | [`L52`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L52) |
| `fn` | `DatabaseSession::search` | [`L59`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L59) |
| `fn` | `DatabaseSession::get_pgn` | [`L93`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L93) |
| `fn` | `DatabaseSession::print_info` | [`L100`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L100) |
| `fn` | `handle_repl` | [`L126`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L126) |
| `fn` | `print_help` | [`L329`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L329) |
| `fn` | `print_schema` | [`L353`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L353) |

### [`src/cli/commands/search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/search.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_search` | [`L8`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/search.rs#L8) |
| `fn` | `handle_explain` | [`L155`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/search.rs#L155) |
| `fn` | `handle_search_pos` | [`L195`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/search.rs#L195) |
| `fn` | `handle_search_mat` | [`L253`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/search.rs#L253) |
| `fn` | `handle_get` | [`L350`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/search.rs#L350) |

### [`src/cli/commands/sort.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/sort.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_sort_pgn` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/sort.rs#L6) |
| `fn` | `handle_sort_db` | [`L30`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/sort.rs#L30) |

### [`src/cli/commands/tree.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/tree.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_build_tree` | [`L7`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/tree.rs#L7) |
| `fn` | `handle_tree` | [`L72`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/tree.rs#L72) |

### [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `ScidDatabaseWrapper` | [`L13`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L13) |
| `fn` | `detect_format_from_path` | [`L31`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L31) |
| `fn` | `ScidDatabaseWrapper::open` | [`L49`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L49) |
| `fn` | `ScidDatabaseWrapper::create` | [`L116`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L116) |
| `fn` | `ScidDatabaseWrapper::format` | [`L151`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L151) |
| `fn` | `ScidDatabaseWrapper::index_path` | [`L155`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L155) |
| `fn` | `ScidDatabaseWrapper::game_count` | [`L159`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L159) |
| `fn` | `ScidDatabaseWrapper::entries` | [`L163`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L163) |
| `fn` | `ScidDatabaseWrapper::names` | [`L167`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L167) |
| `fn` | `ScidDatabaseWrapper::games_path` | [`L171`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L171) |
| `fn` | `ScidDatabaseWrapper::is_deleted` | [`L175`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L175) |
| `fn` | `ScidDatabaseWrapper::get_blob` | [`L179`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L179) |
| `fn` | `ScidDatabaseWrapper::game_pgn` | [`L205`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L205) |
| `fn` | `ScidDatabaseWrapper::stats` | [`L216`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L216) |

### [`src/db/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mod.rs)

_No public structs or functions found._

### [`src/db/mutations.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mutations.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `ScidDatabaseWrapper::add_game` | [`L12`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mutations.rs#L12) |
| `fn` | `ScidDatabaseWrapper::update_game` | [`L53`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mutations.rs#L53) |
| `fn` | `ScidDatabaseWrapper::delete_game` | [`L98`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mutations.rs#L98) |
| `fn` | `ScidDatabaseWrapper::undelete_game` | [`L108`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mutations.rs#L108) |
| `fn` | `ScidDatabaseWrapper::compact` | [`L118`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mutations.rs#L118) |
| `fn` | `ScidDatabaseWrapper::save` | [`L148`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mutations.rs#L148) |

### [`src/db/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `ScidDatabaseWrapper::clear_query_caches` | [`L10`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L10) |
| `fn` | `ScidDatabaseWrapper::get_player_ranks` | [`L19`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L19) |
| `fn` | `ScidDatabaseWrapper::get_event_ranks` | [`L32`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L32) |
| `fn` | `ScidDatabaseWrapper::get_site_ranks` | [`L45`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L45) |
| `fn` | `ScidDatabaseWrapper::get_round_ranks` | [`L58`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L58) |
| `fn` | `ScidDatabaseWrapper::get_game_summary` | [`L71`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L71) |
| `fn` | `ScidDatabaseWrapper::query_games_with_progress` | [`L101`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L101) |
| `fn` | `ScidDatabaseWrapper::query_games` | [`L577`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L577) |
| `fn` | `ScidDatabaseWrapper::get_cached_query_indices` | [`L586`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L586) |

### [`src/db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `ScidDatabaseWrapper::search_position_with_progress` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L6) |
| `fn` | `ScidDatabaseWrapper::search_position` | [`L72`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L72) |
| `fn` | `ScidDatabaseWrapper::search_material_with_progress` | [`L82`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L82) |
| `fn` | `ScidDatabaseWrapper::search_material` | [`L98`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L98) |
| `fn` | `ScidDatabaseWrapper::search_query_with_progress` | [`L106`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L106) |
| `fn` | `ScidDatabaseWrapper::search_query_progress_helper` | [`L123`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L123) |
| `fn` | `ScidDatabaseWrapper::search_query` | [`L141`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L141) |
| `fn` | `ScidDatabaseWrapper::search_query_range_with_progress` | [`L149`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L149) |
| `fn` | `ScidDatabaseWrapper::search_query_range` | [`L171`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L171) |

### [`src/db/sorting.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/sorting.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `ScidDatabaseWrapper::sort_database` | [`L12`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/sorting.rs#L12) |
| `fn` | `ScidDatabaseWrapper::sort_database_to` | [`L61`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/sorting.rs#L61) |
| `fn` | `ScidDatabaseWrapper::sort_indices` | [`L128`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/sorting.rs#L128) |

### [`src/db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs)

| Kind | Name | Line |
|---|---|---|
| `enum` | `ScidFormat` | [`L5`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L5) |
| `fn` | `std::fmt` | [`L11`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L11) |
| `struct` | `GameSummary` | [`L20`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L20) |
| `struct` | `GameFilter` | [`L39`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L39) |
| `fn` | `GameFilter::same_search_criteria` | [`L64`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L64) |
| `fn` | `GameFilter::is_empty` | [`L85`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L85) |
| `struct` | `DbStats` | [`L105`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L105) |
| `fn` | `result_code_to_str` | [`L120`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L120) |

### [`src/pgn_db/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/builder.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `save_index_file` | [`L14`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/builder.rs#L14) |
| `fn` | `scan_pgn_parallel` | [`L77`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/builder.rs#L77) |
| `fn` | `scan_chunk` | [`L179`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/builder.rs#L179) |
| `fn` | `parse_tag` | [`L283`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/builder.rs#L283) |

### [`src/pgn_db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `PgnDatabaseWrapper` | [`L20`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L20) |
| `fn` | `PgnDatabaseWrapper::companion_path` | [`L36`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L36) |
| `fn` | `PgnDatabaseWrapper::load_index_file` | [`L48`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L48) |
| `fn` | `PgnDatabaseWrapper::open` | [`L88`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L88) |
| `fn` | `PgnDatabaseWrapper::game_count` | [`L150`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L150) |
| `fn` | `PgnDatabaseWrapper::mmap_ref` | [`L154`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L154) |
| `fn` | `PgnDatabaseWrapper::get_game_pgn` | [`L159`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L159) |
| `fn` | `PgnDatabaseWrapper::get_summary` | [`L179`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L179) |
| `fn` | `PgnDatabaseWrapper::query_games_with_progress` | [`L201`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L201) |
| `fn` | `PgnDatabaseWrapper::query_games` | [`L669`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L669) |
| `fn` | `PgnDatabaseWrapper::get_cached_query_indices` | [`L678`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L678) |

### [`src/pgn_db/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/mod.rs)

_No public structs or functions found._

### [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `PgnDatabaseWrapper::search_position` | [`L15`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L15) |
| `fn` | `PgnDatabaseWrapper::search_material` | [`L107`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L107) |
| `fn` | `PgnDatabaseWrapper::search_query_range_with_progress` | [`L148`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L148) |
| `fn` | `PgnDatabaseWrapper::search_query_with_progress` | [`L247`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L247) |
| `fn` | `PgnDatabaseWrapper::search_query` | [`L259`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L259) |
| `fn` | `PgnDatabaseWrapper::search_query_range` | [`L267`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L267) |
| `struct` | `PositionFinder` | [`L281`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L281) |
| `fn` | `PositionFinder::new` | [`L290`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L290) |
| `fn` | `PositionFinder::check_current_pos` | [`L302`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L302) |
| `fn` | `pgn_reader::begin_game` | [`L315`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L315) |
| `fn` | `pgn_reader::begin_variation` | [`L319`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L319) |
| `fn` | `pgn_reader::san` | [`L323`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L323) |
| `fn` | `pgn_reader::end_game` | [`L335`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L335) |
| `struct` | `MaterialFinder` | [`L340`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L340) |
| `fn` | `MaterialFinder::new` | [`L350`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L350) |
| `fn` | `MaterialFinder::check_material` | [`L367`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L367) |
| `fn` | `pgn_reader::begin_variation` | [`L375`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L375) |
| `fn` | `pgn_reader::san` | [`L379`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L379) |
| `fn` | `pgn_reader::end_game` | [`L396`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L396) |

### [`src/pgn_db/sorting.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `PgnDatabaseWrapper::get_player_ranks` | [`L14`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs#L14) |
| `fn` | `PgnDatabaseWrapper::get_event_ranks` | [`L27`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs#L27) |
| `fn` | `PgnDatabaseWrapper::get_site_ranks` | [`L40`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs#L40) |
| `fn` | `PgnDatabaseWrapper::sort_indices` | [`L53`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs#L53) |
| `fn` | `PgnDatabaseWrapper::sort_and_export` | [`L147`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs#L147) |
| `fn` | `sort_pgn_file` | [`L186`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs#L186) |

### [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `CompactPgnRecord` | [`L9`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L9) |
| `fn` | `CompactPgnRecord::result_str` | [`L28`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L28) |
| `fn` | `CompactPgnRecord::date_str` | [`L33`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L33) |
| `fn` | `CompactPgnRecord::eco_str` | [`L38`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L38) |
| `struct` | `PgnNameTables` | [`L45`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L45) |
| `fn` | `PgnNameTables::new` | [`L52`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L52) |
| `fn` | `PgnNameTables::player` | [`L61`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L61) |
| `fn` | `PgnNameTables::event` | [`L69`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L69) |
| `fn` | `PgnNameTables::site` | [`L77`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L77) |
| `struct` | `PgnIndexHeader` | [`L87`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L87) |
| `struct` | `RawGameRecord` | [`L99`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L99) |
| `fn` | `pack_date` | [`L113`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L113) |
| `fn` | `unpack_date` | [`L124`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L124) |
| `fn` | `pack_eco` | [`L146`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L146) |
| `fn` | `unpack_eco` | [`L166`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L166) |
| `fn` | `pack_result` | [`L176`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L176) |
| `fn` | `unpack_result` | [`L185`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L185) |

### [`src/pgn_io/encoder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/encoder.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `standard_piece_slots` | [`L7`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/encoder.rs#L7) |
| `fn` | `encode_rook_like` | [`L19`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/encoder.rs#L19) |
| `fn` | `encode_scid_move_byte` | [`L30`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/encoder.rs#L30) |
| `fn` | `update_piece_slots` | [`L132`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/encoder.rs#L132) |

### [`src/pgn_io/export.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/export.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `fast_game_to_pgn` | [`L13`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/export.rs#L13) |
| `fn` | `export_pgn_ultra_fast` | [`L133`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/export.rs#L133) |
| `fn` | `export_pgn_file` | [`L214`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/export.rs#L214) |

### [`src/pgn_io/import.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/import.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `parse_game_bytes_fast` | [`L17`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/import.rs#L17) |
| `fn` | `import_pgn_ultra_fast` | [`L279`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/import.rs#L279) |
| `fn` | `import_pgn_file_with_progress` | [`L481`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/import.rs#L481) |

### [`src/pgn_io/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/mod.rs)

_No public structs or functions found._

### [`src/pgn_io/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `ImportProgress` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L6) |
| `struct` | `ExportProgress` | [`L17`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L17) |
| `struct` | `RawPgnTags` | [`L26`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L26) |
| `struct` | `FastNameTables` | [`L40`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L40) |
| `fn` | `FastNameTables::from_name_tables` | [`L52`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L52) |
| `fn` | `FastNameTables::player_id` | [`L86`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L86) |
| `fn` | `FastNameTables::event_id` | [`L98`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L98) |
| `fn` | `FastNameTables::site_id` | [`L110`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L110) |
| `fn` | `FastNameTables::round_id` | [`L122`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L122) |
| `fn` | `FastNameTables::to_name_tables` | [`L133`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L133) |

### [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `StripedPositionPostingMap` | [`L25`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L25) |
| `fn` | `StripedPositionPostingMap::new` | [`L31`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L31) |
| `fn` | `StripedPositionPostingMap::stripe_index` | [`L43`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L43) |
| `fn` | `StripedPositionPostingMap::record` | [`L47`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L47) |
| `fn` | `StripedPositionPostingMap::total_positions` | [`L65`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L65) |
| `fn` | `StripedPositionPostingMap::into_map` | [`L69`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L69) |
| `struct` | `PgnPositionVisitor` | [`L90`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L90) |
| `fn` | `PgnPositionVisitor::new` | [`L99`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L99) |
| `fn` | `pgn_reader::begin_game` | [`L117`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L117) |
| `fn` | `pgn_reader::begin_variation` | [`L124`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L124) |
| `fn` | `pgn_reader::san` | [`L128`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L128) |
| `fn` | `pgn_reader::end_game` | [`L141`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L141) |
| `fn` | `build_for_scid` | [`L151`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L151) |
| `fn` | `build_for_pgn` | [`L307`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L307) |
| `fn` | `write_static_binary_file` | [`L382`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L382) |

### [`src/position_index/codec.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/codec.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `encode_posting_payload` | [`L13`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/codec.rs#L13) |
| `fn` | `decode_position_game_ids` | [`L28`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/codec.rs#L28) |
| `fn` | `parse_target_position` | [`L43`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/codec.rs#L43) |
| `fn` | `write_varint` | [`L65`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/codec.rs#L65) |
| `fn` | `read_varint` | [`L74`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/codec.rs#L74) |

### [`src/position_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `PositionIndex` | [`L15`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L15) |
| `fn` | `PositionIndex::companion_path` | [`L23`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L23) |
| `fn` | `PositionIndex::check_status` | [`L43`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L43) |
| `fn` | `PositionIndex::load` | [`L100`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L100) |
| `fn` | `PositionIndex::index_entries` | [`L136`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L136) |
| `fn` | `PositionIndex::get_matching_game_ids` | [`L148`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L148) |
| `fn` | `PositionIndex::get_all_position_games` | [`L172`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L172) |
| `fn` | `PositionIndex::scan_diagnostics` | [`L177`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L177) |
| `fn` | `PositionIndex::build_for_scid` | [`L221`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L221) |
| `fn` | `PositionIndex::build_for_pgn` | [`L238`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L238) |

### [`src/position_index/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/mod.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `test_posting_payload_roundtrip` | [`L24`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/mod.rs#L24) |

### [`src/position_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs)

| Kind | Name | Line |
|---|---|---|
| `enum` | `IndexStatus` | [`L13`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L13) |
| `struct` | `PositionIndexHeader` | [`L20`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L20) |
| `fn` | `PositionIndexHeader::read_from_slice` | [`L35`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L35) |
| `fn` | `PositionIndexHeader::write_to` | [`L70`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L70) |
| `struct` | `SortedIndexEntry` | [`L87`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L87) |
| `struct` | `PositionPostingList` | [`L93`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L93) |
| `fn` | `PositionPostingList::new` | [`L99`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L99) |
| `fn` | `PositionPostingList::add` | [`L107`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L107) |
| `fn` | `PositionPostingList::merge` | [`L114`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L114) |
| `struct` | `IndexDiagnostics` | [`L122`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L122) |

### [`src/position_search/decoder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/decoder.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `skip_extra_tags` | [`L8`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/decoder.rs#L8) |
| `fn` | `decode_extra_tags` | [`L33`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/decoder.rs#L33) |
| `fn` | `parse_start_position` | [`L91`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/decoder.rs#L91) |
| `fn` | `standard_piece_slots` | [`L118`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/decoder.rs#L118) |
| `fn` | `update_slots_on_move` | [`L131`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/decoder.rs#L131) |
| `fn` | `decode_raw_move` | [`L172`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/decoder.rs#L172) |

### [`src/position_search/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/mod.rs)

_No public structs or functions found._

### [`src/position_search/parser.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/parser.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `parse_piece_placements` | [`L8`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/parser.rs#L8) |
| `fn` | `matches_piece_placements` | [`L56`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/parser.rs#L56) |
| `fn` | `parse_position_matcher` | [`L67`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/parser.rs#L67) |

### [`src/position_search/scid_search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `matches_material` | [`L19`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L19) |
| `fn` | `search_position_matcher_mmap_with_progress` | [`L106`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L106) |
| `fn` | `search_position_mmap_with_progress` | [`L231`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L231) |
| `fn` | `search_position_mmap` | [`L263`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L263) |
| `fn` | `search_piece_placements_mmap_with_progress` | [`L273`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L273) |
| `fn` | `search_piece_placements_mmap` | [`L399`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L399) |
| `fn` | `search_material_mmap_with_progress` | [`L417`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L417) |
| `fn` | `search_material_mmap` | [`L539`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L539) |

### [`src/position_search/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/types.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `PositionMatch` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/types.rs#L6) |
| `struct` | `PositionSearchResult` | [`L12`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/types.rs#L12) |
| `enum` | `PositionTargetMatcher` | [`L21`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/types.rs#L21) |
| `fn` | `PositionTargetMatcher::matches` | [`L35`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/types.rs#L35) |
| `struct` | `MaterialFilter` | [`L60`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/types.rs#L60) |

### [`src/search/annotation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs)

| Kind | Name | Line |
|---|---|---|
| `enum` | `AnnotationPredicate` | [`L3`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L3) |
| `enum` | `CommentPredicate` | [`L12`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L12) |
| `fn` | `CommentPredicate::matches` | [`L30`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L30) |
| `enum` | `NagPredicate` | [`L67`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L67) |
| `fn` | `NagPredicate::matches` | [`L77`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L77) |
| `fn` | `NagPredicate::symbol_to_nag` | [`L86`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L86) |
| `struct` | `AnnotationManager` | [`L118`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L118) |
| `fn` | `AnnotationManager::strip_comments` | [`L122`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L122) |
| `fn` | `AnnotationManager::extract_comments` | [`L144`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L144) |

### [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs)

| Kind | Name | Line |
|---|---|---|
| `trait` | `ToDsl` | [`L13`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L13) |
| `fn` | `to_dsl` | [`L14`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L14) |
| `fn` | `ComparisonOp (impl ToDsl)::to_dsl` | [`L18`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L18) |
| `fn` | `PieceMatcher (impl ToDsl)::to_dsl` | [`L35`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L35) |
| `fn` | `SquareOrPiece (impl ToDsl)::to_dsl` | [`L69`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L69) |
| `fn` | `SquareContent (impl ToDsl)::to_dsl` | [`L80`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L80) |
| `fn` | `SquareSetExpr (impl ToDsl)::to_dsl` | [`L137`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L137) |
| `fn` | `SetPredicate (impl ToDsl)::to_dsl` | [`L197`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L197) |
| `fn` | `HeaderPredicate (impl ToDsl)::to_dsl` | [`L211`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L211) |
| `fn` | `PositionPattern (impl ToDsl)::to_dsl` | [`L286`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L286) |
| `fn` | `MovePattern (impl ToDsl)::to_dsl` | [`L399`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L399) |
| `fn` | `PathPattern (impl ToDsl)::to_dsl` | [`L542`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L542) |
| `fn` | `CqlPathConstituent (impl ToDsl)::to_dsl` | [`L608`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L608) |
| `fn` | `CqlPathPattern (impl ToDsl)::to_dsl` | [`L642`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L642) |
| `fn` | `CqlLinePattern (impl ToDsl)::to_dsl` | [`L669`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L669) |
| `fn` | `TacticalPredicate (impl ToDsl)::to_dsl` | [`L721`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L721) |
| `fn` | `MaterialPredicate (impl ToDsl)::to_dsl` | [`L854`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L854) |
| `fn` | `PawnPredicate (impl ToDsl)::to_dsl` | [`L904`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L904) |
| `fn` | `PowerPredicate (impl ToDsl)::to_dsl` | [`L951`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L951) |
| `fn` | `AnnotationPredicate (impl ToDsl)::to_dsl` | [`L981`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L981) |
| `fn` | `SearchQuery (impl ToDsl)::to_dsl` | [`L987`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L987) |
| `struct` | `QueryBranch` | [`L1109`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L1109) |
| `struct` | `QueryExplanation` | [`L1116`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L1116) |
| `fn` | `collect_fens` | [`L1124`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L1124) |
| `fn` | `contains_symmetry` | [`L1155`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L1155) |
| `fn` | `symmetry_label` | [`L1168`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L1168) |
| `fn` | `explain_query` | [`L1184`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L1184) |

### [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `HeaderMatcher` | [`L5`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L5) |
| `fn` | `HeaderMatcher::matches` | [`L9`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L9) |
| `fn` | `HeaderMatcher::matches_entry` | [`L133`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L133) |
| `fn` | `HeaderMatcher::matches_pgn_entry` | [`L387`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L387) |
| `fn` | `get_header_value` | [`L624`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L624) |
| `fn` | `parse_u16` | [`L638`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L638) |
| `fn` | `compare_numeric` | [`L642`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L642) |
| `fn` | `compare_string` | [`L659`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L659) |
| `struct` | `ParsedDate` | [`L690`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L690) |
| `fn` | `ParsedDate::parse` | [`L697`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L697) |
| `fn` | `ParsedDate::to_bound` | [`L736`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L736) |
| `fn` | `compare_date` | [`L744`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L744) |
| `fn` | `wildcard_match` | [`L797`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L797) |

### [`src/search/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/mod.rs)

_No public structs or functions found._

### [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `MoveRecord` | [`L14`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L14) |
| `struct` | `PathMatcher` | [`L24`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L24) |
| `fn` | `PathMatcher::match_move` | [`L28`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L28) |
| `fn` | `PathMatcher::match_legal_move` | [`L279`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L279) |
| `fn` | `PathMatcher::match_path` | [`L515`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L515) |
| `fn` | `match_steps_from` | [`L584`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L584) |
| `fn` | `move_destination_matches` | [`L693`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L693) |
| `fn` | `move_destinations_contain` | [`L710`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L710) |
| `struct` | `CqlPathMatcher` | [`L736`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L736) |
| `fn` | `CqlPathMatcher::match_cql_path` | [`L740`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L740) |
| `fn` | `match_cql_constituents_from` | [`L786`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L786) |
| `fn` | `match_rep_helper` | [`L885`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L885) |
| `struct` | `CqlLineMatcher` | [`L1049`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L1049) |
| `fn` | `CqlLineMatcher::match_cql_line` | [`L1053`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L1053) |
| `fn` | `match_line_forward_from` | [`L1162`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L1162) |
| `fn` | `try_rep_forward` | [`L1262`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L1262) |
| `fn` | `match_line_backward_from` | [`L1463`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L1463) |
| `fn` | `try_rep_backward` | [`L1574`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L1574) |

### [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `PositionMatcher` | [`L12`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L12) |
| `fn` | `PositionMatcher::matches` | [`L16`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L16) |
| `fn` | `PositionMatcher::matches_at_ply` | [`L21`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L21) |
| `fn` | `PositionMatcher::matches_at_ply_with_env` | [`L161`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L161) |
| `fn` | `PositionMatcher::matches_material` | [`L180`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L180) |
| `fn` | `PositionMatcher::matches_power` | [`L373`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L373) |
| `fn` | `match_square_content` | [`L407`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L407) |
| `fn` | `count_square_content` | [`L420`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L420) |
| `fn` | `calculate_power` | [`L460`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L460) |
| `fn` | `check_opposite_bishops` | [`L495`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L495) |
| `fn` | `check_same_colored_bishops` | [`L505`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L505) |
| `fn` | `compare_usize` | [`L515`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L515) |
| `fn` | `compare_i32` | [`L527`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L527) |
| `fn` | `match_wildcard_fen` | [`L539`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L539) |
| `enum` | `RankCellMatcher` | [`L592`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L592) |
| `fn` | `match_rank_pattern` | [`L601`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L601) |
| `fn` | `parse_fen_piece_char` | [`L626`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L626) |
| `fn` | `match_cells_recursive` | [`L680`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L680) |

### [`src/search/pawn.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `PawnEvaluator` | [`L74`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L74) |
| `fn` | `PawnEvaluator::matches` | [`L78`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L78) |
| `fn` | `PawnEvaluator::count_passed_pawns` | [`L104`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L104) |
| `fn` | `PawnEvaluator::count_isolated_pawns` | [`L135`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L135) |
| `fn` | `PawnEvaluator::pawn_file_mask` | [`L151`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L151) |
| `fn` | `PawnEvaluator::count_doubled_pawns` | [`L165`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L165) |
| `fn` | `PawnEvaluator::count_backward_pawns` | [`L175`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L175) |
| `fn` | `PawnEvaluator::count_pawn_islands` | [`L226`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L226) |
| `fn` | `rank_span_above_or_equal` | [`L236`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L236) |
| `fn` | `rank_span_below_or_equal` | [`L244`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L244) |
| `fn` | `pawn_attacks` | [`L252`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L252) |
| `fn` | `compare_usize` | [`L285`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L285) |

### [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs)

| Kind | Name | Line |
|---|---|---|
| `enum` | `SquareSetExpr` | [`L7`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L7) |
| `enum` | `SetPredicate` | [`L67`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L67) |
| `enum` | `ComparisonOp` | [`L91`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L91) |
| `fn` | `ComparisonOp::invert` | [`L105`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L105) |
| `enum` | `HeaderPredicate` | [`L118`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L118) |
| `enum` | `SquareContent` | [`L188`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L188) |
| `enum` | `PositionPattern` | [`L200`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L200) |
| `enum` | `Direction` | [`L257`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L257) |
| `fn` | `Direction::delta_vectors` | [`L274`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L274) |
| `fn` | `Direction::expand_square` | [`L301`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L301) |
| `fn` | `Direction::expand_squares` | [`L320`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L320) |
| `struct` | `MovePattern` | [`L333`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L333) |
| `enum` | `PathStep` | [`L383`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L383) |
| `struct` | `PathPattern` | [`L396`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L396) |
| `enum` | `CqlPathConstituent` | [`L414`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L414) |
| `struct` | `CqlPathPattern` | [`L431`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L431) |
| `fn` | `CqlPathPattern::requires_san_strings` | [`L441`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L441) |
| `enum` | `LineDirection` | [`L448`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L448) |
| `struct` | `CqlLinePattern` | [`L456`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L456) |
| `fn` | `CqlLinePattern::requires_san_strings` | [`L470`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L470) |
| `fn` | `CqlPathConstituent::requires_san_strings` | [`L476`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L476) |
| `struct` | `MaterialPredicate` | [`L490`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L490) |
| `struct` | `PieceMatcher` | [`L517`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L517) |
| `fn` | `PieceMatcher::new` | [`L536`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L536) |
| `fn` | `PieceMatcher::matches` | [`L540`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L540) |
| `enum` | `SquareOrPiece` | [`L557`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L557) |
| `enum` | `VariableDomain` | [`L566`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L566) |
| `enum` | `TacticalPredicate` | [`L574`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L574) |
| `enum` | `PawnPredicate` | [`L626`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L626) |
| `enum` | `PowerPredicate` | [`L656`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L656) |
| `enum` | `SearchQuery` | [`L675`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L675) |
| `fn` | `SearchQuery::and` | [`L740`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L740) |
| `fn` | `SearchQuery::or` | [`L744`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L744) |
| `fn` | `SearchQuery::negate` | [`L748`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L748) |
| `fn` | `SearchQuery::is_header_only` | [`L752`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L752) |
| `fn` | `SearchQuery::requires_san_strings` | [`L767`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L767) |
| `fn` | `SearchQuery::has_header_predicates` | [`L790`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L790) |
| `fn` | `SearchQuery::can_stream_early_exit` | [`L810`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L810) |
| `fn` | `SearchQuery::max_ply_cutoff` | [`L840`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L840) |
| `fn` | `std::not` | [`L859`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L859) |

### [`src/search/scid_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `ScidMatchResult` | [`L14`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L14) |
| `struct` | `ScidSearchAdapter` | [`L20`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L20) |
| `fn` | `ScidSearchAdapter::evaluate_scid_game` | [`L24`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L24) |
| `fn` | `ScidSearchAdapter::evaluate_scid_game_streaming` | [`L123`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L123) |
| `fn` | `ScidSearchAdapter::replay_scid_blob` | [`L236`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L236) |
| `fn` | `ScidSearchAdapter::replay_scid_blob_opt` | [`L241`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L241) |
| `fn` | `ScidSearchAdapter::search_parallel_range_with_progress` | [`L325`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L325) |
| `fn` | `ScidSearchAdapter::search_parallel_with_progress` | [`L410`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L410) |
| `fn` | `ScidSearchAdapter::search_parallel` | [`L433`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L433) |
| `fn` | `ScidSearchAdapter::search_parallel_range` | [`L446`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L446) |
| `fn` | `matches_single_ply` | [`L469`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L469) |

### [`src/search/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/squares.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `SquareSetEvaluator` | [`L8`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/squares.rs#L8) |
| `fn` | `SquareSetEvaluator::eval_expr` | [`L12`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/squares.rs#L12) |
| `fn` | `SquareSetEvaluator::matches_with_env` | [`L140`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/squares.rs#L140) |
| `fn` | `SquareSetEvaluator::matches` | [`L180`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/squares.rs#L180) |

### [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `TacticsEvaluator` | [`L5`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L5) |
| `fn` | `TacticsEvaluator::matches_with_env` | [`L9`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L9) |
| `fn` | `TacticsEvaluator::matches` | [`L56`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L56) |
| `fn` | `TacticsEvaluator::matches_attacks_with_env` | [`L61`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L61) |
| `fn` | `TacticsEvaluator::matches_attacks` | [`L83`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L83) |
| `fn` | `TacticsEvaluator::has_pin` | [`L88`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L88) |
| `fn` | `TacticsEvaluator::has_fork` | [`L181`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L181) |
| `fn` | `TacticsEvaluator::match_distinct_target_slots` | [`L256`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L256) |
| `fn` | `TacticsEvaluator::has_discovered_attack` | [`L283`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L283) |
| `fn` | `TacticsEvaluator::has_skewer` | [`L317`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L317) |
| `fn` | `TacticsEvaluator::has_trapped_piece` | [`L403`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L403) |
| `fn` | `TacticsEvaluator::has_outpost` | [`L438`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L438) |
| `fn` | `TacticsEvaluator::has_rook_on_seventh` | [`L529`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L529) |
| `fn` | `TacticsEvaluator::has_open_file` | [`L545`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L545) |
| `fn` | `TacticsEvaluator::matches_distance_with_env` | [`L596`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L596) |
| `fn` | `TacticsEvaluator::matches_distance` | [`L620`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L620) |
| `fn` | `resolve_squares` | [`L638`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L638) |
| `fn` | `chebyshev_distance` | [`L675`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L675) |
| `fn` | `square_from_coords` | [`L684`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L684) |
| `fn` | `pawn_defenders` | [`L692`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L692) |
| `fn` | `piece_value` | [`L723`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L723) |
| `fn` | `compare_distance` | [`L734`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L734) |

### [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs)

| Kind | Name | Line |
|---|---|---|
| `enum` | `BoardSymmetry` | [`L11`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L11) |
| `fn` | `BoardSymmetry::expand` | [`L38`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L38) |
| `fn` | `BoardSymmetry::transform_square` | [`L79`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L79) |
| `fn` | `BoardSymmetry::transform_piece` | [`L101`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L101) |
| `fn` | `BoardSymmetry::transform_piece_matcher` | [`L112`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L112) |
| `fn` | `BoardSymmetry::transform_square_or_piece` | [`L126`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L126) |
| `fn` | `BoardSymmetry::transform_square_content` | [`L136`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L136) |
| `fn` | `BoardSymmetry::transform_fen` | [`L158`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L158) |
| `fn` | `BoardSymmetry::transform_piece_placement` | [`L207`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L207) |
| `fn` | `BoardSymmetry::transform_castling_rights` | [`L265`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L265) |
| `fn` | `BoardSymmetry::transform_position_pattern` | [`L317`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L317) |
| `fn` | `BoardSymmetry::transform_tactical_predicate` | [`L395`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L395) |
| `fn` | `BoardSymmetry::transform_material_predicate` | [`L517`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L517) |
| `fn` | `BoardSymmetry::transform_query` | [`L587`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L587) |
| `fn` | `BoardSymmetry::transform_cql_line_pattern` | [`L781`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L781) |
| `fn` | `BoardSymmetry::transform_cql_path_pattern` | [`L812`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L812) |
| `fn` | `BoardSymmetry::transform_cql_path_constituent` | [`L836`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L836) |
| `fn` | `BoardSymmetry::transform_set_predicate` | [`L867`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L867) |
| `fn` | `BoardSymmetry::transform_square_set_expr` | [`L886`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L886) |
| `fn` | `BoardSymmetry::transform_direction` | [`L945`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L945) |
| `fn` | `BoardSymmetry::transform_header_predicate` | [`L1016`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L1016) |
| `fn` | `BoardSymmetry::transform_move_pattern` | [`L1072`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L1072) |
| `struct` | `TransformMatcher` | [`L1125`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L1125) |
| `fn` | `TransformMatcher::matches_with_symmetry` | [`L1129`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L1129) |

### [`src/search/evaluator/matcher.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `evaluate_with_timeline_env` | [`L14`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs#L14) |
| `fn` | `quick_check_headers_only` | [`L744`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs#L744) |
| `fn` | `quick_check_entry_headers` | [`L769`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs#L769) |
| `fn` | `quick_check_pgn_entry_headers` | [`L795`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs#L795) |
| `fn` | `matches_single_ply` | [`L821`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs#L821) |
| `fn` | `predicate_eval_cost` | [`L974`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs#L974) |

### [`src/search/evaluator/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `QueryMatchResult` | [`L22`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L22) |
| `struct` | `GameSearchEvaluator` | [`L29`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L29) |
| `fn` | `GameSearchEvaluator::evaluate_pgn` | [`L33`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L33) |
| `fn` | `GameSearchEvaluator::evaluate_pgn_detailed` | [`L68`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L68) |
| `fn` | `GameSearchEvaluator::evaluate_game` | [`L90`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L90) |
| `fn` | `GameSearchEvaluator::evaluate_with_timeline` | [`L121`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L121) |
| `fn` | `GameSearchEvaluator::evaluate_with_timeline_env` | [`L131`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L131) |

### [`src/search/evaluator/replayer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/replayer.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `split_pgn_headers_and_moves` | [`L13`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/replayer.rs#L13) |
| `fn` | `parse_pgn_headers_and_moves` | [`L47`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/replayer.rs#L47) |
| `fn` | `evaluate_pgn_streaming` | [`L53`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/replayer.rs#L53) |
| `fn` | `replay_game` | [`L178`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/replayer.rs#L178) |

### [`src/search/parser/headers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/headers.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `QueryParser::parse_header_keyword` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/headers.rs#L6) |

### [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `parse_piece_specifier` | [`L7`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L7) |
| `fn` | `parse_compact_piece_placement` | [`L87`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L87) |
| `fn` | `parse_square_or_piece` | [`L171`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L171) |
| `fn` | `expand_rectangular_range` | [`L189`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L189) |
| `fn` | `expand_diagonal_ray` | [`L205`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L205) |
| `fn` | `expand_square_specifier` | [`L233`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L233) |
| `fn` | `file_char_to_idx` | [`L344`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L344) |
| `fn` | `rank_char_to_idx` | [`L351`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L351) |
| `fn` | `parse_source_part` | [`L359`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L359) |
| `fn` | `parse_direction_ident` | [`L463`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L463) |
| `fn` | `parse_target_part` | [`L485`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L485) |
| `fn` | `parse_capture_move` | [`L626`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L626) |
| `fn` | `parse_quiet_move` | [`L642`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L642) |
| `fn` | `parse_path_move_token` | [`L663`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L663) |
| `fn` | `parse_promotion_into_pattern` | [`L839`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L839) |

### [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `ParseError` | [`L3`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L3) |
| `fn` | `ParseError::new` | [`L19`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L19) |
| `fn` | `ParseError::with_help` | [`L30`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L30) |
| `fn` | `ParseError::with_source_context` | [`L36`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L36) |
| `fn` | `calculate_line_col_snippet` | [`L59`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L59) |
| `fn` | `generate_smart_help` | [`L90`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L90) |
| `fn` | `std::fmt` | [`L112`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L112) |
| `enum` | `Token` | [`L131`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L131) |
| `struct` | `Lexer` | [`L162`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L162) |
| `fn` | `Lexer::new` | [`L169`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L169) |
| `fn` | `Lexer::current_pos` | [`L177`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L177) |
| `fn` | `Lexer::peek` | [`L185`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L185) |
| `fn` | `Lexer::advance` | [`L189`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L189) |
| `fn` | `Lexer::tokenize` | [`L199`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L199) |
| `fn` | `Lexer::is_next_digit` | [`L457`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L457) |
| `fn` | `is_ident_start` | [`L466`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L466) |
| `fn` | `is_ident_char` | [`L470`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L470) |

### [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `QueryParser` | [`L23`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L23) |
| `fn` | `QueryParser::new` | [`L29`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L29) |
| `fn` | `QueryParser::parse_str` | [`L34`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L34) |
| `fn` | `QueryParser::explain` | [`L59`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L59) |
| `fn` | `QueryParser::peek` | [`L64`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L64) |
| `fn` | `QueryParser::peek_nth` | [`L68`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L68) |
| `fn` | `QueryParser::current_pos` | [`L72`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L72) |
| `fn` | `QueryParser::advance` | [`L76`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L76) |
| `fn` | `QueryParser::match_ident` | [`L86`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L86) |
| `fn` | `QueryParser::expect_ident` | [`L96`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L96) |
| `fn` | `QueryParser::expect_string_or_ident` | [`L111`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L111) |
| `fn` | `QueryParser::expect_number` | [`L127`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L127) |
| `fn` | `QueryParser::expect_token` | [`L142`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L142) |
| `fn` | `QueryParser::parse_comparison_op` | [`L157`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L157) |
| `fn` | `QueryParser::parse_string_comparison_op` | [`L190`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L190) |
| `fn` | `QueryParser::parse_string_or_regex_val` | [`L247`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L247) |
| `fn` | `QueryParser::parse_or_expr` | [`L275`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L275) |
| `fn` | `QueryParser::parse_and_expr` | [`L292`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L292) |
| `fn` | `QueryParser::has_more_in_and` | [`L308`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L308) |
| `fn` | `QueryParser::parse_unary_expr` | [`L320`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L320) |
| `fn` | `QueryParser::parse_primary_expr` | [`L328`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L328) |

### [`src/search/parser/moves.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `parse_token_repetition` | [`L19`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L19) |
| `fn` | `validate_move_pattern` | [`L40`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L40) |
| `fn` | `validate_single_color_step` | [`L87`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L87) |
| `fn` | `QueryParser::parse_path_expr` | [`L134`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L134) |
| `fn` | `QueryParser::parse_cql_path_expr` | [`L419`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L419) |
| `fn` | `QueryParser::parse_cql_path_single_constituent` | [`L511`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L511) |
| `fn` | `QueryParser::parse_cql_path_repetition_quantifier` | [`L701`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L701) |
| `fn` | `QueryParser::parse_cql_line_or_legacy_path` | [`L746`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L746) |
| `fn` | `QueryParser::parse_cql_line_expr` | [`L793`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L793) |
| `fn` | `QueryParser::parse_cql_line_constituent` | [`L974`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L974) |
| `fn` | `QueryParser::parse_square_or_piece_set` | [`L1417`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L1417) |
| `fn` | `QueryParser::parse_move_filter` | [`L1573`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L1573) |

### [`src/search/parser/pieces.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `QueryParser::parse_piece_on_square` | [`L14`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L14) |
| `fn` | `QueryParser::parse_piece_on_square_with_filter` | [`L18`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L18) |
| `fn` | `QueryParser::parse_piece_count_or_squares` | [`L266`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L266) |
| `fn` | `QueryParser::parse_pawn_color_opt` | [`L361`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L361) |
| `fn` | `QueryParser::parse_color_token` | [`L373`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L373) |
| `fn` | `QueryParser::parse_pawn_pred_passed` | [`L388`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L388) |
| `fn` | `QueryParser::parse_pawn_pred_isolated` | [`L399`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L399) |
| `fn` | `QueryParser::parse_pawn_pred_doubled` | [`L410`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L410) |
| `fn` | `QueryParser::parse_pawn_pred_backward` | [`L421`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L421) |
| `fn` | `QueryParser::parse_pawn_pred_islands` | [`L432`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L432) |
| `fn` | `QueryParser::parse_variable_domain` | [`L443`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L443) |
| `fn` | `QueryParser::parse_castling_filter` | [`L497`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L497) |

### [`src/search/parser/ranges.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/ranges.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `QueryParser::parse_ply_expr` | [`L8`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/ranges.rs#L8) |
| `fn` | `QueryParser::parse_move_number_expr` | [`L95`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/ranges.rs#L95) |
| `fn` | `QueryParser::parse_occurrences_expr` | [`L104`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/ranges.rs#L104) |

### [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `QueryParser::parse_square_set` | [`L21`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L21) |
| `fn` | `QueryParser::parse_direction_expression` | [`L154`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L154) |
| `fn` | `QueryParser::parse_diagonal_or_ray_expression` | [`L214`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L214) |
| `fn` | `QueryParser::parse_color_modified_piece` | [`L316`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L316) |
| `fn` | `QueryParser::parse_square_set_query` | [`L523`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L523) |
| `fn` | `QueryParser::parse_square_set_expr` | [`L563`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L563) |
| `fn` | `QueryParser::parse_square_set_union` | [`L567`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L567) |
| `fn` | `QueryParser::parse_square_set_diff` | [`L579`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L579) |
| `fn` | `QueryParser::parse_square_set_intersection` | [`L595`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L595) |
| `fn` | `QueryParser::is_juxtaposition_square_start` | [`L617`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L617) |
| `fn` | `QueryParser::parse_square_set_unary` | [`L636`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L636) |
| `fn` | `QueryParser::is_square_set_atom_start` | [`L719`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L719) |
| `fn` | `QueryParser::parse_square_set_atom` | [`L771`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L771) |
| `fn` | `QueryParser::is_bracket_piece_list` | [`L1018`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L1018) |

### [`src/search/parser/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `QueryParser::parse_piece_matcher_arg` | [`L12`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L12) |
| `fn` | `QueryParser::parse_pin_expr` | [`L40`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L40) |
| `fn` | `QueryParser::parse_fork_expr` | [`L180`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L180) |
| `fn` | `QueryParser::parse_skewer_expr` | [`L363`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L363) |
| `fn` | `QueryParser::parse_trapped_expr` | [`L503`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L503) |
| `fn` | `QueryParser::parse_outpost_expr` | [`L524`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L524) |
| `fn` | `QueryParser::parse_open_file_expr` | [`L586`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L586) |
| `fn` | `QueryParser::parse_distance_expr` | [`L642`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L642) |
| `fn` | `QueryParser::parse_attacks_expr` | [`L683`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L683) |
| `fn` | `QueryParser::parse_attacked_expr` | [`L763`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L763) |
| `fn` | `QueryParser::parse_piece_matcher_list` | [`L823`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L823) |

### [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `validate_query_semantics` | [`L13`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L13) |
| `fn` | `validate_square_map` | [`L54`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L54) |
| `fn` | `validate_and_clauses` | [`L107`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L107) |
| `struct` | `PieceCountAccumulator` | [`L758`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L758) |
| `fn` | `PieceCountAccumulator::add_piece_at_square` | [`L770`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L770) |
| `fn` | `PieceCountAccumulator::set_count` | [`L788`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L788) |
| `fn` | `PieceCountAccumulator::min_promotions_needed` | [`L799`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L799) |
| `fn` | `PieceCountAccumulator::total_pieces` | [`L811`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L811) |
| `fn` | `PieceCountAccumulator::validate` | [`L815`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L815) |
| `fn` | `validate_piece_count` | [`L915`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L915) |
| `fn` | `piece_directly_attacks` | [`L1036`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L1036) |
| `fn` | `get_move_color` | [`L1064`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L1064) |
| `fn` | `get_piece_color_from_content` | [`L1076`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L1076) |

### [`src/search/tests/adapters.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/adapters.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `test_zero_copy_scid_adapter` | [`L4`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/adapters.rs#L4) |
| `fn` | `test_alex_pgn_queries` | [`L45`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/adapters.rs#L45) |

### [`src/search/tests/dsl_and_validation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `test_composite_boolean_and_ply_range_queries` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L6) |
| `fn` | `test_cql_dsl_query_parser` | [`L63`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L63) |
| `fn` | `test_descriptive_parse_error_diagnostics` | [`L104`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L104) |
| `fn` | `test_query_comments_support` | [`L150`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L150) |
| `fn` | `test_query_explain_and_to_dsl` | [`L165`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L165) |
| `fn` | `test_advanced_move_separator_promotions_and_captures` | [`L209`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L209) |
| `fn` | `test_manual_examples_all_valid` | [`L355`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L355) |
| `fn` | `test_query_semantic_validation_contradictions` | [`L499`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L499) |
| `fn` | `test_parent_and_child_scoping` | [`L743`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L743) |

### [`src/search/tests/fixtures.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/fixtures.rs)

_No public structs or functions found._

### [`src/search/tests/geometry.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `test_multi_square_and_symmetry_transformations` | [`L7`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L7) |
| `fn` | `test_square_ranges_and_diagonal_expansion` | [`L31`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L31) |
| `fn` | `test_light_and_dark_square_and_bishop_filtering` | [`L94`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L94) |
| `fn` | `test_wtm_btm_any_color_and_legal_move_filters` | [`L158`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L158) |
| `fn` | `test_square_set_algebra_and_bitboard_engine` | [`L263`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L263) |
| `fn` | `test_cqli_direction_spatial_shifts_and_rotations` | [`L310`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L310) |
| `fn` | `test_bracket_set_comparisons` | [`L383`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L383) |

### [`src/search/tests/headers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/headers.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `test_header_metadata_search` | [`L5`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/headers.rs#L5) |
| `fn` | `test_tag_and_custom_headers` | [`L85`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/headers.rs#L85) |
| `fn` | `test_scid_index_entry_header_prefiltering` | [`L117`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/headers.rs#L117) |
| `fn` | `test_pgn_index_entry_header_prefiltering` | [`L185`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/headers.rs#L185) |
| `fn` | `test_comments_and_nag_annotations` | [`L259`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/headers.rs#L259) |

### [`src/search/tests/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/mod.rs)

_No public structs or functions found._

### [`src/search/tests/moves_and_paths.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `test_move_pattern_and_path_sequence_search` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L6) |
| `fn` | `test_path_piece_identifiers_search` | [`L70`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L70) |
| `fn` | `test_wildcard_moves_and_promotions_and_en_passant` | [`L108`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L108) |
| `fn` | `test_path_regex_quantifiers_and_consecutive_default` | [`L196`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L196) |
| `fn` | `test_previous_move_queries` | [`L243`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L243) |
| `fn` | `test_direction_filters_and_move_paths` | [`L303`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L303) |
| `fn` | `test_single_color_path_and_move_repetition_quantifiers` | [`L396`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L396) |
| `fn` | `test_cql_path_specification_and_interleaved_filters` | [`L439`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L439) |
| `fn` | `test_cql_line_specification_forward_backward_and_modifiers` | [`L475`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L475) |
| `fn` | `test_move_piece_targets_ply_movenumber_and_multiline_and` | [`L565`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L565) |
| `fn` | `test_move_capture_parameter_logic` | [`L635`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L635) |

### [`src/search/tests/positions.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `test_position_pattern_search` | [`L7`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L7) |
| `fn` | `test_pawn_structure_search` | [`L75`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L75) |
| `fn` | `test_wildcard_fen_and_casing_conventions` | [`L113`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L113) |
| `fn` | `test_power_expressions` | [`L148`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L148) |
| `fn` | `test_bare_piece_syntax_and_cql_piece_counts` | [`L201`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L201) |
| `fn` | `test_compact_piece_placements` | [`L243`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L243) |
| `fn` | `test_bracketed_piece_group_counts` | [`L268`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L268) |
| `fn` | `test_fen_transformations_and_symmetries` | [`L316`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L316) |
| `fn` | `test_position_and_move_or_path_combination` | [`L341`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L341) |

### [`src/search/tests/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `test_tactical_motifs_and_geometry` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L6) |
| `fn` | `test_mating_themes_catalog_parsing_and_evaluation` | [`L42`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L42) |
| `fn` | `test_pin_named_parameters` | [`L156`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L156) |
| `fn` | `test_variable_binding_and_fork_queries` | [`L181`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L181) |
| `fn` | `test_fork_exact_slots_and_piece_options` | [`L239`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L239) |
| `fn` | `test_legal_mate_queries` | [`L320`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L320) |
| `fn` | `test_play_and_leads_to_hypothetical_moves` | [`L353`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L353) |
| `fn` | `test_play_and_not_move_missed_mate_regression` | [`L447`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L447) |

### [`src/server/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/mod.rs)

| Kind | Name | Line |
|---|---|---|
| `enum` | `DatabaseBackend` | [`L14`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/mod.rs#L14) |
| `struct` | `RequestMessage` | [`L20`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/mod.rs#L20) |
| `struct` | `ResponseMessage` | [`L28`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/mod.rs#L28) |
| `fn` | `run_interactive_server` | [`L37`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/mod.rs#L37) |
| `fn` | `handle_command` | [`L137`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/mod.rs#L137) |

### [`src/server/handlers/config.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/config.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_set_threads` | [`L3`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/config.rs#L3) |
| `fn` | `handle_get_threads` | [`L40`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/config.rs#L40) |

### [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_open_db` | [`L9`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L9) |
| `fn` | `handle_create_db` | [`L172`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L172) |
| `fn` | `handle_info_stats` | [`L223`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L223) |
| `fn` | `handle_query_games` | [`L292`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L292) |
| `fn` | `handle_get_game_summaries` | [`L378`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L378) |
| `fn` | `handle_get_game_pgn` | [`L424`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L424) |
| `fn` | `handle_add_game` | [`L482`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L482) |
| `fn` | `handle_update_game` | [`L538`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L538) |
| `fn` | `handle_delete_game` | [`L608`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L608) |
| `fn` | `handle_undelete_game` | [`L668`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L668) |
| `fn` | `handle_compact` | [`L728`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L728) |
| `fn` | `handle_save` | [`L769`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L769) |
| `fn` | `handle_sort_database` | [`L813`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L813) |
| `fn` | `handle_sort_pgn` | [`L886`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L886) |

### [`src/server/handlers/import_export.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/import_export.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_import_pgn` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/import_export.rs#L6) |
| `fn` | `handle_export_pgn` | [`L79`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/import_export.rs#L79) |
| `fn` | `handle_benchmark` | [`L142`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/import_export.rs#L142) |

### [`src/server/handlers/index.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/index.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_unload_pos_index` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/index.rs#L6) |
| `fn` | `handle_pos_index_status` | [`L19`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/index.rs#L19) |
| `fn` | `handle_pos_index_diagnostics` | [`L62`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/index.rs#L62) |
| `fn` | `handle_build_pos_index` | [`L113`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/index.rs#L113) |

### [`src/server/handlers/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/mod.rs)

_No public structs or functions found._

### [`src/server/handlers/position.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/position.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_search_position` | [`L5`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/position.rs#L5) |
| `fn` | `handle_search_material` | [`L186`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/position.rs#L186) |

### [`src/server/handlers/search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/search.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_validate_dsl` | [`L7`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/search.rs#L7) |
| `fn` | `handle_explain_dsl` | [`L37`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/search.rs#L37) |
| `fn` | `handle_cql_search` | [`L70`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/search.rs#L70) |

### [`src/server/handlers/tree.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/tree.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `handle_opening_tree` | [`L11`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/tree.rs#L11) |
| `fn` | `handle_tree_index_status` | [`L303`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/tree.rs#L303) |
| `fn` | `handle_tree_index_diagnostics` | [`L346`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/tree.rs#L346) |
| `fn` | `handle_build_tree_index` | [`L397`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/tree.rs#L397) |

### [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `StripedTreePositionMap` | [`L26`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L26) |
| `fn` | `StripedTreePositionMap::new` | [`L32`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L32) |
| `fn` | `StripedTreePositionMap::stripe_index` | [`L44`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L44) |
| `fn` | `StripedTreePositionMap::record` | [`L49`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L49) |
| `fn` | `StripedTreePositionMap::total_positions` | [`L77`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L77) |
| `fn` | `StripedTreePositionMap::into_map` | [`L81`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L81) |
| `struct` | `PgnTreeStatsVisitor` | [`L102`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L102) |
| `fn` | `PgnTreeStatsVisitor::new` | [`L115`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L115) |
| `fn` | `pgn_reader::begin_game` | [`L141`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L141) |
| `fn` | `pgn_reader::begin_variation` | [`L146`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L146) |
| `fn` | `pgn_reader::san` | [`L150`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L150) |
| `fn` | `pgn_reader::end_game` | [`L187`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L187) |
| `fn` | `build_for_scid` | [`L197`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L197) |
| `fn` | `build_for_pgn` | [`L381`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L381) |
| `fn` | `write_static_binary_file` | [`L472`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L472) |

### [`src/tree_index/codec.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/codec.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `encode_tree_position_payload` | [`L16`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/codec.rs#L16) |
| `fn` | `decode_tree_position_payload` | [`L37`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/codec.rs#L37) |
| `fn` | `generate_tree_report` | [`L89`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/codec.rs#L89) |
| `fn` | `parse_target_position` | [`L161`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/codec.rs#L161) |
| `fn` | `write_varint` | [`L183`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/codec.rs#L183) |
| `fn` | `read_varint` | [`L192`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/codec.rs#L192) |

### [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs)

| Kind | Name | Line |
|---|---|---|
| `struct` | `TreeIndex` | [`L15`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L15) |
| `fn` | `TreeIndex::companion_path` | [`L23`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L23) |
| `fn` | `TreeIndex::check_status` | [`L43`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L43) |
| `fn` | `TreeIndex::load` | [`L100`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L100) |
| `fn` | `TreeIndex::index_entries` | [`L136`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L136) |
| `fn` | `TreeIndex::get_position` | [`L148`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L148) |
| `fn` | `TreeIndex::query_tree` | [`L170`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L170) |
| `fn` | `TreeIndex::query_tree_with_options` | [`L177`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L177) |
| `fn` | `TreeIndex::scan_diagnostics` | [`L187`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L187) |
| `fn` | `TreeIndex::calculate_tree_for_scid` | [`L223`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L223) |
| `fn` | `TreeIndex::calculate_tree_for_pgn` | [`L240`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L240) |
| `fn` | `TreeIndex::build_for_scid` | [`L252`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L252) |
| `fn` | `TreeIndex::build_for_pgn` | [`L269`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L269) |

### [`src/tree_index/dynamic.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs)

| Kind | Name | Line |
|---|---|---|
| `fn` | `calculate_tree_for_scid` | [`L16`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L16) |
| `fn` | `calculate_tree_for_pgn` | [`L142`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L142) |
| `struct` | `PgnSinglePositionVisitor` | [`L243`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L243) |
| `fn` | `PgnSinglePositionVisitor::new` | [`L259`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L259) |
| `fn` | `pgn_reader::begin_game` | [`L288`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L288) |
| `fn` | `pgn_reader::begin_variation` | [`L294`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L294) |
| `fn` | `pgn_reader::san` | [`L298`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L298) |
| `fn` | `pgn_reader::end_game` | [`L326`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L326) |

### [`src/tree_index/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/mod.rs)

_No public structs or functions found._

### [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs)

| Kind | Name | Line |
|---|---|---|
| `enum` | `IndexStatus` | [`L13`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L13) |
| `struct` | `TreeIndexDiagnostics` | [`L20`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L20) |
| `struct` | `PackedMove` | [`L37`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L37) |
| `fn` | `PackedMove::new` | [`L41`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L41) |
| `fn` | `PackedMove::from_square` | [`L55`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L55) |
| `fn` | `PackedMove::to_square` | [`L60`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L60) |
| `fn` | `PackedMove::promotion` | [`L65`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L65) |
| `fn` | `PackedMove::to_uci_string` | [`L75`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L75) |
| `fn` | `PackedMove::to_shakmaty_move` | [`L88`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L88) |
| `fn` | `PackedMove (impl From<&shakmaty::Move>)::from` | [`L101`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L101) |
| `struct` | `TreeIndexHeader` | [`L109`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L109) |
| `fn` | `TreeIndexHeader::read_from_slice` | [`L124`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L124) |
| `fn` | `TreeIndexHeader::write_to` | [`L159`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L159) |
| `struct` | `SortedTreeIndexEntry` | [`L176`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L176) |
| `struct` | `TreeMoveStats` | [`L182`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L182) |
| `fn` | `TreeMoveStats::avg_white_elo` | [`L194`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L194) |
| `fn` | `TreeMoveStats::avg_black_elo` | [`L202`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L202) |
| `struct` | `TreePositionNode` | [`L212`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L212) |
| `fn` | `TreePositionNode::new` | [`L222`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L222) |
| `fn` | `TreePositionNode::record_game` | [`L233`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L233) |
| `fn` | `TreePositionNode::merge` | [`L277`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L277) |
| `struct` | `OpeningTreeMoveView` | [`L304`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L304) |
| `struct` | `OpeningTreeReport` | [`L322`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L322) |

## 4. Python GUI Frontend (`scripts/gui/`)

### [`scripts/cql_search_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `CqlSearchStandaloneWindow` | [`L15`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L15) |
| `func` | `__init__` | [`L20`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L20) |
| `func` | `init_menus` | [`L38`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L38) |
| `func` | `show_about` | [`L88`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L88) |
| `func` | `init_backend` | [`L105`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L105) |
| `func` | `closeEvent` | [`L116`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L116) |
| `func` | `main` | [`L121`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L121) |

### [`scripts/index_codebase.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/index_codebase.py)

| Kind | Name | Line |
|---|---|---|
| `func` | `scan_rust_file` | [`L20`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/index_codebase.py#L20) |
| `func` | `scan_python_file` | [`L63`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/index_codebase.py#L63) |
| `func` | `scan_server_commands` | [`L88`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/index_codebase.py#L88) |
| `func` | `build_index` | [`L101`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/index_codebase.py#L101) |

### [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `PosIdxDevWorkbench` | [`L44`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L44) |
| `func` | `__init__` | [`L45`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L45) |
| `func` | `init_ui` | [`L60`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L60) |
| `func` | `start_backend` | [`L249`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L249) |
| `func` | `browse_db` | [`L266`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L266) |
| `func` | `open_database` | [`L277`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L277) |
| `func` | `open_build_dialog` | [`L284`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L284) |
| `func` | `reset_board` | [`L288`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L288) |
| `func` | `query_opening_tree` | [`L294`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L294) |
| `func` | `on_tree_resp` | [`L302`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L302) |
| `func` | `on_move_double_clicked` | [`L339`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L339) |
| `func` | `scan_diagnostics` | [`L348`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L348) |
| `func` | `on_diag_resp` | [`L354`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L354) |
| `func` | `run_filter_benchmark` | [`L413`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L413) |
| `func` | `on_bench_resp` | [`L424`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L424) |
| `func` | `on_backend_response` | [`L450`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L450) |
| `func` | `on_backend_error` | [`L464`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L464) |
| `func` | `closeEvent` | [`L467`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L467) |
| `func` | `main` | [`L477`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L477) |

### [`scripts/scid_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/scid_gui.py)

| Kind | Name | Line |
|---|---|---|
| `func` | `main` | [`L13`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/scid_gui.py#L13) |

### [`scripts/gui/__init__.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/__init__.py)

_No classes or functions found._

### [`scripts/gui/backend_client.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `BackendClient` | [`L12`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L12) |
| `func` | `__init__` | [`L23`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L23) |
| `func` | `_dispatch_callback` | [`L34`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L34) |
| `func` | `is_running` | [`L47`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L47) |
| `func` | `start` | [`L50`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L50) |
| `func` | `_read_stdout_loop` | [`L99`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L99) |
| `func` | `_write_stdin_loop` | [`L116`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L116) |
| `func` | `_read_stderr_loop` | [`L134`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L134) |
| `func` | `send_request` | [`L143`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L143) |
| `func` | `stop` | [`L165`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L165) |

### [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `MainWindow` | [`L20`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L20) |
| `func` | `__init__` | [`L25`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L25) |
| `func` | `init_ui` | [`L47`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L47) |
| `func` | `start_backend` | [`L118`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L118) |
| `func` | `stop_backend` | [`L134`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L134) |
| `func` | `open_database` | [`L138`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L138) |
| `func` | `create_database` | [`L142`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L142) |
| `func` | `refresh_database_info` | [`L151`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L151) |
| `func` | `compact_db` | [`L155`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L155) |
| `func` | `save_db` | [`L159`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L159) |
| `func` | `import_pgn` | [`L163`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L163) |
| `func` | `export_pgn` | [`L187`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L187) |
| `func` | `prompt_build_pos_index` | [`L208`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L208) |
| `func` | `on_search_applied` | [`L215`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L215) |
| `func` | `on_filters_cleared` | [`L222`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L222) |
| `func` | `on_game_selected` | [`L229`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L229) |
| `func` | `load_game_by_id` | [`L236`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L236) |
| `func` | `add_game_dialog` | [`L243`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L243) |
| `func` | `edit_game_dialog` | [`L253`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L253) |
| `func` | `delete_selected_game` | [`L264`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L264) |
| `func` | `undelete_selected_game` | [`L269`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L269) |
| `func` | `on_tab_changed` | [`L274`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L274) |
| `func` | `update_ui_disconnected` | [`L278`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L278) |
| `func` | `on_response_received` | [`L287`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L287) |
| `func` | `update_indexes_badge` | [`L452`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L452) |
| `func` | `on_process_error` | [`L456`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L456) |
| `func` | `on_process_stopped` | [`L460`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L460) |
| `func` | `closeEvent` | [`L464`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L464) |

### [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `VirtualScidTableModel` | [`L8`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L8) |
| `func` | `__init__` | [`L46`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L46) |
| `func` | `rowCount` | [`L58`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L58) |
| `func` | `columnCount` | [`L61`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L61) |
| `func` | `headerData` | [`L64`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L64) |
| `func` | `toggle_sort_column` | [`L76`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L76) |
| `func` | `data` | [`L90`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L90) |
| `func` | `_format_cell` | [`L115`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L115) |
| `func` | `get_game_at` | [`L149`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L149) |
| `func` | `request_chunks_for_range` | [`L158`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L158) |
| `func` | `_request_chunk` | [`L175`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L175) |
| `func` | `set_filters` | [`L184`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L184) |
| `func` | `invalidate_cache_and_reload` | [`L195`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L195) |
| `func` | `clear` | [`L204`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L204) |
| `func` | `on_backend_response` | [`L212`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L212) |

### [`scripts/gui/dialogs/__init__.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/__init__.py)

_No classes or functions found._

### [`scripts/gui/dialogs/add_edit_game_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/add_edit_game_dialog.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `AddEditGameDialog` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/add_edit_game_dialog.py#L6) |
| `func` | `__init__` | [`L7`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/add_edit_game_dialog.py#L7) |
| `func` | `get_pgn` | [`L42`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/add_edit_game_dialog.py#L42) |

### [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `AdvancedSearchDialog` | [`L26`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L26) |
| `func` | `__init__` | [`L35`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L35) |
| `func` | `update_tab_titles` | [`L525`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L525) |
| `func` | `select_all_categories` | [`L539`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L539) |
| `func` | `clear_all_categories` | [`L545`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L545) |
| `func` | `mark_info_modified` | [`L551`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L551) |
| `func` | `mark_pos_modified` | [`L555`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L555) |
| `func` | `mark_mat_modified` | [`L559`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L559) |
| `func` | `mark_cql_modified` | [`L563`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L563) |
| `func` | `on_cql_preset_changed` | [`L567`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L567) |
| `func` | `on_material_preset_changed` | [`L575`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L575) |
| `func` | `set_val` | [`L590`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L590) |
| `func` | `on_board_fen_changed` | [`L656`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L656) |
| `func` | `on_fen_text_edited` | [`L663`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L663) |
| `func` | `set_single_piece_demo` | [`L671`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L671) |
| `func` | `reset_all` | [`L677`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L677) |
| `func` | `load_filter` | [`L711`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L711) |
| `func` | `get_filter_dict` | [`L827`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L827) |
| `func` | `parse_val` | [`L885`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L885) |

### [`scripts/gui/dialogs/benchmark_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/benchmark_dialog.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `BenchmarkDialog` | [`L23`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/benchmark_dialog.py#L23) |
| `func` | `__init__` | [`L24`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/benchmark_dialog.py#L24) |
| `func` | `init_ui` | [`L35`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/benchmark_dialog.py#L35) |
| `func` | `run_benchmark` | [`L140`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/benchmark_dialog.py#L140) |
| `func` | `display_report` | [`L152`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/benchmark_dialog.py#L152) |
| `func` | `copy_report` | [`L204`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/benchmark_dialog.py#L204) |

### [`scripts/gui/dialogs/build_pos_index_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `BuildPosIndexDialog` | [`L18`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L18) |
| `func` | `__init__` | [`L21`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L21) |
| `func` | `update_progress` | [`L116`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L116) |
| `func` | `_on_backend_message` | [`L130`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L130) |
| `func` | `start_build` | [`L155`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L155) |
| `func` | `_run_next_task` | [`L182`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L182) |
| `func` | `_handle_task_complete` | [`L210`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L210) |
| `func` | `_all_tasks_completed` | [`L220`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L220) |
| `func` | `open_diagnostics` | [`L244`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L244) |
| `func` | `closeEvent` | [`L252`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L252) |

### [`scripts/gui/dialogs/columns_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/columns_dialog.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `ColumnsConfigDialog` | [`L14`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/columns_dialog.py#L14) |
| `func` | `__init__` | [`L17`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/columns_dialog.py#L17) |
| `func` | `select_all` | [`L69`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/columns_dialog.py#L69) |
| `func` | `reset_defaults` | [`L73`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/columns_dialog.py#L73) |
| `func` | `apply_changes` | [`L77`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/columns_dialog.py#L77) |

### [`scripts/gui/dialogs/new_db_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/new_db_dialog.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `NewDatabaseDialog` | [`L15`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/new_db_dialog.py#L15) |
| `func` | `__init__` | [`L16`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/new_db_dialog.py#L16) |
| `func` | `browse_path` | [`L54`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/new_db_dialog.py#L54) |
| `func` | `get_data` | [`L65`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/new_db_dialog.py#L65) |

### [`scripts/gui/dialogs/pos_idx_diagnostics_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/pos_idx_diagnostics_dialog.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `PosIdxDiagnosticsDialog` | [`L16`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/pos_idx_diagnostics_dialog.py#L16) |
| `func` | `__init__` | [`L19`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/pos_idx_diagnostics_dialog.py#L19) |
| `func` | `load_diagnostics` | [`L122`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/pos_idx_diagnostics_dialog.py#L122) |
| `func` | `on_diagnostics_received` | [`L132`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/pos_idx_diagnostics_dialog.py#L132) |
| `func` | `populate_data` | [`L141`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/pos_idx_diagnostics_dialog.py#L141) |

### [`scripts/gui/dialogs/search_progress_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/search_progress_dialog.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `SearchProgressDialog` | [`L13`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/search_progress_dialog.py#L13) |
| `func` | `__init__` | [`L19`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/search_progress_dialog.py#L19) |
| `func` | `update_progress` | [`L75`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/search_progress_dialog.py#L75) |
| `func` | `on_finished` | [`L89`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/search_progress_dialog.py#L89) |

### [`scripts/gui/dialogs/settings_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/settings_dialog.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `SettingsDialog` | [`L16`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/settings_dialog.py#L16) |
| `func` | `__init__` | [`L22`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/settings_dialog.py#L22) |
| `func` | `init_ui` | [`L35`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/settings_dialog.py#L35) |
| `func` | `update_cpu_hint` | [`L87`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/settings_dialog.py#L87) |
| `func` | `load_settings` | [`L100`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/settings_dialog.py#L100) |
| `func` | `save_settings` | [`L112`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/settings_dialog.py#L112) |

### [`scripts/gui/widgets/__init__.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/__init__.py)

_No classes or functions found._

### [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py)

| Kind | Name | Line |
|---|---|---|
| `func` | `get_piece_pixmap` | [`L19`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L19) |
| `class` | `SquareWidget` | [`L32`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L32) |
| `func` | `__init__` | [`L35`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L35) |
| `func` | `update_background` | [`L46`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L46) |
| `func` | `mousePressEvent` | [`L52`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L52) |
| `func` | `mouseMoveEvent` | [`L57`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L57) |
| `func` | `dragEnterEvent` | [`L82`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L82) |
| `func` | `dropEvent` | [`L86`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L86) |
| `class` | `ChessBoardEditorWidget` | [`L92`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L92) |
| `func` | `__init__` | [`L100`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L100) |
| `func` | `init_ui` | [`L111`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L111) |
| `func` | `create_toolbar` | [`L146`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L146) |
| `func` | `square_clicked` | [`L201`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L201) |
| `func` | `handle_drop` | [`L227`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L227) |
| `func` | `update_board_ui` | [`L236`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L236) |
| `func` | `clear_board` | [`L248`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L248) |
| `func` | `reset_to_initial` | [`L253`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L253) |
| `func` | `set_fen` | [`L258`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L258) |
| `func` | `get_board_fen` | [`L266`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L266) |

### [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `CqlExplainDialog` | [`L59`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L59) |
| `func` | `__init__` | [`L64`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L64) |
| `func` | `init_ui` | [`L71`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L71) |
| `class` | `CqlSearchWidget` | [`L186`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L186) |
| `func` | `__init__` | [`L193`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L193) |
| `func` | `init_ui` | [`L209`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L209) |
| `func` | `eventFilter` | [`L472`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L472) |
| `func` | `keyPressEvent` | [`L488`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L488) |
| `func` | `on_preset_selected` | [`L504`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L504) |
| `func` | `on_query_text_changed` | [`L510`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L510) |
| `func` | `on_source_mode_changed` | [`L514`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L514) |
| `func` | `browse_pgn_file` | [`L518`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L518) |
| `func` | `open_database_dialog` | [`L523`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L523) |
| `func` | `open_database` | [`L533`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L533) |
| `func` | `on_db_opened` | [`L544`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L544) |
| `func` | `explain_current_query` | [`L567`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L567) |
| `func` | `on_explain_done` | [`L577`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L577) |
| `func` | `validate_current_query` | [`L588`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L588) |
| `func` | `on_valid_resp` | [`L600`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L600) |
| `func` | `execute_search` | [`L631`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L631) |
| `func` | `on_search_done` | [`L659`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L659) |
| `func` | `on_backend_event` | [`L685`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L685) |
| `func` | `populate_results_table` | [`L697`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L697) |
| `func` | `clear_game_display` | [`L729`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L729) |
| `func` | `on_result_row_selected` | [`L742`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L742) |
| `func` | `on_pgn_loaded` | [`L764`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L764) |
| `func` | `display_game` | [`L771`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L771) |
| `func` | `jump_to_match_ply` | [`L803`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L803) |
| `func` | `set_ply` | [`L807`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L807) |
| `func` | `update_legal_moves_display` | [`L830`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L830) |
| `func` | `move_sort_key` | [`L839`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L839) |
| `func` | `on_legal_move_clicked` | [`L921`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L921) |
| `func` | `go_first_move` | [`L930`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L930) |
| `func` | `go_prev_move` | [`L933`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L933) |
| `func` | `go_next_move` | [`L936`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L936) |
| `func` | `go_last_move` | [`L939`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L939) |
| `func` | `update_board_display` | [`L943`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L943) |

### [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `DatabaseControlWidget` | [`L17`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L17) |
| `func` | `__init__` | [`L33`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L33) |
| `func` | `init_ui` | [`L45`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L45) |
| `func` | `auto_detect_defaults` | [`L170`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L170) |
| `func` | `browse_binary` | [`L176`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L176) |
| `func` | `browse_db` | [`L185`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L185) |
| `func` | `create_new_db` | [`L196`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L196) |
| `func` | `toggle_backend` | [`L205`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L205) |
| `func` | `on_import_pgn` | [`L223`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L223) |
| `func` | `on_export_pgn` | [`L231`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L231) |
| `func` | `open_settings_dialog` | [`L239`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L239) |
| `func` | `open_pos_idx_diagnostics_dialog` | [`L243`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L243) |
| `func` | `open_benchmark_dialog` | [`L250`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L250) |
| `func` | `prompt_build_pos_index` | [`L257`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L257) |
| `func` | `update_ui_connected` | [`L264`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L264) |
| `func` | `update_ui_disconnected` | [`L270`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L270) |
| `func` | `update_stats` | [`L284`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L284) |
| `func` | `update_indexes_badge` | [`L294`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L294) |

### [`scripts/gui/widgets/filter_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `FilterPanelWidget` | [`L11`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L11) |
| `func` | `__init__` | [`L19`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L19) |
| `func` | `init_ui` | [`L25`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L25) |
| `func` | `get_filter_dict` | [`L121`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L121) |
| `func` | `on_search_clicked` | [`L145`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L145) |
| `func` | `reset_filters` | [`L148`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L148) |
| `func` | `open_advanced_search` | [`L165`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L165) |

### [`scripts/gui/widgets/game_preview_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `GamePreviewPanelWidget` | [`L11`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L11) |
| `func` | `__init__` | [`L21`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L21) |
| `func` | `init_ui` | [`L26`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L26) |
| `func` | `set_selected_game` | [`L66`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L66) |
| `func` | `set_pgn_text` | [`L77`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L77) |
| `func` | `get_pgn_text` | [`L80`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L80) |
| `func` | `copy_pgn_text` | [`L83`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L83) |
| `func` | `clear` | [`L88`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L88) |

### [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `GameTablePanelWidget` | [`L12`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L12) |
| `func` | `__init__` | [`L20`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L20) |
| `func` | `init_ui` | [`L33`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L33) |
| `func` | `set_default_column_widths` | [`L77`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L77) |
| `func` | `show_header_context_menu` | [`L82`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L82) |
| `func` | `toggle_column_visibility` | [`L109`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L109) |
| `func` | `show_all_columns` | [`L113`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L113) |
| `func` | `open_columns_dialog` | [`L118`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L118) |
| `func` | `save_column_settings` | [`L122`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L122) |
| `func` | `load_column_settings` | [`L127`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L127) |
| `func` | `on_model_stats_updated` | [`L139`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L139) |
| `func` | `_on_scroll_changed` | [`L144`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L144) |
| `func` | `_on_scroll_settled` | [`L147`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L147) |
| `func` | `on_table_selection_changed` | [`L159`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L159) |

### [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `OpeningTreeWidget` | [`L23`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L23) |
| `func` | `__init__` | [`L31`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L31) |
| `func` | `init_ui` | [`L41`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L41) |
| `func` | `_update_table_headers` | [`L209`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L209) |
| `func` | `on_last_played_toggled` | [`L217`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L217) |
| `func` | `update_tree_index_badge` | [`L222`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L222) |
| `func` | `on_scope_changed` | [`L252`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L252) |
| `func` | `refresh_current_position` | [`L258`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L258) |
| `func` | `go_to_start` | [`L276`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L276) |
| `func` | `go_back` | [`L281`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L281) |
| `func` | `play_move_san` | [`L287`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L287) |
| `func` | `on_row_double_clicked` | [`L296`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L296) |
| `func` | `on_tree_selection_changed` | [`L303`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L303) |
| `func` | `on_sample_game_double_clicked` | [`L328`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L328) |
| `func` | `on_game_summaries_received` | [`L338`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L338) |
| `func` | `_populate_sample_games` | [`L341`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L341) |
| `func` | `_update_history_label` | [`L372`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L372) |
| `func` | `on_tree_report` | [`L384`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L384) |
| `func` | `_render_tree_table` | [`L406`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L406) |
| `func` | `unload_index` | [`L454`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L454) |

### [`scripts/gui/widgets/protocol_log_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py)

| Kind | Name | Line |
|---|---|---|
| `class` | `ProtocolLogPanelWidget` | [`L6`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L6) |
| `func` | `__init__` | [`L11`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L11) |
| `func` | `init_ui` | [`L15`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L15) |
| `func` | `clear_logs` | [`L35`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L35) |
| `func` | `append_message` | [`L38`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L38) |
| `func` | `append_json_payload` | [`L41`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L41) |
| `func` | `_sanitize_payload` | [`L52`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L52) |

## 5. Alphabetical Symbol Quick-Find

| Symbol Name | Kind | File | Line |
|---|---|---|---|
| `__init__` | `func` | [`scripts/cql_search_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L20) | L20 |
| `__init__` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L45) | L45 |
| `__init__` | `func` | [`scripts/gui/backend_client.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L23) | L23 |
| `__init__` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L25) | L25 |
| `__init__` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L46) | L46 |
| `__init__` | `func` | [`scripts/gui/dialogs/add_edit_game_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/add_edit_game_dialog.py#L7) | L7 |
| `__init__` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L35) | L35 |
| `__init__` | `func` | [`scripts/gui/dialogs/benchmark_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/benchmark_dialog.py#L24) | L24 |
| `__init__` | `func` | [`scripts/gui/dialogs/build_pos_index_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L21) | L21 |
| `__init__` | `func` | [`scripts/gui/dialogs/columns_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/columns_dialog.py#L17) | L17 |
| `__init__` | `func` | [`scripts/gui/dialogs/new_db_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/new_db_dialog.py#L16) | L16 |
| `__init__` | `func` | [`scripts/gui/dialogs/pos_idx_diagnostics_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/pos_idx_diagnostics_dialog.py#L19) | L19 |
| `__init__` | `func` | [`scripts/gui/dialogs/search_progress_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/search_progress_dialog.py#L19) | L19 |
| `__init__` | `func` | [`scripts/gui/dialogs/settings_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/settings_dialog.py#L22) | L22 |
| `__init__` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L35) | L35 |
| `__init__` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L100) | L100 |
| `__init__` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L64) | L64 |
| `__init__` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L193) | L193 |
| `__init__` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L33) | L33 |
| `__init__` | `func` | [`scripts/gui/widgets/filter_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L19) | L19 |
| `__init__` | `func` | [`scripts/gui/widgets/game_preview_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L21) | L21 |
| `__init__` | `func` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L20) | L20 |
| `__init__` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L31) | L31 |
| `__init__` | `func` | [`scripts/gui/widgets/protocol_log_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L11) | L11 |
| `_all_tasks_completed` | `func` | [`scripts/gui/dialogs/build_pos_index_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L220) | L220 |
| `_dispatch_callback` | `func` | [`scripts/gui/backend_client.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L34) | L34 |
| `_format_cell` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L115) | L115 |
| `_handle_task_complete` | `func` | [`scripts/gui/dialogs/build_pos_index_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L210) | L210 |
| `_on_backend_message` | `func` | [`scripts/gui/dialogs/build_pos_index_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L130) | L130 |
| `_on_scroll_changed` | `func` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L144) | L144 |
| `_on_scroll_settled` | `func` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L147) | L147 |
| `_populate_sample_games` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L341) | L341 |
| `_read_stderr_loop` | `func` | [`scripts/gui/backend_client.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L134) | L134 |
| `_read_stdout_loop` | `func` | [`scripts/gui/backend_client.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L99) | L99 |
| `_render_tree_table` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L406) | L406 |
| `_request_chunk` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L175) | L175 |
| `_run_next_task` | `func` | [`scripts/gui/dialogs/build_pos_index_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L182) | L182 |
| `_sanitize_payload` | `func` | [`scripts/gui/widgets/protocol_log_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L52) | L52 |
| `_update_history_label` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L372) | L372 |
| `_update_table_headers` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L209) | L209 |
| `_write_stdin_loop` | `func` | [`scripts/gui/backend_client.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L116) | L116 |
| `add_game_dialog` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L243) | L243 |
| `AddEditGameDialog` | `class` | [`scripts/gui/dialogs/add_edit_game_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/add_edit_game_dialog.py#L6) | L6 |
| `AdvancedSearchDialog` | `class` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L26) | L26 |
| `AnnotationManager` | `struct` | [`src/search/annotation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L118) | L118 |
| `AnnotationManager::extract_comments` | `fn` | [`src/search/annotation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L144) | L144 |
| `AnnotationManager::strip_comments` | `fn` | [`src/search/annotation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L122) | L122 |
| `AnnotationPredicate` | `enum` | [`src/search/annotation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L3) | L3 |
| `AnnotationPredicate (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L981) | L981 |
| `append_json_payload` | `func` | [`scripts/gui/widgets/protocol_log_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L41) | L41 |
| `append_message` | `func` | [`scripts/gui/widgets/protocol_log_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L38) | L38 |
| `apply_changes` | `func` | [`scripts/gui/dialogs/columns_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/columns_dialog.py#L77) | L77 |
| `auto_detect_defaults` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L170) | L170 |
| `BackendClient` | `class` | [`scripts/gui/backend_client.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L12) | L12 |
| `BenchmarkDialog` | `class` | [`scripts/gui/dialogs/benchmark_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/benchmark_dialog.py#L23) | L23 |
| `BenchmarkItem` | `struct` | [`src/benchmark.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/benchmark.rs#L9) | L9 |
| `BenchmarkReport` | `struct` | [`src/benchmark.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/benchmark.rs#L18) | L18 |
| `BoardSymmetry` | `enum` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L11) | L11 |
| `BoardSymmetry::expand` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L38) | L38 |
| `BoardSymmetry::transform_castling_rights` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L265) | L265 |
| `BoardSymmetry::transform_cql_line_pattern` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L781) | L781 |
| `BoardSymmetry::transform_cql_path_constituent` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L836) | L836 |
| `BoardSymmetry::transform_cql_path_pattern` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L812) | L812 |
| `BoardSymmetry::transform_direction` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L945) | L945 |
| `BoardSymmetry::transform_fen` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L158) | L158 |
| `BoardSymmetry::transform_header_predicate` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L1016) | L1016 |
| `BoardSymmetry::transform_material_predicate` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L517) | L517 |
| `BoardSymmetry::transform_move_pattern` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L1072) | L1072 |
| `BoardSymmetry::transform_piece` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L101) | L101 |
| `BoardSymmetry::transform_piece_matcher` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L112) | L112 |
| `BoardSymmetry::transform_piece_placement` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L207) | L207 |
| `BoardSymmetry::transform_position_pattern` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L317) | L317 |
| `BoardSymmetry::transform_query` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L587) | L587 |
| `BoardSymmetry::transform_set_predicate` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L867) | L867 |
| `BoardSymmetry::transform_square` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L79) | L79 |
| `BoardSymmetry::transform_square_content` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L136) | L136 |
| `BoardSymmetry::transform_square_or_piece` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L126) | L126 |
| `BoardSymmetry::transform_square_set_expr` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L886) | L886 |
| `BoardSymmetry::transform_tactical_predicate` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L395) | L395 |
| `browse_binary` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L176) | L176 |
| `browse_db` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L266) | L266 |
| `browse_db` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L185) | L185 |
| `browse_path` | `func` | [`scripts/gui/dialogs/new_db_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/new_db_dialog.py#L54) | L54 |
| `browse_pgn_file` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L518) | L518 |
| `build_for_pgn` | `fn` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L307) | L307 |
| `build_for_pgn` | `fn` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L381) | L381 |
| `build_for_scid` | `fn` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L151) | L151 |
| `build_for_scid` | `fn` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L197) | L197 |
| `build_index` | `func` | [`scripts/index_codebase.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/index_codebase.py#L101) | L101 |
| `BuildCommands` | `enum` | [`src/cli/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/mod.rs#L30) | L30 |
| `BuildPosIndexDialog` | `class` | [`scripts/gui/dialogs/build_pos_index_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L18) | L18 |
| `calculate_line_col_snippet` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L59) | L59 |
| `calculate_power` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L460) | L460 |
| `calculate_tree_for_pgn` | `fn` | [`src/tree_index/dynamic.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L142) | L142 |
| `calculate_tree_for_scid` | `fn` | [`src/tree_index/dynamic.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L16) | L16 |
| `chebyshev_distance` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L675) | L675 |
| `check_opposite_bishops` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L495) | L495 |
| `check_same_colored_bishops` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L505) | L505 |
| `ChessBoardEditorWidget` | `class` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L92) | L92 |
| `clear` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L204) | L204 |
| `clear` | `func` | [`scripts/gui/widgets/game_preview_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L88) | L88 |
| `clear_all_categories` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L545) | L545 |
| `clear_board` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L248) | L248 |
| `clear_game_display` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L729) | L729 |
| `clear_logs` | `func` | [`scripts/gui/widgets/protocol_log_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L35) | L35 |
| `Cli` | `struct` | [`src/cli/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/mod.rs#L12) | L12 |
| `closeEvent` | `func` | [`scripts/cql_search_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L116) | L116 |
| `closeEvent` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L467) | L467 |
| `closeEvent` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L464) | L464 |
| `closeEvent` | `func` | [`scripts/gui/dialogs/build_pos_index_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L252) | L252 |
| `collect_fens` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L1124) | L1124 |
| `columnCount` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L61) | L61 |
| `ColumnsConfigDialog` | `class` | [`scripts/gui/dialogs/columns_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/columns_dialog.py#L14) | L14 |
| `Commands` | `enum` | [`src/cli/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/mod.rs#L97) | L97 |
| `CommentPredicate` | `enum` | [`src/search/annotation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L12) | L12 |
| `CommentPredicate::matches` | `fn` | [`src/search/annotation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L30) | L30 |
| `compact_db` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L155) | L155 |
| `CompactPgnRecord` | `struct` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L9) | L9 |
| `CompactPgnRecord::date_str` | `fn` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L33) | L33 |
| `CompactPgnRecord::eco_str` | `fn` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L38) | L38 |
| `CompactPgnRecord::result_str` | `fn` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L28) | L28 |
| `compare_date` | `fn` | [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L744) | L744 |
| `compare_distance` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L734) | L734 |
| `compare_i32` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L527) | L527 |
| `compare_numeric` | `fn` | [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L642) | L642 |
| `compare_string` | `fn` | [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L659) | L659 |
| `compare_usize` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L515) | L515 |
| `compare_usize` | `fn` | [`src/search/pawn.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L285) | L285 |
| `ComparisonOp` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L91) | L91 |
| `ComparisonOp (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L18) | L18 |
| `ComparisonOp::invert` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L105) | L105 |
| `contains_symmetry` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L1155) | L1155 |
| `copy_pgn_text` | `func` | [`scripts/gui/widgets/game_preview_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L83) | L83 |
| `copy_report` | `func` | [`scripts/gui/dialogs/benchmark_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/benchmark_dialog.py#L204) | L204 |
| `count_square_content` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L420) | L420 |
| `CqlExplainDialog` | `class` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L59) | L59 |
| `CqlLineMatcher` | `struct` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L1049) | L1049 |
| `CqlLineMatcher::match_cql_line` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L1053) | L1053 |
| `CqlLinePattern` | `struct` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L456) | L456 |
| `CqlLinePattern (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L669) | L669 |
| `CqlLinePattern::requires_san_strings` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L470) | L470 |
| `CqlPathConstituent` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L414) | L414 |
| `CqlPathConstituent (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L608) | L608 |
| `CqlPathConstituent::requires_san_strings` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L476) | L476 |
| `CqlPathMatcher` | `struct` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L736) | L736 |
| `CqlPathMatcher::match_cql_path` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L740) | L740 |
| `CqlPathPattern` | `struct` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L431) | L431 |
| `CqlPathPattern (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L642) | L642 |
| `CqlPathPattern::requires_san_strings` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L441) | L441 |
| `CqlSearchStandaloneWindow` | `class` | [`scripts/cql_search_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L15) | L15 |
| `CqlSearchWidget` | `class` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L186) | L186 |
| `create_database` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L142) | L142 |
| `create_new_db` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L196) | L196 |
| `create_toolbar` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L146) | L146 |
| `data` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L90) | L90 |
| `DatabaseBackend` | `enum` | [`src/server/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/mod.rs#L14) | L14 |
| `DatabaseCheckReport` | `struct` | [`src/cli/commands/check.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/check.rs#L10) | L10 |
| `DatabaseControlWidget` | `class` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L17) | L17 |
| `DatabaseSession` | `enum` | [`src/cli/commands/repl.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L11) | L11 |
| `DatabaseSession::file_name` | `fn` | [`src/cli/commands/repl.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L37) | L37 |
| `DatabaseSession::format_name` | `fn` | [`src/cli/commands/repl.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L52) | L52 |
| `DatabaseSession::game_count` | `fn` | [`src/cli/commands/repl.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L30) | L30 |
| `DatabaseSession::get_pgn` | `fn` | [`src/cli/commands/repl.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L93) | L93 |
| `DatabaseSession::open` | `fn` | [`src/cli/commands/repl.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L17) | L17 |
| `DatabaseSession::print_info` | `fn` | [`src/cli/commands/repl.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L100) | L100 |
| `DatabaseSession::search` | `fn` | [`src/cli/commands/repl.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L59) | L59 |
| `DbStats` | `struct` | [`src/db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L105) | L105 |
| `decode_extra_tags` | `fn` | [`src/position_search/decoder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/decoder.rs#L33) | L33 |
| `decode_position_game_ids` | `fn` | [`src/position_index/codec.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/codec.rs#L28) | L28 |
| `decode_raw_move` | `fn` | [`src/position_search/decoder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/decoder.rs#L172) | L172 |
| `decode_tree_position_payload` | `fn` | [`src/tree_index/codec.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/codec.rs#L37) | L37 |
| `delete_selected_game` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L264) | L264 |
| `detect_format_from_path` | `fn` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L31) | L31 |
| `Direction` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L257) | L257 |
| `Direction::delta_vectors` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L274) | L274 |
| `Direction::expand_square` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L301) | L301 |
| `Direction::expand_squares` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L320) | L320 |
| `display_game` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L771) | L771 |
| `display_report` | `func` | [`scripts/gui/dialogs/benchmark_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/benchmark_dialog.py#L152) | L152 |
| `dragEnterEvent` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L82) | L82 |
| `dropEvent` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L86) | L86 |
| `edit_game_dialog` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L253) | L253 |
| `encode_posting_payload` | `fn` | [`src/position_index/codec.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/codec.rs#L13) | L13 |
| `encode_rook_like` | `fn` | [`src/pgn_io/encoder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/encoder.rs#L19) | L19 |
| `encode_scid_move_byte` | `fn` | [`src/pgn_io/encoder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/encoder.rs#L30) | L30 |
| `encode_tree_position_payload` | `fn` | [`src/tree_index/codec.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/codec.rs#L16) | L16 |
| `evaluate_pgn_streaming` | `fn` | [`src/search/evaluator/replayer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/replayer.rs#L53) | L53 |
| `evaluate_with_timeline_env` | `fn` | [`src/search/evaluator/matcher.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs#L14) | L14 |
| `eventFilter` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L472) | L472 |
| `execute_search` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L631) | L631 |
| `expand_diagonal_ray` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L205) | L205 |
| `expand_rectangular_range` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L189) | L189 |
| `expand_square_specifier` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L233) | L233 |
| `explain_current_query` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L567) | L567 |
| `explain_query` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L1184) | L1184 |
| `export_pgn` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L187) | L187 |
| `export_pgn_file` | `fn` | [`src/pgn_io/export.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/export.rs#L214) | L214 |
| `export_pgn_ultra_fast` | `fn` | [`src/pgn_io/export.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/export.rs#L133) | L133 |
| `ExportProgress` | `struct` | [`src/pgn_io/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L17) | L17 |
| `fast_game_to_pgn` | `fn` | [`src/pgn_io/export.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/export.rs#L13) | L13 |
| `FastNameTables` | `struct` | [`src/pgn_io/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L40) | L40 |
| `FastNameTables::event_id` | `fn` | [`src/pgn_io/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L98) | L98 |
| `FastNameTables::from_name_tables` | `fn` | [`src/pgn_io/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L52) | L52 |
| `FastNameTables::player_id` | `fn` | [`src/pgn_io/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L86) | L86 |
| `FastNameTables::round_id` | `fn` | [`src/pgn_io/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L122) | L122 |
| `FastNameTables::site_id` | `fn` | [`src/pgn_io/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L110) | L110 |
| `FastNameTables::to_name_tables` | `fn` | [`src/pgn_io/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L133) | L133 |
| `file_char_to_idx` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L344) | L344 |
| `FilterPanelWidget` | `class` | [`scripts/gui/widgets/filter_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L11) | L11 |
| `GameFilter` | `struct` | [`src/db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L39) | L39 |
| `GameFilter::is_empty` | `fn` | [`src/db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L85) | L85 |
| `GameFilter::same_search_criteria` | `fn` | [`src/db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L64) | L64 |
| `GamePreviewPanelWidget` | `class` | [`scripts/gui/widgets/game_preview_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L11) | L11 |
| `GameSearchEvaluator` | `struct` | [`src/search/evaluator/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L29) | L29 |
| `GameSearchEvaluator::evaluate_game` | `fn` | [`src/search/evaluator/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L90) | L90 |
| `GameSearchEvaluator::evaluate_pgn` | `fn` | [`src/search/evaluator/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L33) | L33 |
| `GameSearchEvaluator::evaluate_pgn_detailed` | `fn` | [`src/search/evaluator/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L68) | L68 |
| `GameSearchEvaluator::evaluate_with_timeline` | `fn` | [`src/search/evaluator/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L121) | L121 |
| `GameSearchEvaluator::evaluate_with_timeline_env` | `fn` | [`src/search/evaluator/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L131) | L131 |
| `GameSummary` | `struct` | [`src/db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L20) | L20 |
| `GameTablePanelWidget` | `class` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L12) | L12 |
| `generate_smart_help` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L90) | L90 |
| `generate_tree_report` | `fn` | [`src/tree_index/codec.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/codec.rs#L89) | L89 |
| `get_board_fen` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L266) | L266 |
| `get_data` | `func` | [`scripts/gui/dialogs/new_db_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/new_db_dialog.py#L65) | L65 |
| `get_filter_dict` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L827) | L827 |
| `get_filter_dict` | `func` | [`scripts/gui/widgets/filter_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L121) | L121 |
| `get_game_at` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L149) | L149 |
| `get_header_value` | `fn` | [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L624) | L624 |
| `get_move_color` | `fn` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L1064) | L1064 |
| `get_pgn` | `func` | [`scripts/gui/dialogs/add_edit_game_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/add_edit_game_dialog.py#L42) | L42 |
| `get_pgn_text` | `func` | [`scripts/gui/widgets/game_preview_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L80) | L80 |
| `get_piece_color_from_content` | `fn` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L1076) | L1076 |
| `get_piece_pixmap` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L19) | L19 |
| `go_back` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L281) | L281 |
| `go_first_move` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L930) | L930 |
| `go_last_move` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L939) | L939 |
| `go_next_move` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L936) | L936 |
| `go_prev_move` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L933) | L933 |
| `go_to_start` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L276) | L276 |
| `handle_add_game` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L482) | L482 |
| `handle_bench` | `fn` | [`src/cli/commands/bench.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/bench.rs#L5) | L5 |
| `handle_benchmark` | `fn` | [`src/server/handlers/import_export.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/import_export.rs#L142) | L142 |
| `handle_build_all` | `fn` | [`src/cli/commands/index.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/index.rs#L74) | L74 |
| `handle_build_pos_idx` | `fn` | [`src/cli/commands/index.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/index.rs#L7) | L7 |
| `handle_build_pos_index` | `fn` | [`src/server/handlers/index.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/index.rs#L113) | L113 |
| `handle_build_tree` | `fn` | [`src/cli/commands/tree.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/tree.rs#L7) | L7 |
| `handle_build_tree_index` | `fn` | [`src/server/handlers/tree.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/tree.rs#L397) | L397 |
| `handle_check` | `fn` | [`src/cli/commands/check.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/check.rs#L39) | L39 |
| `handle_command` | `fn` | [`src/server/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/mod.rs#L137) | L137 |
| `handle_compact` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L728) | L728 |
| `handle_cql_search` | `fn` | [`src/server/handlers/search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/search.rs#L70) | L70 |
| `handle_create` | `fn` | [`src/cli/commands/import_export.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/import_export.rs#L69) | L69 |
| `handle_create_db` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L172) | L172 |
| `handle_delete_game` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L608) | L608 |
| `handle_drop` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L227) | L227 |
| `handle_explain` | `fn` | [`src/cli/commands/search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/search.rs#L155) | L155 |
| `handle_explain_dsl` | `fn` | [`src/server/handlers/search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/search.rs#L37) | L37 |
| `handle_export` | `fn` | [`src/cli/commands/import_export.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/import_export.rs#L46) | L46 |
| `handle_export_pgn` | `fn` | [`src/server/handlers/import_export.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/import_export.rs#L79) | L79 |
| `handle_get` | `fn` | [`src/cli/commands/search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/search.rs#L350) | L350 |
| `handle_get_game_pgn` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L424) | L424 |
| `handle_get_game_summaries` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L378) | L378 |
| `handle_get_threads` | `fn` | [`src/server/handlers/config.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/config.rs#L40) | L40 |
| `handle_import` | `fn` | [`src/cli/commands/import_export.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/import_export.rs#L6) | L6 |
| `handle_import_pgn` | `fn` | [`src/server/handlers/import_export.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/import_export.rs#L6) | L6 |
| `handle_info` | `fn` | [`src/cli/commands/info.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/info.rs#L6) | L6 |
| `handle_info_stats` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L223) | L223 |
| `handle_list` | `fn` | [`src/cli/commands/list.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/list.rs#L7) | L7 |
| `handle_open_db` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L9) | L9 |
| `handle_opening_tree` | `fn` | [`src/server/handlers/tree.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/tree.rs#L11) | L11 |
| `handle_pos_index_diagnostics` | `fn` | [`src/server/handlers/index.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/index.rs#L62) | L62 |
| `handle_pos_index_status` | `fn` | [`src/server/handlers/index.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/index.rs#L19) | L19 |
| `handle_query_games` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L292) | L292 |
| `handle_repl` | `fn` | [`src/cli/commands/repl.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L126) | L126 |
| `handle_save` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L769) | L769 |
| `handle_search` | `fn` | [`src/cli/commands/search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/search.rs#L8) | L8 |
| `handle_search_mat` | `fn` | [`src/cli/commands/search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/search.rs#L253) | L253 |
| `handle_search_material` | `fn` | [`src/server/handlers/position.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/position.rs#L186) | L186 |
| `handle_search_pos` | `fn` | [`src/cli/commands/search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/search.rs#L195) | L195 |
| `handle_search_position` | `fn` | [`src/server/handlers/position.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/position.rs#L5) | L5 |
| `handle_set_threads` | `fn` | [`src/server/handlers/config.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/config.rs#L3) | L3 |
| `handle_sort_database` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L813) | L813 |
| `handle_sort_db` | `fn` | [`src/cli/commands/sort.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/sort.rs#L30) | L30 |
| `handle_sort_pgn` | `fn` | [`src/cli/commands/sort.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/sort.rs#L6) | L6 |
| `handle_sort_pgn` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L886) | L886 |
| `handle_tree` | `fn` | [`src/cli/commands/tree.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/tree.rs#L72) | L72 |
| `handle_tree_index_diagnostics` | `fn` | [`src/server/handlers/tree.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/tree.rs#L346) | L346 |
| `handle_tree_index_status` | `fn` | [`src/server/handlers/tree.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/tree.rs#L303) | L303 |
| `handle_undelete_game` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L668) | L668 |
| `handle_unload_pos_index` | `fn` | [`src/server/handlers/index.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/index.rs#L6) | L6 |
| `handle_update_game` | `fn` | [`src/server/handlers/db.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/db.rs#L538) | L538 |
| `handle_validate_dsl` | `fn` | [`src/server/handlers/search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/handlers/search.rs#L7) | L7 |
| `headerData` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L64) | L64 |
| `HeaderMatcher` | `struct` | [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L5) | L5 |
| `HeaderMatcher::matches` | `fn` | [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L9) | L9 |
| `HeaderMatcher::matches_entry` | `fn` | [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L133) | L133 |
| `HeaderMatcher::matches_pgn_entry` | `fn` | [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L387) | L387 |
| `HeaderPredicate` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L118) | L118 |
| `HeaderPredicate (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L211) | L211 |
| `import_pgn` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L163) | L163 |
| `import_pgn_file_with_progress` | `fn` | [`src/pgn_io/import.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/import.rs#L481) | L481 |
| `import_pgn_ultra_fast` | `fn` | [`src/pgn_io/import.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/import.rs#L279) | L279 |
| `ImportProgress` | `struct` | [`src/pgn_io/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L6) | L6 |
| `IndexCheckReport` | `struct` | [`src/cli/commands/check.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/check.rs#L25) | L25 |
| `IndexDiagnostics` | `struct` | [`src/position_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L122) | L122 |
| `IndexStatus` | `enum` | [`src/position_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L13) | L13 |
| `IndexStatus` | `enum` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L13) | L13 |
| `init_backend` | `func` | [`scripts/cql_search_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L105) | L105 |
| `init_menus` | `func` | [`scripts/cql_search_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L38) | L38 |
| `init_ui` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L60) | L60 |
| `init_ui` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L47) | L47 |
| `init_ui` | `func` | [`scripts/gui/dialogs/benchmark_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/benchmark_dialog.py#L35) | L35 |
| `init_ui` | `func` | [`scripts/gui/dialogs/settings_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/settings_dialog.py#L35) | L35 |
| `init_ui` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L111) | L111 |
| `init_ui` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L71) | L71 |
| `init_ui` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L209) | L209 |
| `init_ui` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L45) | L45 |
| `init_ui` | `func` | [`scripts/gui/widgets/filter_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L25) | L25 |
| `init_ui` | `func` | [`scripts/gui/widgets/game_preview_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L26) | L26 |
| `init_ui` | `func` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L33) | L33 |
| `init_ui` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L41) | L41 |
| `init_ui` | `func` | [`scripts/gui/widgets/protocol_log_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L15) | L15 |
| `invalidate_cache_and_reload` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L195) | L195 |
| `is_ident_char` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L470) | L470 |
| `is_ident_start` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L466) | L466 |
| `is_running` | `func` | [`scripts/gui/backend_client.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L47) | L47 |
| `jump_to_match_ply` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L803) | L803 |
| `keyPressEvent` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L488) | L488 |
| `Lexer` | `struct` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L162) | L162 |
| `Lexer::advance` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L189) | L189 |
| `Lexer::current_pos` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L177) | L177 |
| `Lexer::is_next_digit` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L457) | L457 |
| `Lexer::new` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L169) | L169 |
| `Lexer::peek` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L185) | L185 |
| `Lexer::tokenize` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L199) | L199 |
| `LineDirection` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L448) | L448 |
| `load_column_settings` | `func` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L127) | L127 |
| `load_diagnostics` | `func` | [`scripts/gui/dialogs/pos_idx_diagnostics_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/pos_idx_diagnostics_dialog.py#L122) | L122 |
| `load_filter` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L711) | L711 |
| `load_game_by_id` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L236) | L236 |
| `load_settings` | `func` | [`scripts/gui/dialogs/settings_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/settings_dialog.py#L100) | L100 |
| `main` | `fn` | [`src/main.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/main.rs#L3) | L3 |
| `main` | `func` | [`scripts/cql_search_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L121) | L121 |
| `main` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L477) | L477 |
| `main` | `func` | [`scripts/scid_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/scid_gui.py#L13) | L13 |
| `MainWindow` | `class` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L20) | L20 |
| `mark_cql_modified` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L563) | L563 |
| `mark_info_modified` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L551) | L551 |
| `mark_mat_modified` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L559) | L559 |
| `mark_pos_modified` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L555) | L555 |
| `match_cells_recursive` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L680) | L680 |
| `match_cql_constituents_from` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L786) | L786 |
| `match_line_backward_from` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L1463) | L1463 |
| `match_line_forward_from` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L1162) | L1162 |
| `match_rank_pattern` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L601) | L601 |
| `match_rep_helper` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L885) | L885 |
| `match_square_content` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L407) | L407 |
| `match_steps_from` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L584) | L584 |
| `match_wildcard_fen` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L539) | L539 |
| `matches_material` | `fn` | [`src/position_search/scid_search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L19) | L19 |
| `matches_piece_placements` | `fn` | [`src/position_search/parser.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/parser.rs#L56) | L56 |
| `matches_single_ply` | `fn` | [`src/search/scid_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L469) | L469 |
| `matches_single_ply` | `fn` | [`src/search/evaluator/matcher.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs#L821) | L821 |
| `MaterialFilter` | `struct` | [`src/position_search/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/types.rs#L60) | L60 |
| `MaterialFinder` | `struct` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L340) | L340 |
| `MaterialFinder::check_material` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L367) | L367 |
| `MaterialFinder::new` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L350) | L350 |
| `MaterialPredicate` | `struct` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L490) | L490 |
| `MaterialPredicate (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L854) | L854 |
| `mouseMoveEvent` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L57) | L57 |
| `mousePressEvent` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L52) | L52 |
| `move_destination_matches` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L693) | L693 |
| `move_destinations_contain` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L710) | L710 |
| `move_sort_key` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L839) | L839 |
| `MovePattern` | `struct` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L333) | L333 |
| `MovePattern (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L399) | L399 |
| `MoveRecord` | `struct` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L14) | L14 |
| `NagPredicate` | `enum` | [`src/search/annotation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L67) | L67 |
| `NagPredicate::matches` | `fn` | [`src/search/annotation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L77) | L77 |
| `NagPredicate::symbol_to_nag` | `fn` | [`src/search/annotation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/annotation.rs#L86) | L86 |
| `NewDatabaseDialog` | `class` | [`scripts/gui/dialogs/new_db_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/new_db_dialog.py#L15) | L15 |
| `on_backend_error` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L464) | L464 |
| `on_backend_event` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L685) | L685 |
| `on_backend_response` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L450) | L450 |
| `on_backend_response` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L212) | L212 |
| `on_bench_resp` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L424) | L424 |
| `on_board_fen_changed` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L656) | L656 |
| `on_cql_preset_changed` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L567) | L567 |
| `on_db_opened` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L544) | L544 |
| `on_diag_resp` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L354) | L354 |
| `on_diagnostics_received` | `func` | [`scripts/gui/dialogs/pos_idx_diagnostics_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/pos_idx_diagnostics_dialog.py#L132) | L132 |
| `on_explain_done` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L577) | L577 |
| `on_export_pgn` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L231) | L231 |
| `on_fen_text_edited` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L663) | L663 |
| `on_filters_cleared` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L222) | L222 |
| `on_finished` | `func` | [`scripts/gui/dialogs/search_progress_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/search_progress_dialog.py#L89) | L89 |
| `on_game_selected` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L229) | L229 |
| `on_game_summaries_received` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L338) | L338 |
| `on_import_pgn` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L223) | L223 |
| `on_last_played_toggled` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L217) | L217 |
| `on_legal_move_clicked` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L921) | L921 |
| `on_material_preset_changed` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L575) | L575 |
| `on_model_stats_updated` | `func` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L139) | L139 |
| `on_move_double_clicked` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L339) | L339 |
| `on_pgn_loaded` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L764) | L764 |
| `on_preset_selected` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L504) | L504 |
| `on_process_error` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L456) | L456 |
| `on_process_stopped` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L460) | L460 |
| `on_query_text_changed` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L510) | L510 |
| `on_response_received` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L287) | L287 |
| `on_result_row_selected` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L742) | L742 |
| `on_row_double_clicked` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L296) | L296 |
| `on_sample_game_double_clicked` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L328) | L328 |
| `on_scope_changed` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L252) | L252 |
| `on_search_applied` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L215) | L215 |
| `on_search_clicked` | `func` | [`scripts/gui/widgets/filter_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L145) | L145 |
| `on_search_done` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L659) | L659 |
| `on_source_mode_changed` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L514) | L514 |
| `on_tab_changed` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L274) | L274 |
| `on_table_selection_changed` | `func` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L159) | L159 |
| `on_tree_report` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L384) | L384 |
| `on_tree_resp` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L302) | L302 |
| `on_tree_selection_changed` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L303) | L303 |
| `on_valid_resp` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L600) | L600 |
| `open_advanced_search` | `func` | [`scripts/gui/widgets/filter_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L165) | L165 |
| `open_benchmark_dialog` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L250) | L250 |
| `open_build_dialog` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L284) | L284 |
| `open_columns_dialog` | `func` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L118) | L118 |
| `open_database` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L277) | L277 |
| `open_database` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L138) | L138 |
| `open_database` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L533) | L533 |
| `open_database_dialog` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L523) | L523 |
| `open_diagnostics` | `func` | [`scripts/gui/dialogs/build_pos_index_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L244) | L244 |
| `open_pos_idx_diagnostics_dialog` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L243) | L243 |
| `open_settings_dialog` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L239) | L239 |
| `OpeningTreeMoveView` | `struct` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L304) | L304 |
| `OpeningTreeReport` | `struct` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L322) | L322 |
| `OpeningTreeWidget` | `class` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L23) | L23 |
| `pack_date` | `fn` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L113) | L113 |
| `pack_eco` | `fn` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L146) | L146 |
| `pack_result` | `fn` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L176) | L176 |
| `PackedMove` | `struct` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L37) | L37 |
| `PackedMove (impl From<&shakmaty::Move>)::from` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L101) | L101 |
| `PackedMove::from_square` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L55) | L55 |
| `PackedMove::new` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L41) | L41 |
| `PackedMove::promotion` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L65) | L65 |
| `PackedMove::to_shakmaty_move` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L88) | L88 |
| `PackedMove::to_square` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L60) | L60 |
| `PackedMove::to_uci_string` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L75) | L75 |
| `parse_capture_move` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L626) | L626 |
| `parse_compact_piece_placement` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L87) | L87 |
| `parse_direction_ident` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L463) | L463 |
| `parse_fen_piece_char` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L626) | L626 |
| `parse_game_bytes_fast` | `fn` | [`src/pgn_io/import.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/import.rs#L17) | L17 |
| `parse_path_move_token` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L663) | L663 |
| `parse_pgn_headers_and_moves` | `fn` | [`src/search/evaluator/replayer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/replayer.rs#L47) | L47 |
| `parse_piece_placements` | `fn` | [`src/position_search/parser.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/parser.rs#L8) | L8 |
| `parse_piece_specifier` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L7) | L7 |
| `parse_position_matcher` | `fn` | [`src/position_search/parser.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/parser.rs#L67) | L67 |
| `parse_promotion_into_pattern` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L839) | L839 |
| `parse_quiet_move` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L642) | L642 |
| `parse_source_part` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L359) | L359 |
| `parse_square_or_piece` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L171) | L171 |
| `parse_start_position` | `fn` | [`src/position_search/decoder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/decoder.rs#L91) | L91 |
| `parse_tag` | `fn` | [`src/pgn_db/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/builder.rs#L283) | L283 |
| `parse_target_part` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L485) | L485 |
| `parse_target_position` | `fn` | [`src/position_index/codec.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/codec.rs#L43) | L43 |
| `parse_target_position` | `fn` | [`src/tree_index/codec.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/codec.rs#L161) | L161 |
| `parse_token_repetition` | `fn` | [`src/search/parser/moves.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L19) | L19 |
| `parse_u16` | `fn` | [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L638) | L638 |
| `parse_val` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L885) | L885 |
| `ParsedDate` | `struct` | [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L690) | L690 |
| `ParsedDate::parse` | `fn` | [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L697) | L697 |
| `ParsedDate::to_bound` | `fn` | [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L736) | L736 |
| `ParseError` | `struct` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L3) | L3 |
| `ParseError::new` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L19) | L19 |
| `ParseError::with_help` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L30) | L30 |
| `ParseError::with_source_context` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L36) | L36 |
| `PathMatcher` | `struct` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L24) | L24 |
| `PathMatcher::match_legal_move` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L279) | L279 |
| `PathMatcher::match_move` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L28) | L28 |
| `PathMatcher::match_path` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L515) | L515 |
| `PathPattern` | `struct` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L396) | L396 |
| `PathPattern (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L542) | L542 |
| `PathStep` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L383) | L383 |
| `pawn_attacks` | `fn` | [`src/search/pawn.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L252) | L252 |
| `pawn_defenders` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L692) | L692 |
| `PawnEvaluator` | `struct` | [`src/search/pawn.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L74) | L74 |
| `PawnEvaluator::count_backward_pawns` | `fn` | [`src/search/pawn.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L175) | L175 |
| `PawnEvaluator::count_doubled_pawns` | `fn` | [`src/search/pawn.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L165) | L165 |
| `PawnEvaluator::count_isolated_pawns` | `fn` | [`src/search/pawn.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L135) | L135 |
| `PawnEvaluator::count_passed_pawns` | `fn` | [`src/search/pawn.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L104) | L104 |
| `PawnEvaluator::count_pawn_islands` | `fn` | [`src/search/pawn.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L226) | L226 |
| `PawnEvaluator::matches` | `fn` | [`src/search/pawn.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L78) | L78 |
| `PawnEvaluator::pawn_file_mask` | `fn` | [`src/search/pawn.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L151) | L151 |
| `PawnPredicate` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L626) | L626 |
| `PawnPredicate (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L904) | L904 |
| `pgn_reader::begin_game` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L315) | L315 |
| `pgn_reader::begin_game` | `fn` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L117) | L117 |
| `pgn_reader::begin_game` | `fn` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L141) | L141 |
| `pgn_reader::begin_game` | `fn` | [`src/tree_index/dynamic.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L288) | L288 |
| `pgn_reader::begin_variation` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L319) | L319 |
| `pgn_reader::begin_variation` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L375) | L375 |
| `pgn_reader::begin_variation` | `fn` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L124) | L124 |
| `pgn_reader::begin_variation` | `fn` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L146) | L146 |
| `pgn_reader::begin_variation` | `fn` | [`src/tree_index/dynamic.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L294) | L294 |
| `pgn_reader::end_game` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L335) | L335 |
| `pgn_reader::end_game` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L396) | L396 |
| `pgn_reader::end_game` | `fn` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L141) | L141 |
| `pgn_reader::end_game` | `fn` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L187) | L187 |
| `pgn_reader::end_game` | `fn` | [`src/tree_index/dynamic.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L326) | L326 |
| `pgn_reader::san` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L323) | L323 |
| `pgn_reader::san` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L379) | L379 |
| `pgn_reader::san` | `fn` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L128) | L128 |
| `pgn_reader::san` | `fn` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L150) | L150 |
| `pgn_reader::san` | `fn` | [`src/tree_index/dynamic.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L298) | L298 |
| `PgnDatabaseWrapper` | `struct` | [`src/pgn_db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L20) | L20 |
| `PgnDatabaseWrapper::companion_path` | `fn` | [`src/pgn_db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L36) | L36 |
| `PgnDatabaseWrapper::game_count` | `fn` | [`src/pgn_db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L150) | L150 |
| `PgnDatabaseWrapper::get_cached_query_indices` | `fn` | [`src/pgn_db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L678) | L678 |
| `PgnDatabaseWrapper::get_event_ranks` | `fn` | [`src/pgn_db/sorting.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs#L27) | L27 |
| `PgnDatabaseWrapper::get_game_pgn` | `fn` | [`src/pgn_db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L159) | L159 |
| `PgnDatabaseWrapper::get_player_ranks` | `fn` | [`src/pgn_db/sorting.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs#L14) | L14 |
| `PgnDatabaseWrapper::get_site_ranks` | `fn` | [`src/pgn_db/sorting.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs#L40) | L40 |
| `PgnDatabaseWrapper::get_summary` | `fn` | [`src/pgn_db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L179) | L179 |
| `PgnDatabaseWrapper::load_index_file` | `fn` | [`src/pgn_db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L48) | L48 |
| `PgnDatabaseWrapper::mmap_ref` | `fn` | [`src/pgn_db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L154) | L154 |
| `PgnDatabaseWrapper::open` | `fn` | [`src/pgn_db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L88) | L88 |
| `PgnDatabaseWrapper::query_games` | `fn` | [`src/pgn_db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L669) | L669 |
| `PgnDatabaseWrapper::query_games_with_progress` | `fn` | [`src/pgn_db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L201) | L201 |
| `PgnDatabaseWrapper::search_material` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L107) | L107 |
| `PgnDatabaseWrapper::search_position` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L15) | L15 |
| `PgnDatabaseWrapper::search_query` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L259) | L259 |
| `PgnDatabaseWrapper::search_query_range` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L267) | L267 |
| `PgnDatabaseWrapper::search_query_range_with_progress` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L148) | L148 |
| `PgnDatabaseWrapper::search_query_with_progress` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L247) | L247 |
| `PgnDatabaseWrapper::sort_and_export` | `fn` | [`src/pgn_db/sorting.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs#L147) | L147 |
| `PgnDatabaseWrapper::sort_indices` | `fn` | [`src/pgn_db/sorting.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs#L53) | L53 |
| `PgnIndexHeader` | `struct` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L87) | L87 |
| `PgnNameTables` | `struct` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L45) | L45 |
| `PgnNameTables::event` | `fn` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L69) | L69 |
| `PgnNameTables::new` | `fn` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L52) | L52 |
| `PgnNameTables::player` | `fn` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L61) | L61 |
| `PgnNameTables::site` | `fn` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L77) | L77 |
| `PgnPositionVisitor` | `struct` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L90) | L90 |
| `PgnPositionVisitor::new` | `fn` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L99) | L99 |
| `PgnSinglePositionVisitor` | `struct` | [`src/tree_index/dynamic.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L243) | L243 |
| `PgnSinglePositionVisitor::new` | `fn` | [`src/tree_index/dynamic.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/dynamic.rs#L259) | L259 |
| `PgnTreeStatsVisitor` | `struct` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L102) | L102 |
| `PgnTreeStatsVisitor::new` | `fn` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L115) | L115 |
| `piece_directly_attacks` | `fn` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L1036) | L1036 |
| `piece_value` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L723) | L723 |
| `PieceCountAccumulator` | `struct` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L758) | L758 |
| `PieceCountAccumulator::add_piece_at_square` | `fn` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L770) | L770 |
| `PieceCountAccumulator::min_promotions_needed` | `fn` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L799) | L799 |
| `PieceCountAccumulator::set_count` | `fn` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L788) | L788 |
| `PieceCountAccumulator::total_pieces` | `fn` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L811) | L811 |
| `PieceCountAccumulator::validate` | `fn` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L815) | L815 |
| `PieceMatcher` | `struct` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L517) | L517 |
| `PieceMatcher (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L35) | L35 |
| `PieceMatcher::matches` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L540) | L540 |
| `PieceMatcher::new` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L536) | L536 |
| `play_move_san` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L287) | L287 |
| `populate_data` | `func` | [`scripts/gui/dialogs/pos_idx_diagnostics_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/pos_idx_diagnostics_dialog.py#L141) | L141 |
| `populate_results_table` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L697) | L697 |
| `PosIdxDevWorkbench` | `class` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L44) | L44 |
| `PosIdxDiagnosticsDialog` | `class` | [`scripts/gui/dialogs/pos_idx_diagnostics_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/pos_idx_diagnostics_dialog.py#L16) | L16 |
| `PositionFinder` | `struct` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L281) | L281 |
| `PositionFinder::check_current_pos` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L302) | L302 |
| `PositionFinder::new` | `fn` | [`src/pgn_db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/search_adapter.rs#L290) | L290 |
| `PositionIndex` | `struct` | [`src/position_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L15) | L15 |
| `PositionIndex::build_for_pgn` | `fn` | [`src/position_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L238) | L238 |
| `PositionIndex::build_for_scid` | `fn` | [`src/position_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L221) | L221 |
| `PositionIndex::check_status` | `fn` | [`src/position_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L43) | L43 |
| `PositionIndex::companion_path` | `fn` | [`src/position_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L23) | L23 |
| `PositionIndex::get_all_position_games` | `fn` | [`src/position_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L172) | L172 |
| `PositionIndex::get_matching_game_ids` | `fn` | [`src/position_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L148) | L148 |
| `PositionIndex::index_entries` | `fn` | [`src/position_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L136) | L136 |
| `PositionIndex::load` | `fn` | [`src/position_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L100) | L100 |
| `PositionIndex::scan_diagnostics` | `fn` | [`src/position_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L177) | L177 |
| `PositionIndexHeader` | `struct` | [`src/position_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L20) | L20 |
| `PositionIndexHeader::read_from_slice` | `fn` | [`src/position_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L35) | L35 |
| `PositionIndexHeader::write_to` | `fn` | [`src/position_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L70) | L70 |
| `PositionMatch` | `struct` | [`src/position_search/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/types.rs#L6) | L6 |
| `PositionMatcher` | `struct` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L12) | L12 |
| `PositionMatcher::matches` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L16) | L16 |
| `PositionMatcher::matches_at_ply` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L21) | L21 |
| `PositionMatcher::matches_at_ply_with_env` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L161) | L161 |
| `PositionMatcher::matches_material` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L180) | L180 |
| `PositionMatcher::matches_power` | `fn` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L373) | L373 |
| `PositionPattern` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L200) | L200 |
| `PositionPattern (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L286) | L286 |
| `PositionPostingList` | `struct` | [`src/position_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L93) | L93 |
| `PositionPostingList::add` | `fn` | [`src/position_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L107) | L107 |
| `PositionPostingList::merge` | `fn` | [`src/position_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L114) | L114 |
| `PositionPostingList::new` | `fn` | [`src/position_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L99) | L99 |
| `PositionSearchResult` | `struct` | [`src/position_search/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/types.rs#L12) | L12 |
| `PositionTargetMatcher` | `enum` | [`src/position_search/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/types.rs#L21) | L21 |
| `PositionTargetMatcher::matches` | `fn` | [`src/position_search/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/types.rs#L35) | L35 |
| `PowerPredicate` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L656) | L656 |
| `PowerPredicate (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L951) | L951 |
| `predicate_eval_cost` | `fn` | [`src/search/evaluator/matcher.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs#L974) | L974 |
| `print_help` | `fn` | [`src/cli/commands/repl.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L329) | L329 |
| `print_schema` | `fn` | [`src/cli/commands/repl.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/repl.rs#L353) | L353 |
| `prompt_build_pos_index` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L208) | L208 |
| `prompt_build_pos_index` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L257) | L257 |
| `ProtocolLogPanelWidget` | `class` | [`scripts/gui/widgets/protocol_log_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/protocol_log_panel.py#L6) | L6 |
| `query_opening_tree` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L294) | L294 |
| `QueryBranch` | `struct` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L1109) | L1109 |
| `QueryExplanation` | `struct` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L1116) | L1116 |
| `QueryMatchResult` | `struct` | [`src/search/evaluator/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/mod.rs#L22) | L22 |
| `QueryParser` | `struct` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L23) | L23 |
| `QueryParser::advance` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L76) | L76 |
| `QueryParser::current_pos` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L72) | L72 |
| `QueryParser::expect_ident` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L96) | L96 |
| `QueryParser::expect_number` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L127) | L127 |
| `QueryParser::expect_string_or_ident` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L111) | L111 |
| `QueryParser::expect_token` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L142) | L142 |
| `QueryParser::explain` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L59) | L59 |
| `QueryParser::has_more_in_and` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L308) | L308 |
| `QueryParser::is_bracket_piece_list` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L1018) | L1018 |
| `QueryParser::is_juxtaposition_square_start` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L617) | L617 |
| `QueryParser::is_square_set_atom_start` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L719) | L719 |
| `QueryParser::match_ident` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L86) | L86 |
| `QueryParser::new` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L29) | L29 |
| `QueryParser::parse_and_expr` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L292) | L292 |
| `QueryParser::parse_attacked_expr` | `fn` | [`src/search/parser/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L763) | L763 |
| `QueryParser::parse_attacks_expr` | `fn` | [`src/search/parser/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L683) | L683 |
| `QueryParser::parse_castling_filter` | `fn` | [`src/search/parser/pieces.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L497) | L497 |
| `QueryParser::parse_color_modified_piece` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L316) | L316 |
| `QueryParser::parse_color_token` | `fn` | [`src/search/parser/pieces.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L373) | L373 |
| `QueryParser::parse_comparison_op` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L157) | L157 |
| `QueryParser::parse_cql_line_constituent` | `fn` | [`src/search/parser/moves.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L974) | L974 |
| `QueryParser::parse_cql_line_expr` | `fn` | [`src/search/parser/moves.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L793) | L793 |
| `QueryParser::parse_cql_line_or_legacy_path` | `fn` | [`src/search/parser/moves.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L746) | L746 |
| `QueryParser::parse_cql_path_expr` | `fn` | [`src/search/parser/moves.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L419) | L419 |
| `QueryParser::parse_cql_path_repetition_quantifier` | `fn` | [`src/search/parser/moves.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L701) | L701 |
| `QueryParser::parse_cql_path_single_constituent` | `fn` | [`src/search/parser/moves.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L511) | L511 |
| `QueryParser::parse_diagonal_or_ray_expression` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L214) | L214 |
| `QueryParser::parse_direction_expression` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L154) | L154 |
| `QueryParser::parse_distance_expr` | `fn` | [`src/search/parser/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L642) | L642 |
| `QueryParser::parse_fork_expr` | `fn` | [`src/search/parser/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L180) | L180 |
| `QueryParser::parse_header_keyword` | `fn` | [`src/search/parser/headers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/headers.rs#L6) | L6 |
| `QueryParser::parse_move_filter` | `fn` | [`src/search/parser/moves.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L1573) | L1573 |
| `QueryParser::parse_move_number_expr` | `fn` | [`src/search/parser/ranges.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/ranges.rs#L95) | L95 |
| `QueryParser::parse_occurrences_expr` | `fn` | [`src/search/parser/ranges.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/ranges.rs#L104) | L104 |
| `QueryParser::parse_open_file_expr` | `fn` | [`src/search/parser/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L586) | L586 |
| `QueryParser::parse_or_expr` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L275) | L275 |
| `QueryParser::parse_outpost_expr` | `fn` | [`src/search/parser/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L524) | L524 |
| `QueryParser::parse_path_expr` | `fn` | [`src/search/parser/moves.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L134) | L134 |
| `QueryParser::parse_pawn_color_opt` | `fn` | [`src/search/parser/pieces.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L361) | L361 |
| `QueryParser::parse_pawn_pred_backward` | `fn` | [`src/search/parser/pieces.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L421) | L421 |
| `QueryParser::parse_pawn_pred_doubled` | `fn` | [`src/search/parser/pieces.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L410) | L410 |
| `QueryParser::parse_pawn_pred_islands` | `fn` | [`src/search/parser/pieces.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L432) | L432 |
| `QueryParser::parse_pawn_pred_isolated` | `fn` | [`src/search/parser/pieces.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L399) | L399 |
| `QueryParser::parse_pawn_pred_passed` | `fn` | [`src/search/parser/pieces.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L388) | L388 |
| `QueryParser::parse_piece_count_or_squares` | `fn` | [`src/search/parser/pieces.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L266) | L266 |
| `QueryParser::parse_piece_matcher_arg` | `fn` | [`src/search/parser/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L12) | L12 |
| `QueryParser::parse_piece_matcher_list` | `fn` | [`src/search/parser/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L823) | L823 |
| `QueryParser::parse_piece_on_square` | `fn` | [`src/search/parser/pieces.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L14) | L14 |
| `QueryParser::parse_piece_on_square_with_filter` | `fn` | [`src/search/parser/pieces.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L18) | L18 |
| `QueryParser::parse_pin_expr` | `fn` | [`src/search/parser/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L40) | L40 |
| `QueryParser::parse_ply_expr` | `fn` | [`src/search/parser/ranges.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/ranges.rs#L8) | L8 |
| `QueryParser::parse_primary_expr` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L328) | L328 |
| `QueryParser::parse_skewer_expr` | `fn` | [`src/search/parser/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L363) | L363 |
| `QueryParser::parse_square_or_piece_set` | `fn` | [`src/search/parser/moves.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L1417) | L1417 |
| `QueryParser::parse_square_set` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L21) | L21 |
| `QueryParser::parse_square_set_atom` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L771) | L771 |
| `QueryParser::parse_square_set_diff` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L579) | L579 |
| `QueryParser::parse_square_set_expr` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L563) | L563 |
| `QueryParser::parse_square_set_intersection` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L595) | L595 |
| `QueryParser::parse_square_set_query` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L523) | L523 |
| `QueryParser::parse_square_set_unary` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L636) | L636 |
| `QueryParser::parse_square_set_union` | `fn` | [`src/search/parser/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/squares.rs#L567) | L567 |
| `QueryParser::parse_str` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L34) | L34 |
| `QueryParser::parse_string_comparison_op` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L190) | L190 |
| `QueryParser::parse_string_or_regex_val` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L247) | L247 |
| `QueryParser::parse_trapped_expr` | `fn` | [`src/search/parser/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/tactics.rs#L503) | L503 |
| `QueryParser::parse_unary_expr` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L320) | L320 |
| `QueryParser::parse_variable_domain` | `fn` | [`src/search/parser/pieces.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/pieces.rs#L443) | L443 |
| `QueryParser::peek` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L64) | L64 |
| `QueryParser::peek_nth` | `fn` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs#L68) | L68 |
| `quick_check_entry_headers` | `fn` | [`src/search/evaluator/matcher.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs#L769) | L769 |
| `quick_check_headers_only` | `fn` | [`src/search/evaluator/matcher.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs#L744) | L744 |
| `quick_check_pgn_entry_headers` | `fn` | [`src/search/evaluator/matcher.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs#L795) | L795 |
| `rank_char_to_idx` | `fn` | [`src/search/parser/helpers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/helpers.rs#L351) | L351 |
| `rank_span_above_or_equal` | `fn` | [`src/search/pawn.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L236) | L236 |
| `rank_span_below_or_equal` | `fn` | [`src/search/pawn.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pawn.rs#L244) | L244 |
| `RankCellMatcher` | `enum` | [`src/search/pattern.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/pattern.rs#L592) | L592 |
| `RawGameRecord` | `struct` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L99) | L99 |
| `RawPgnTags` | `struct` | [`src/pgn_io/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/types.rs#L26) | L26 |
| `read_varint` | `fn` | [`src/position_index/codec.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/codec.rs#L74) | L74 |
| `read_varint` | `fn` | [`src/tree_index/codec.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/codec.rs#L192) | L192 |
| `refresh_current_position` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L258) | L258 |
| `refresh_database_info` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L151) | L151 |
| `replay_game` | `fn` | [`src/search/evaluator/replayer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/replayer.rs#L178) | L178 |
| `request_chunks_for_range` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L158) | L158 |
| `RequestMessage` | `struct` | [`src/server/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/mod.rs#L20) | L20 |
| `reset_all` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L677) | L677 |
| `reset_board` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L288) | L288 |
| `reset_defaults` | `func` | [`scripts/gui/dialogs/columns_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/columns_dialog.py#L73) | L73 |
| `reset_filters` | `func` | [`scripts/gui/widgets/filter_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/filter_panel.py#L148) | L148 |
| `reset_to_initial` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L253) | L253 |
| `resolve_squares` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L638) | L638 |
| `ResponseMessage` | `struct` | [`src/server/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/mod.rs#L28) | L28 |
| `result_code_to_str` | `fn` | [`src/db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L120) | L120 |
| `rowCount` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L58) | L58 |
| `run` | `fn` | [`src/cli/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/mod.rs#L422) | L422 |
| `run_benchmark` | `fn` | [`src/benchmark.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/benchmark.rs#L30) | L30 |
| `run_benchmark` | `func` | [`scripts/gui/dialogs/benchmark_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/benchmark_dialog.py#L140) | L140 |
| `run_filter_benchmark` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L413) | L413 |
| `run_interactive_server` | `fn` | [`src/server/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/mod.rs#L37) | L37 |
| `save_column_settings` | `func` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L122) | L122 |
| `save_db` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L159) | L159 |
| `save_index_file` | `fn` | [`src/pgn_db/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/builder.rs#L14) | L14 |
| `save_settings` | `func` | [`scripts/gui/dialogs/settings_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/settings_dialog.py#L112) | L112 |
| `scan_chunk` | `fn` | [`src/pgn_db/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/builder.rs#L179) | L179 |
| `scan_diagnostics` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L348) | L348 |
| `scan_pgn_parallel` | `fn` | [`src/pgn_db/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/builder.rs#L77) | L77 |
| `scan_python_file` | `func` | [`scripts/index_codebase.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/index_codebase.py#L63) | L63 |
| `scan_rust_file` | `func` | [`scripts/index_codebase.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/index_codebase.py#L20) | L20 |
| `scan_server_commands` | `func` | [`scripts/index_codebase.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/index_codebase.py#L88) | L88 |
| `ScidDatabaseWrapper` | `struct` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L13) | L13 |
| `ScidDatabaseWrapper::add_game` | `fn` | [`src/db/mutations.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mutations.rs#L12) | L12 |
| `ScidDatabaseWrapper::clear_query_caches` | `fn` | [`src/db/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L10) | L10 |
| `ScidDatabaseWrapper::compact` | `fn` | [`src/db/mutations.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mutations.rs#L118) | L118 |
| `ScidDatabaseWrapper::create` | `fn` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L116) | L116 |
| `ScidDatabaseWrapper::delete_game` | `fn` | [`src/db/mutations.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mutations.rs#L98) | L98 |
| `ScidDatabaseWrapper::entries` | `fn` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L163) | L163 |
| `ScidDatabaseWrapper::format` | `fn` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L151) | L151 |
| `ScidDatabaseWrapper::game_count` | `fn` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L159) | L159 |
| `ScidDatabaseWrapper::game_pgn` | `fn` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L205) | L205 |
| `ScidDatabaseWrapper::games_path` | `fn` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L171) | L171 |
| `ScidDatabaseWrapper::get_blob` | `fn` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L179) | L179 |
| `ScidDatabaseWrapper::get_cached_query_indices` | `fn` | [`src/db/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L586) | L586 |
| `ScidDatabaseWrapper::get_event_ranks` | `fn` | [`src/db/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L32) | L32 |
| `ScidDatabaseWrapper::get_game_summary` | `fn` | [`src/db/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L71) | L71 |
| `ScidDatabaseWrapper::get_player_ranks` | `fn` | [`src/db/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L19) | L19 |
| `ScidDatabaseWrapper::get_round_ranks` | `fn` | [`src/db/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L58) | L58 |
| `ScidDatabaseWrapper::get_site_ranks` | `fn` | [`src/db/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L45) | L45 |
| `ScidDatabaseWrapper::index_path` | `fn` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L155) | L155 |
| `ScidDatabaseWrapper::is_deleted` | `fn` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L175) | L175 |
| `ScidDatabaseWrapper::names` | `fn` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L167) | L167 |
| `ScidDatabaseWrapper::open` | `fn` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L49) | L49 |
| `ScidDatabaseWrapper::query_games` | `fn` | [`src/db/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L577) | L577 |
| `ScidDatabaseWrapper::query_games_with_progress` | `fn` | [`src/db/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L101) | L101 |
| `ScidDatabaseWrapper::save` | `fn` | [`src/db/mutations.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mutations.rs#L148) | L148 |
| `ScidDatabaseWrapper::search_material` | `fn` | [`src/db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L98) | L98 |
| `ScidDatabaseWrapper::search_material_with_progress` | `fn` | [`src/db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L82) | L82 |
| `ScidDatabaseWrapper::search_position` | `fn` | [`src/db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L72) | L72 |
| `ScidDatabaseWrapper::search_position_with_progress` | `fn` | [`src/db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L6) | L6 |
| `ScidDatabaseWrapper::search_query` | `fn` | [`src/db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L141) | L141 |
| `ScidDatabaseWrapper::search_query_progress_helper` | `fn` | [`src/db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L123) | L123 |
| `ScidDatabaseWrapper::search_query_range` | `fn` | [`src/db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L171) | L171 |
| `ScidDatabaseWrapper::search_query_range_with_progress` | `fn` | [`src/db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L149) | L149 |
| `ScidDatabaseWrapper::search_query_with_progress` | `fn` | [`src/db/search_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/search_adapter.rs#L106) | L106 |
| `ScidDatabaseWrapper::sort_database` | `fn` | [`src/db/sorting.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/sorting.rs#L12) | L12 |
| `ScidDatabaseWrapper::sort_database_to` | `fn` | [`src/db/sorting.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/sorting.rs#L61) | L61 |
| `ScidDatabaseWrapper::sort_indices` | `fn` | [`src/db/sorting.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/sorting.rs#L128) | L128 |
| `ScidDatabaseWrapper::stats` | `fn` | [`src/db/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L216) | L216 |
| `ScidDatabaseWrapper::undelete_game` | `fn` | [`src/db/mutations.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mutations.rs#L108) | L108 |
| `ScidDatabaseWrapper::update_game` | `fn` | [`src/db/mutations.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/mutations.rs#L53) | L53 |
| `ScidFormat` | `enum` | [`src/db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L5) | L5 |
| `ScidMatchResult` | `struct` | [`src/search/scid_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L14) | L14 |
| `ScidSearchAdapter` | `struct` | [`src/search/scid_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L20) | L20 |
| `ScidSearchAdapter::evaluate_scid_game` | `fn` | [`src/search/scid_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L24) | L24 |
| `ScidSearchAdapter::evaluate_scid_game_streaming` | `fn` | [`src/search/scid_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L123) | L123 |
| `ScidSearchAdapter::replay_scid_blob` | `fn` | [`src/search/scid_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L236) | L236 |
| `ScidSearchAdapter::replay_scid_blob_opt` | `fn` | [`src/search/scid_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L241) | L241 |
| `ScidSearchAdapter::search_parallel` | `fn` | [`src/search/scid_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L433) | L433 |
| `ScidSearchAdapter::search_parallel_range` | `fn` | [`src/search/scid_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L446) | L446 |
| `ScidSearchAdapter::search_parallel_range_with_progress` | `fn` | [`src/search/scid_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L325) | L325 |
| `ScidSearchAdapter::search_parallel_with_progress` | `fn` | [`src/search/scid_adapter.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/scid_adapter.rs#L410) | L410 |
| `search_material_mmap` | `fn` | [`src/position_search/scid_search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L539) | L539 |
| `search_material_mmap_with_progress` | `fn` | [`src/position_search/scid_search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L417) | L417 |
| `search_piece_placements_mmap` | `fn` | [`src/position_search/scid_search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L399) | L399 |
| `search_piece_placements_mmap_with_progress` | `fn` | [`src/position_search/scid_search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L273) | L273 |
| `search_position_matcher_mmap_with_progress` | `fn` | [`src/position_search/scid_search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L106) | L106 |
| `search_position_mmap` | `fn` | [`src/position_search/scid_search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L263) | L263 |
| `search_position_mmap_with_progress` | `fn` | [`src/position_search/scid_search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/scid_search.rs#L231) | L231 |
| `SearchProgressDialog` | `class` | [`scripts/gui/dialogs/search_progress_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/search_progress_dialog.py#L13) | L13 |
| `SearchQuery` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L675) | L675 |
| `SearchQuery (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L987) | L987 |
| `SearchQuery::and` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L740) | L740 |
| `SearchQuery::can_stream_early_exit` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L810) | L810 |
| `SearchQuery::has_header_predicates` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L790) | L790 |
| `SearchQuery::is_header_only` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L752) | L752 |
| `SearchQuery::max_ply_cutoff` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L840) | L840 |
| `SearchQuery::negate` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L748) | L748 |
| `SearchQuery::or` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L744) | L744 |
| `SearchQuery::requires_san_strings` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L767) | L767 |
| `select_all` | `func` | [`scripts/gui/dialogs/columns_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/columns_dialog.py#L69) | L69 |
| `select_all_categories` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L539) | L539 |
| `send_request` | `func` | [`scripts/gui/backend_client.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L143) | L143 |
| `set_default_column_widths` | `func` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L77) | L77 |
| `set_fen` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L258) | L258 |
| `set_filters` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L184) | L184 |
| `set_pgn_text` | `func` | [`scripts/gui/widgets/game_preview_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L77) | L77 |
| `set_ply` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L807) | L807 |
| `set_selected_game` | `func` | [`scripts/gui/widgets/game_preview_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_preview_panel.py#L66) | L66 |
| `set_single_piece_demo` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L671) | L671 |
| `set_val` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L590) | L590 |
| `SetPredicate` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L67) | L67 |
| `SetPredicate (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L197) | L197 |
| `SettingsDialog` | `class` | [`scripts/gui/dialogs/settings_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/settings_dialog.py#L16) | L16 |
| `show_about` | `func` | [`scripts/cql_search_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/cql_search_gui.py#L88) | L88 |
| `show_all_columns` | `func` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L113) | L113 |
| `show_header_context_menu` | `func` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L82) | L82 |
| `skip_extra_tags` | `fn` | [`src/position_search/decoder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/decoder.rs#L8) | L8 |
| `sort_pgn_file` | `fn` | [`src/pgn_db/sorting.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs#L186) | L186 |
| `SortedIndexEntry` | `struct` | [`src/position_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/types.rs#L87) | L87 |
| `SortedTreeIndexEntry` | `struct` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L176) | L176 |
| `split_pgn_headers_and_moves` | `fn` | [`src/search/evaluator/replayer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/replayer.rs#L13) | L13 |
| `square_clicked` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L201) | L201 |
| `square_from_coords` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L684) | L684 |
| `SquareContent` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L188) | L188 |
| `SquareContent (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L80) | L80 |
| `SquareOrPiece` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L557) | L557 |
| `SquareOrPiece (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L69) | L69 |
| `SquareSetEvaluator` | `struct` | [`src/search/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/squares.rs#L8) | L8 |
| `SquareSetEvaluator::eval_expr` | `fn` | [`src/search/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/squares.rs#L12) | L12 |
| `SquareSetEvaluator::matches` | `fn` | [`src/search/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/squares.rs#L180) | L180 |
| `SquareSetEvaluator::matches_with_env` | `fn` | [`src/search/squares.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/squares.rs#L140) | L140 |
| `SquareSetExpr` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L7) | L7 |
| `SquareSetExpr (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L137) | L137 |
| `SquareWidget` | `class` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L32) | L32 |
| `standard_piece_slots` | `fn` | [`src/pgn_io/encoder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/encoder.rs#L7) | L7 |
| `standard_piece_slots` | `fn` | [`src/position_search/decoder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/decoder.rs#L118) | L118 |
| `start` | `func` | [`scripts/gui/backend_client.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L50) | L50 |
| `start_backend` | `func` | [`scripts/pos_idx_dev_gui.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/pos_idx_dev_gui.py#L249) | L249 |
| `start_backend` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L118) | L118 |
| `start_build` | `func` | [`scripts/gui/dialogs/build_pos_index_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L155) | L155 |
| `std::fmt` | `fn` | [`src/db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/types.rs#L11) | L11 |
| `std::fmt` | `fn` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L112) | L112 |
| `std::not` | `fn` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L859) | L859 |
| `stop` | `func` | [`scripts/gui/backend_client.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/backend_client.py#L165) | L165 |
| `stop_backend` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L134) | L134 |
| `StripedPositionPostingMap` | `struct` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L25) | L25 |
| `StripedPositionPostingMap::into_map` | `fn` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L69) | L69 |
| `StripedPositionPostingMap::new` | `fn` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L31) | L31 |
| `StripedPositionPostingMap::record` | `fn` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L47) | L47 |
| `StripedPositionPostingMap::stripe_index` | `fn` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L43) | L43 |
| `StripedPositionPostingMap::total_positions` | `fn` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L65) | L65 |
| `StripedTreePositionMap` | `struct` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L26) | L26 |
| `StripedTreePositionMap::into_map` | `fn` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L81) | L81 |
| `StripedTreePositionMap::new` | `fn` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L32) | L32 |
| `StripedTreePositionMap::record` | `fn` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L49) | L49 |
| `StripedTreePositionMap::stripe_index` | `fn` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L44) | L44 |
| `StripedTreePositionMap::total_positions` | `fn` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L77) | L77 |
| `symmetry_label` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L1168) | L1168 |
| `TacticalPredicate` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L574) | L574 |
| `TacticalPredicate (impl ToDsl)::to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L721) | L721 |
| `TacticsEvaluator` | `struct` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L5) | L5 |
| `TacticsEvaluator::has_discovered_attack` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L283) | L283 |
| `TacticsEvaluator::has_fork` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L181) | L181 |
| `TacticsEvaluator::has_open_file` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L545) | L545 |
| `TacticsEvaluator::has_outpost` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L438) | L438 |
| `TacticsEvaluator::has_pin` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L88) | L88 |
| `TacticsEvaluator::has_rook_on_seventh` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L529) | L529 |
| `TacticsEvaluator::has_skewer` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L317) | L317 |
| `TacticsEvaluator::has_trapped_piece` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L403) | L403 |
| `TacticsEvaluator::match_distinct_target_slots` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L256) | L256 |
| `TacticsEvaluator::matches` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L56) | L56 |
| `TacticsEvaluator::matches_attacks` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L83) | L83 |
| `TacticsEvaluator::matches_attacks_with_env` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L61) | L61 |
| `TacticsEvaluator::matches_distance` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L620) | L620 |
| `TacticsEvaluator::matches_distance_with_env` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L596) | L596 |
| `TacticsEvaluator::matches_with_env` | `fn` | [`src/search/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tactics.rs#L9) | L9 |
| `test_advanced_move_separator_promotions_and_captures` | `fn` | [`src/search/tests/dsl_and_validation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L209) | L209 |
| `test_alex_pgn_queries` | `fn` | [`src/search/tests/adapters.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/adapters.rs#L45) | L45 |
| `test_bare_piece_syntax_and_cql_piece_counts` | `fn` | [`src/search/tests/positions.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L201) | L201 |
| `test_bracket_set_comparisons` | `fn` | [`src/search/tests/geometry.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L383) | L383 |
| `test_bracketed_piece_group_counts` | `fn` | [`src/search/tests/positions.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L268) | L268 |
| `test_comments_and_nag_annotations` | `fn` | [`src/search/tests/headers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/headers.rs#L259) | L259 |
| `test_compact_piece_placements` | `fn` | [`src/search/tests/positions.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L243) | L243 |
| `test_composite_boolean_and_ply_range_queries` | `fn` | [`src/search/tests/dsl_and_validation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L6) | L6 |
| `test_cql_dsl_query_parser` | `fn` | [`src/search/tests/dsl_and_validation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L63) | L63 |
| `test_cql_line_specification_forward_backward_and_modifiers` | `fn` | [`src/search/tests/moves_and_paths.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L475) | L475 |
| `test_cql_path_specification_and_interleaved_filters` | `fn` | [`src/search/tests/moves_and_paths.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L439) | L439 |
| `test_cqli_direction_spatial_shifts_and_rotations` | `fn` | [`src/search/tests/geometry.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L310) | L310 |
| `test_descriptive_parse_error_diagnostics` | `fn` | [`src/search/tests/dsl_and_validation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L104) | L104 |
| `test_direction_filters_and_move_paths` | `fn` | [`src/search/tests/moves_and_paths.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L303) | L303 |
| `test_fen_transformations_and_symmetries` | `fn` | [`src/search/tests/positions.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L316) | L316 |
| `test_fork_exact_slots_and_piece_options` | `fn` | [`src/search/tests/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L239) | L239 |
| `test_header_metadata_search` | `fn` | [`src/search/tests/headers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/headers.rs#L5) | L5 |
| `test_legal_mate_queries` | `fn` | [`src/search/tests/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L320) | L320 |
| `test_light_and_dark_square_and_bishop_filtering` | `fn` | [`src/search/tests/geometry.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L94) | L94 |
| `test_manual_examples_all_valid` | `fn` | [`src/search/tests/dsl_and_validation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L355) | L355 |
| `test_mating_themes_catalog_parsing_and_evaluation` | `fn` | [`src/search/tests/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L42) | L42 |
| `test_move_capture_parameter_logic` | `fn` | [`src/search/tests/moves_and_paths.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L635) | L635 |
| `test_move_pattern_and_path_sequence_search` | `fn` | [`src/search/tests/moves_and_paths.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L6) | L6 |
| `test_move_piece_targets_ply_movenumber_and_multiline_and` | `fn` | [`src/search/tests/moves_and_paths.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L565) | L565 |
| `test_multi_square_and_symmetry_transformations` | `fn` | [`src/search/tests/geometry.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L7) | L7 |
| `test_parent_and_child_scoping` | `fn` | [`src/search/tests/dsl_and_validation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L743) | L743 |
| `test_path_piece_identifiers_search` | `fn` | [`src/search/tests/moves_and_paths.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L70) | L70 |
| `test_path_regex_quantifiers_and_consecutive_default` | `fn` | [`src/search/tests/moves_and_paths.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L196) | L196 |
| `test_pawn_structure_search` | `fn` | [`src/search/tests/positions.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L75) | L75 |
| `test_pgn_index_entry_header_prefiltering` | `fn` | [`src/search/tests/headers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/headers.rs#L185) | L185 |
| `test_pin_named_parameters` | `fn` | [`src/search/tests/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L156) | L156 |
| `test_play_and_leads_to_hypothetical_moves` | `fn` | [`src/search/tests/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L353) | L353 |
| `test_play_and_not_move_missed_mate_regression` | `fn` | [`src/search/tests/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L447) | L447 |
| `test_position_and_move_or_path_combination` | `fn` | [`src/search/tests/positions.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L341) | L341 |
| `test_position_pattern_search` | `fn` | [`src/search/tests/positions.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L7) | L7 |
| `test_posting_payload_roundtrip` | `fn` | [`src/position_index/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/mod.rs#L24) | L24 |
| `test_power_expressions` | `fn` | [`src/search/tests/positions.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L148) | L148 |
| `test_previous_move_queries` | `fn` | [`src/search/tests/moves_and_paths.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L243) | L243 |
| `test_query_comments_support` | `fn` | [`src/search/tests/dsl_and_validation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L150) | L150 |
| `test_query_explain_and_to_dsl` | `fn` | [`src/search/tests/dsl_and_validation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L165) | L165 |
| `test_query_semantic_validation_contradictions` | `fn` | [`src/search/tests/dsl_and_validation.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/dsl_and_validation.rs#L499) | L499 |
| `test_scid_index_entry_header_prefiltering` | `fn` | [`src/search/tests/headers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/headers.rs#L117) | L117 |
| `test_single_color_path_and_move_repetition_quantifiers` | `fn` | [`src/search/tests/moves_and_paths.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L396) | L396 |
| `test_square_ranges_and_diagonal_expansion` | `fn` | [`src/search/tests/geometry.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L31) | L31 |
| `test_square_set_algebra_and_bitboard_engine` | `fn` | [`src/search/tests/geometry.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L263) | L263 |
| `test_tactical_motifs_and_geometry` | `fn` | [`src/search/tests/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L6) | L6 |
| `test_tag_and_custom_headers` | `fn` | [`src/search/tests/headers.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/headers.rs#L85) | L85 |
| `test_variable_binding_and_fork_queries` | `fn` | [`src/search/tests/tactics.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/tactics.rs#L181) | L181 |
| `test_wildcard_fen_and_casing_conventions` | `fn` | [`src/search/tests/positions.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/positions.rs#L113) | L113 |
| `test_wildcard_moves_and_promotions_and_en_passant` | `fn` | [`src/search/tests/moves_and_paths.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/moves_and_paths.rs#L108) | L108 |
| `test_wtm_btm_any_color_and_legal_move_filters` | `fn` | [`src/search/tests/geometry.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/geometry.rs#L158) | L158 |
| `test_zero_copy_scid_adapter` | `fn` | [`src/search/tests/adapters.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/adapters.rs#L4) | L4 |
| `to_dsl` | `fn` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L14) | L14 |
| `ToDsl` | `trait` | [`src/search/explain.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/explain.rs#L13) | L13 |
| `toggle_backend` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L205) | L205 |
| `toggle_column_visibility` | `func` | [`scripts/gui/widgets/game_table_panel.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/game_table_panel.py#L109) | L109 |
| `toggle_sort_column` | `func` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L76) | L76 |
| `Token` | `enum` | [`src/search/parser/lexer.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/lexer.rs#L131) | L131 |
| `TransformMatcher` | `struct` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L1125) | L1125 |
| `TransformMatcher::matches_with_symmetry` | `fn` | [`src/search/transform.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/transform.rs#L1129) | L1129 |
| `TreeIndex` | `struct` | [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L15) | L15 |
| `TreeIndex::build_for_pgn` | `fn` | [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L269) | L269 |
| `TreeIndex::build_for_scid` | `fn` | [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L252) | L252 |
| `TreeIndex::calculate_tree_for_pgn` | `fn` | [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L240) | L240 |
| `TreeIndex::calculate_tree_for_scid` | `fn` | [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L223) | L223 |
| `TreeIndex::check_status` | `fn` | [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L43) | L43 |
| `TreeIndex::companion_path` | `fn` | [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L23) | L23 |
| `TreeIndex::get_position` | `fn` | [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L148) | L148 |
| `TreeIndex::index_entries` | `fn` | [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L136) | L136 |
| `TreeIndex::load` | `fn` | [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L100) | L100 |
| `TreeIndex::query_tree` | `fn` | [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L170) | L170 |
| `TreeIndex::query_tree_with_options` | `fn` | [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L177) | L177 |
| `TreeIndex::scan_diagnostics` | `fn` | [`src/tree_index/core.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L187) | L187 |
| `TreeIndexDiagnostics` | `struct` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L20) | L20 |
| `TreeIndexHeader` | `struct` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L109) | L109 |
| `TreeIndexHeader::read_from_slice` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L124) | L124 |
| `TreeIndexHeader::write_to` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L159) | L159 |
| `TreeMoveStats` | `struct` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L182) | L182 |
| `TreeMoveStats::avg_black_elo` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L202) | L202 |
| `TreeMoveStats::avg_white_elo` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L194) | L194 |
| `TreePositionNode` | `struct` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L212) | L212 |
| `TreePositionNode::merge` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L277) | L277 |
| `TreePositionNode::new` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L222) | L222 |
| `TreePositionNode::record_game` | `fn` | [`src/tree_index/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/types.rs#L233) | L233 |
| `truncate_str` | `fn` | [`src/cli/formatters.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/formatters.rs#L1) | L1 |
| `try_rep_backward` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L1574) | L1574 |
| `try_rep_forward` | `fn` | [`src/search/path.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/path.rs#L1262) | L1262 |
| `undelete_selected_game` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L269) | L269 |
| `unload_index` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L454) | L454 |
| `unpack_date` | `fn` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L124) | L124 |
| `unpack_eco` | `fn` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L166) | L166 |
| `unpack_result` | `fn` | [`src/pgn_db/types.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/types.rs#L185) | L185 |
| `update_background` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L46) | L46 |
| `update_board_display` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L943) | L943 |
| `update_board_ui` | `func` | [`scripts/gui/widgets/board_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/board_widget.py#L236) | L236 |
| `update_cpu_hint` | `func` | [`scripts/gui/dialogs/settings_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/settings_dialog.py#L87) | L87 |
| `update_indexes_badge` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L452) | L452 |
| `update_indexes_badge` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L294) | L294 |
| `update_legal_moves_display` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L830) | L830 |
| `update_piece_slots` | `fn` | [`src/pgn_io/encoder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_io/encoder.rs#L132) | L132 |
| `update_progress` | `func` | [`scripts/gui/dialogs/build_pos_index_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/build_pos_index_dialog.py#L116) | L116 |
| `update_progress` | `func` | [`scripts/gui/dialogs/search_progress_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/search_progress_dialog.py#L75) | L75 |
| `update_slots_on_move` | `fn` | [`src/position_search/decoder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_search/decoder.rs#L131) | L131 |
| `update_stats` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L284) | L284 |
| `update_tab_titles` | `func` | [`scripts/gui/dialogs/advanced_search_dialog.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/dialogs/advanced_search_dialog.py#L525) | L525 |
| `update_tree_index_badge` | `func` | [`scripts/gui/widgets/opening_tree_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/opening_tree_widget.py#L222) | L222 |
| `update_ui_connected` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L264) | L264 |
| `update_ui_disconnected` | `func` | [`scripts/gui/main_window.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/main_window.py#L278) | L278 |
| `update_ui_disconnected` | `func` | [`scripts/gui/widgets/database_bar.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/database_bar.py#L270) | L270 |
| `validate_and_clauses` | `fn` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L107) | L107 |
| `validate_current_query` | `func` | [`scripts/gui/widgets/cql_search_widget.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/widgets/cql_search_widget.py#L588) | L588 |
| `validate_move_pattern` | `fn` | [`src/search/parser/moves.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L40) | L40 |
| `validate_piece_count` | `fn` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L915) | L915 |
| `validate_query_semantics` | `fn` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L13) | L13 |
| `validate_single_color_step` | `fn` | [`src/search/parser/moves.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/moves.rs#L87) | L87 |
| `validate_square_map` | `fn` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs#L54) | L54 |
| `VariableDomain` | `enum` | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs#L566) | L566 |
| `VirtualScidTableModel` | `class` | [`scripts/gui/models.py`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/scripts/gui/models.py#L8) | L8 |
| `wildcard_match` | `fn` | [`src/search/header.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/header.rs#L797) | L797 |
| `write_static_binary_file` | `fn` | [`src/position_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L382) | L382 |
| `write_static_binary_file` | `fn` | [`src/tree_index/builder.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L472) | L472 |
| `write_varint` | `fn` | [`src/position_index/codec.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/codec.rs#L65) | L65 |
| `write_varint` | `fn` | [`src/tree_index/codec.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/codec.rs#L183) | L183 |

