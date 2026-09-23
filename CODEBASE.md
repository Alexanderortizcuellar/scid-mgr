# Codebase Architecture & System Map (`CODEBASE.md`)

This document serves as the high-level architectural map for the `scid-mgr` codebase. It is designed to allow developers and AI coding agents to navigate, locate, and modify subsystems without scanning the entire repository.

---

## 1. Directory Structure & Subsystem Roles

```
scid-mgr/
├── src/                               # Core Rust library and engine implementation
│   ├── lib.rs                         # Library root, top-level exports & public API
│   ├── main.rs                        # CLI binary entry point and dispatch
│   ├── cli/                           # Modular CLI subcommands and formatting
│   │   ├── mod.rs                     # Clap CLI parser definition and execution router
│   │   ├── commands/                  # Command implementations (check, info, list, search, tree, index, etc.)
│   │   │   ├── check.rs               # Database health & companion index synchronization inspector
│   │   │   └── ...                    # Subcommand handlers
│   │   └── formatters.rs              # Output formatting (tables, JSON, progress)
│   ├── db/                            # SCID (.si4/.si5) database reader, writer, sorter, and editor
│   │   ├── mod.rs                     # Module exports and format detection
│   │   ├── types.rs                   # Core models (ScidFormat, GameSummary, GameFilter, DbStats)
│   │   ├── core.rs                    # ScidDatabaseWrapper struct and file I/O
│   │   ├── mutations.rs               # Game additions, updates, deletions, and compaction
│   │   ├── query.rs                   # Multi-criteria filtering, name ranking, and query caching
│   │   ├── sorting.rs                 # In-place and destination sorting engine
│   │   └── search_adapter.rs          # Position, material, and CQL search adapters
│   ├── pgn_db/                        # Fast memory-mapped/indexed PGN database wrapper
│   │   ├── mod.rs                     # PgnDatabaseWrapper public exports and type bindings
│   │   ├── types.rs                   # CompactPgnRecord, PgnNameTables, PgnIndexHeader, packing helpers
│   │   ├── core.rs                    # PgnDatabaseWrapper struct, companion index loader, query cache & pagination
│   │   ├── builder.rs                 # Parallel chunk scanner and .pgn.idx binary serialization
│   │   ├── sorting.rs                 # Column ranking, multi-column sorting, and sort_and_export
│   │   └── search_adapter.rs          # PositionFinder, MaterialFinder, and parallel CQL search
│   ├── pgn_io/                        # High-throughput PGN import (zero-copy ingest) & export pipeline
│   │   ├── mod.rs                     # Module exports and public bindings
│   │   ├── types.rs                   # ImportProgress, ExportProgress, FastNameTables, RawPgnTags
│   │   ├── encoder.rs                 # Fast SCID binary move encoder & piece slots tracker
│   │   ├── import.rs                  # Parallel memory-mapped PGN parser & zero-copy SCID ingester
│   │   └── export.rs                  # Parallel SCID-to-PGN text formatter & stream writer
│   ├── position_index/                # Inverted position index (.pos.idx) using Delta-Varint posting lists
│   │   ├── mod.rs                     # Position index public exports, tests, and bindings
│   │   ├── types.rs                   # PositionIndexHeader, PositionPostingList, IndexDiagnostics
│   │   ├── core.rs                    # Zero-copy PositionIndex memory-mapped candidate query engine
│   │   ├── codec.rs                   # Delta-Varint posting payload encoding/decoding, FEN parser
│   │   └── builder.rs                 # Parallel map-reduce inverted index builders and static binary writer
│   ├── position_search/               # Exact position, FEN, material predicate search, and SCID blob decoders
│   │   ├── mod.rs                     # Module exports and public search API bindings
│   │   ├── types.rs                   # PositionMatch, PositionSearchResult, MaterialFilter, PositionTargetMatcher
│   │   ├── decoder.rs                 # SCID raw blob move stream decoders, piece slots, and tag skipping
│   │   ├── parser.rs                  # Position matcher & piece placement parsers
│   │   └── scid_search.rs             # Memory-mapped parallel search engine for positions, placements, and material
│   ├── tree_index/                    # Dynamic opening tree index (.tree.idx) and statistics generator
│   │   ├── mod.rs                     # Tree index public exports and type bindings
│   │   ├── types.rs                   # PackedMove, Header, TreePositionNode, OpeningTreeReport
│   │   ├── core.rs                    # Zero-copy TreeIndex memory-mapped query engine
│   │   ├── codec.rs                   # Binary payload serialization/deserialization, varint, and FEN
│   │   ├── dynamic.rs                 # On-the-fly opening tree calculators for SCID and PGN
│   │   └── builder.rs                 # Parallel map-reduce tree index builders and static binary writer
│   ├── benchmark.rs                   # Benchmark runner and latency/throughput measurement harness
│   ├── server/                        # Interactive JSON-RPC stdin/stdout server for GUI integration
│   │   ├── mod.rs                     # Server loop, state management, and request dispatch
│   │   └── handlers/                  # Modular RPC command handlers (db, index, tree, search, config)
│   └── search/                        # CQLite (Chess Query Language Lite) AST, Parser & Evaluator
│       ├── mod.rs                     # Search module exports and query coordination
│       ├── query.rs                   # Query AST definitions (SearchQuery, QueryTerm, Filters)
│       ├── explain.rs                 # Query explanation, AST pretty-printer, DSL formatter
│       ├── transform.rs               # Board transformations & spatial symmetries (rotations, flips)
│       ├── header.rs                  # Header predicate matching (player, event, date, elo, eco, tag)
│       ├── pattern.rs                 # Piece placement and pawn structure patterns
│       ├── pawn.rs                    # Specialized pawn structure definitions (islands, passed, chains)
│       ├── squares.rs                 # Square set algebra and geometric rays (ranks, files, diagonals)
│       ├── tactics.rs                 # Tactical motif evaluators (pins, forks, skewers, discovered, mate)
│       ├── annotation.rs              # NAG annotations and move comments matcher
│       ├── path.rs                    # Piece path tracking and move sequence regex quantifier matching
│       ├── scid_adapter.rs            # Zero-copy query evaluator against SCID binary records
│       ├── evaluator/                 # Evaluation engine and fast move replayer
│       │   ├── mod.rs                 # Evaluator orchestration
│       │   ├── matcher.rs             # Query execution over games and positions
│       │   └── replayer.rs            # Shakmaty-based high-speed move replayer
│       ├── parser/                    # Complete CQLite tokenizer, grammar parser, and validator
│       │   ├── mod.rs                 # Main parser coordination
│       │   ├── lexer.rs               # Lexical tokenizer
│       │   ├── moves.rs               # Move sequences and move syntax parser
│       │   ├── tactics.rs             # Tactical motif syntax parser
│       │   ├── squares.rs             # Square sets, rays, and geometric syntax parser
│       │   ├── pieces.rs              # Piece count and placement syntax parser
│       │   ├── headers.rs             # Metadata and header filter syntax parser
│       │   ├── ranges.rs              # Numeric and ply range parser
│       │   ├── validator.rs           # Semantic validator & query optimizer
│       │   └── helpers.rs             # Common parsing helper routines
│       └── tests/                     # Modularized unit & regression tests for CQLite
├── tests/                             # Integration tests directory (cargo test)
│   └── integration_tests.rs           # End-to-end SCID/PGN lifecycle and index acceleration test suite
├── scripts/                           # Python GUI frontend and developer utilities
│   ├── gui/                           # PySide6 / PyQt desktop client
│   │   ├── main_window.py             # Main GUI application window and event wiring
│   │   ├── backend_client.py          # Async subprocess JSON-RPC client
│   │   ├── models.py                  # Qt table/tree data models
│   │   └── widgets/                   # Modular UI panels (board, tree, cql editor, database bar, etc.)
│   └── index_codebase.py              # Automated symbol & architecture index generator
├── instructions/                      # Project guides, refactoring guidelines, and specs
└── Cargo.toml                         # Rust package manifest & dependencies
```

