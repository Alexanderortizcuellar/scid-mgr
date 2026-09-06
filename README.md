# SCID Database Manager (`scid-mgr`)

A high-performance, multithreaded Rust chess database engine, CLI utility, and JSON-RPC backend server for Shane's Chess Information Database (SCID) formats (**si4** and **si5**) and **PGN** archives, powered by pure Rust.

---

## 📚 Documentation & Technical Guides

Comprehensive technical documentation is available in the [`docs/`](docs/) directory:
- 📖 [**Architecture & End-to-End Workflow**](docs/ARCHITECTURE_AND_WORKFLOW.md)
- ⚡ [**Performance & Engineering Optimizations**](docs/PERFORMANCE_AND_OPTIMIZATIONS.md)
- 📊 [**Benchmarks & Performance Metrics (10.35M Games)**](docs/BENCHMARKS_AND_METRICS.md)
- 📜 [**CQLi (Chess Query Language) Integration Guide**](docs/CQL_INTEGRATION_GUIDE.md)
- 🔌 [**JSON-RPC Server API Reference**](docs/API_REFERENCE.md)

---

## ⚡ Core Engine Features

- **Multi-Format Database Engine**:
  - Native decoding, encoding, and compacting of SCID **si5** (`.si5`/`.sn5`/`.sg5`) and **si4** (`.si4`/`.sn4`/`.sg4`).
  - Fast 1-pass parallel indexing and zero-copy streaming of **PGN** (`.pgn`) archives with binary companion cache (`.pgn.idx`).
- **High-Performance In-Memory & Memory-Mapped Indexing**:
  - Sub-millisecond queries, multi-criteria header filtering, parallel sorting with alphabetical rank tables, and instant statistics.
  - Zero-copy memory mapping (`memmap2`) for move streams with low memory footprint (~580 MB for 10.35 million games).
- **Sub-Millisecond Companion Position Index (`.pos.idx` v3)**:
  - Custom inverted position index format with sorted Zobrist 64-bit keys and Delta-Varint posting list compression.
  - Instant Opening Tree / Explorer (< 0.05 ms lookup time) returning move win/draw/loss statistics, average ratings, and sample game IDs.
  - Dynamic opening tree filtering by metadata (player, rating, date, ECO, custom candidate game lists).
- **Advanced Search Engine**:
  - **Position Search**: Zobrist-hashed position lookup with automatic `.pos.idx` candidate acceleration and multi-threaded fallback move-stream scanning.
  - **Partial Board & Piece Placement Search**: Search arbitrary square configurations (e.g. Queen on d4, King on g1).
  - **Hardware Bitboard Material Search**: Fast endgame and piece combination search (e.g. opposite/same-colored bishops, specific piece counts) evaluated in microseconds via CPU bitwise instructions.
  - **Multi-Attribute Header Filtering**: White/Black/Any player, result, ECO prefix, date range, rating filters, event, site, and round.
- **Full PGN Reconstruction**:
  - Reconstructs Seven Tag Roster, move lists, variations, annotations/comments, NAGs, and custom start positions (FEN).
- **Database Mutations**:
  - `add_game`: Encodes full PGN into SCID move binary format and updates name tables and index.
  - `update_game`: Replaces existing game tags and moves.
  - `delete_game` / `undelete_game`: Toggles standard SCID deletion flags.
  - `compact`: Reclaims dead space from updated/deleted games.
  - `save`: Atomically writes the index, namebase, and games files to disk.
- **High-Speed PGN Ingestion & Export**:
  - Ultra-fast native Rust bitboard ingest (up to ~230,000 games/sec).
  - High-throughput streaming PGN exporter.
- **Interactive JSON-RPC Protocol**:
  - Full-duplex NDJSON protocol over `stdin`/`stdout` for integrating with frontends, GUIs, web servers, or external tooling.

> **Note on GUI**: A reference PyQt5 desktop client is located in [`scripts/`](scripts/scid_gui.py). It serves as a testing harness to exercise and visually verify the JSON-RPC backend capabilities during development.

---

## 🚀 Quick Start & CLI Reference

### 1. Build and Run the Test Suite

```powershell
# Run the Rust unit & integration test suite
cargo test

# Or run the built-in end-to-end integration test command
cargo run --release -- test
```

### 2. Common CLI Commands

```powershell
# Display database statistics and metadata
.\target\release\scid-mgr.exe info games\sample.si5

# List games with pagination, filtering, and sorting
.\target\release\scid-mgr.exe list games\sample.si5 --page 0 --page-size 20 --player "Kasparov" --sort-by date --desc

# Retrieve game #4 reconstructed in standard PGN format
.\target\release\scid-mgr.exe get games\sample.si5 4

# Ultra-Fast Native Rust Bitboard Ingest (280,000 games in ~1.2s at 230,000 games/s)
.\target\release\scid-mgr.exe import my_database.si5 "C:\path\to\archive.pgn" --format si5

# Search for a position by FEN across all games (uses .pos.idx candidate acceleration if available)
.\target\release\scid-mgr.exe search-pos my_database.si5 "rnbqkb1r/1p2pppp/p2p1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 6"

# Partial Board / Piece Placement Search (e.g. White Queen on d4)
.\target\release\scid-mgr.exe search-pos my_database.si5 "8/8/8/8/3Q4/8/8/8"

# High-Speed Bitboard Material Search (e.g. White Queen vs Black No Queen)
.\target\release\scid-mgr.exe search-mat my_database.si5 --wq 1 --bq 0

# Build Companion Position Index (.pos.idx v3) for Sub-Millisecond Opening Tree (< 0.05 ms)
.\target\release\scid-mgr.exe build-pos-idx my_database.si5 --max-ply 24

# Build Compact Position Index (Filter out rare positions with < 5 occurrences)
.\target\release\scid-mgr.exe build-pos-idx my_database.si5 --max-ply 24 --min-games 5

# Query Instant Opening Tree with sample game IDs
.\target\release\scid-mgr.exe tree my_database.si5 --fen "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1" --sample-games 20

# Run Diagnostics on Companion Position Index (Delta-Varint compression and posting list distribution)
.\target\release\scid-mgr.exe diag-pos-idx my_database.si5

# Run Comprehensive Multi-Threaded Engine Benchmarks
.\target\release\scid-mgr.exe bench my_database.si5

# Export SCID database to PGN file
.\target\release\scid-mgr.exe export my_database.si5 exported_games.pgn
```

### 3. Launch Interactive JSON-RPC Server Mode

Run `scid-mgr` as a persistent background daemon communicating over standard I/O pipes:

```powershell
.\target\release\scid-mgr.exe interactive my_database.si5
# or
.\target\release\scid-mgr.exe -i my_database.si5
```

Send single-line JSON requests over `stdin` and receive single-line JSON responses over `stdout`. See [`docs/API_REFERENCE.md`](docs/API_REFERENCE.md) for full protocol specifications and commands.

### 4. Running the Development Test Harness (Optional)

To launch the reference PyQt5 test client:

```powershell
python scripts\scid_gui.py
```