---

## 2. Component Responsibility Matrix

| Component | File / Location | Key Responsibility |
|---|---|---|
| `ScidDatabaseWrapper` | [`src/db/`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/) | SCID `.si4`/`.si5` database access, decoding/encoding records, in-place sorting, flag updates |
| `PgnDatabaseWrapper` | [`src/pgn_db/`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/) | Fast indexed PGN database with binary index (`.pgn.idx`) and string table deduplication |
| `PositionIndex` | [`src/position_index/`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/) | Inverted position index (`.pos.idx`) mapping Zobrist position hashes to game candidate lists |
| `TreeIndex` | [`src/tree_index/`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/) | Opening tree index (`.tree.idx`) calculating move frequencies, win/draw/loss rates, ECO codes |
| `ZeroCopyIngest` | [`src/zero_copy_ingest.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/zero_copy_ingest.rs) | Stream parsing massive PGN chunks into memory/indices with minimal allocation overhead |
| `SearchQuery` / AST | [`src/search/query.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/query.rs) | Abstract Syntax Tree defining combined boolean queries, headers, tactics, piece paths, and positions |
| `GameSearchEvaluator` | [`src/search/evaluator/matcher.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs) | Evaluates `SearchQuery` expressions against games from SCID or PGN databases |
| `CqlParser` | [`src/search/parser/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/mod.rs) | Parses CQLite DSL queries into strongly-typed AST nodes |
| `QueryValidator` | [`src/search/parser/validator.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/parser/validator.rs) | Semantic validation and contradiction detection for CQL queries |
| `InteractiveServer` | [`src/server/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/server/mod.rs) | JSON-RPC server on stdin/stdout connecting GUI frontend to the engine via Rayon thread pool |
| `Cli` | [`src/cli/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/mod.rs) | Clap-based CLI interface for database inspection, listing, searching, indexing, and sorting |

---

## 3. Data & Control Flow

### A. Search Query Execution Flow
```
User Query String ──► Lexer (src/search/parser/lexer.rs)
                   ──► AST Parser (src/search/parser/)
                   ──► Semantic Validator (src/search/parser/validator.rs)
                   ──► Candidate Filter (src/position_index/core.rs / Header filter)
                   ──► Move Replayer & Evaluator (src/search/evaluator/)
                   ──► Result Set / Matches (QueryMatchResult)
```

### B. Opening Tree Query Flow
```
Board Position (FEN / UCI Moves) ──► TreeIndex::query_tree (src/tree_index/core.rs)
                                ──► Lookup Zobrist Hash in .tree.idx
                                ──► Extract Move Stats (White Win %, Draw %, Total Games)
                                ──► OpeningTreeReport / JSON Response
```

### C. GUI to Engine IPC Flow
```
Qt UI Event (e.g. click move) ──► BackendClient (scripts/gui/backend_client.py)
                              ──► JSON-RPC Request over stdin
                              ──► Interactive Server (src/server/mod.rs)
                              ──► Rayon Thread Pool Worker (src/server/handlers/)
                              ──► JSON-RPC Response over stdout
                              ──► Qt Signal / UI Update
```

---

## 4. Key Public APIs

### `scid_mgr::db`
* [`ScidDatabaseWrapper::open(path)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/core.rs#L40)
* [`ScidDatabaseWrapper::query_games(filter, page, page_size)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L20)
* [`ScidDatabaseWrapper::game_pgn(game_id)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/query.rs#L80)
* [`ScidDatabaseWrapper::sort_database(sort_field, desc)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/db/sorting.rs#L20)

### `scid_mgr::pgn_db`
* [`PgnDatabaseWrapper::open(path)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L30)
* [`PgnDatabaseWrapper::query_games(filter, page, page_size)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L380)
* [`PgnDatabaseWrapper::get_game_pgn(index)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/core.rs#L95)
* [`sort_pgn_file(input, output, sort_by, asc)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/pgn_db/sorting.rs#L160)

### `scid_mgr::position_index`
* [`PositionIndex::build_for_scid(db_path, ...)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/builder.rs#L140)
* [`PositionIndex::load(db_path)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L60)
* [`PositionIndex::get_matching_game_ids(zobrist_hash)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/position_index/core.rs#L110)

### `scid_mgr::tree_index`
* [`TreeIndex::build_for_scid(db_path, ...)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/builder.rs#L160)
* [`TreeIndex::load(db_path)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L60)
* [`TreeIndex::query_tree(fen_or_moves)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/tree_index/core.rs#L110)

### `scid_mgr::search`
* [`search::parse_cql(query_str)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/mod.rs#L15)
* [`GameSearchEvaluator::evaluate_game(game, query)`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/evaluator/matcher.rs#L45)

---

## 5. CLI Command Reference & Implementation Map

All CLI subcommands are parsed in [`src/cli/mod.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/mod.rs) and handled by dedicated command modules:

| Subcommand | Flag/Syntax | Handler Location | Description |
|---|---|---|---|
| `info` | `scid-mgr info <DB_PATH>` | [`src/cli/commands/info.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/info.rs) | Displays database headers, game counts, and format stats |
| `list` | `scid-mgr list <DB_PATH>` | [`src/cli/commands/list.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/list.rs) | Lists game metadata with pagination, sorting, and player/ECO filters |
| `search` | `scid-mgr search <DB_PATH> <QUERY>` | [`src/cli/commands/search.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/search.rs) | Executes CQLite search query across games |
| `tree` | `scid-mgr tree <DB_PATH>` | [`src/cli/commands/tree.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/tree.rs) | Queries opening tree statistics for FEN or move sequences |
| `index` | `scid-mgr index <DB_PATH>` | [`src/cli/commands/index.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/index.rs) | Builds or checks `.pos.idx` and `.tree.idx` companion indices |
| `sort` | `scid-mgr sort <DB_PATH>` | [`src/cli/commands/sort.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/sort.rs) | Sorts SCID or PGN database by any header criteria |
| `import` | `scid-mgr import <PGN_PATH> <SCID_PATH>` | [`src/cli/commands/import.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/import.rs) | Converts PGN games into SCID database |
| `bench` | `scid-mgr bench <DB_PATH>` | [`src/cli/commands/bench.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/cli/commands/bench.rs) | Executes search and retrieval performance benchmarks |
| `test` | `scid-mgr test` | [`src/test_suite.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/test_suite.rs) | Runs end-to-end integration test suite |

---

## 6. Testing & Benchmarking Guide

* **Unit & CQLite Tests**: Located in [`src/search/tests/`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/search/tests/) (65+ comprehensive test cases covering syntax, tactics, paths, symmetries, bitboards). Run via `cargo test`.
* **Integration Test Suite**: Located in [`src/test_suite.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/test_suite.rs), validates multi-threading, SCID in-place sorting, PGN indexing, and position search.
* **Benchmarks**: Located in [`src/benchmark.rs`](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/src/benchmark.rs), measures query throughput, memory efficiency, and disk I/O. Run via `scid-mgr bench <db>`.
