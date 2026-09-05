# SCID Database Manager (`scid-mgr`) & PyQt5 Virtual Scrolling GUI

A high-performance Rust backend and PyQt5 GUI for Shane's Chess Information Database (SCID) formats (**si4** and **si5**), powered by the pure-Rust [`chess-scid-rw`](https://crates.io/crates/chess-scid-rw) crate.

---

## 📚 Documentation & Technical Guides

Comprehensive documentation is available in the [`docs/`](docs/) directory:
- 📖 [**Architecture & End-to-End Workflow**](docs/ARCHITECTURE_AND_WORKFLOW.md)
- ⚡ [**Performance & Engineering Optimizations**](docs/PERFORMANCE_AND_OPTIMIZATIONS.md)
- 📊 [**Benchmarks & Performance Metrics (10.35M Games)**](docs/BENCHMARKS_AND_METRICS.md)
- 📜 [**CQLi (Chess Query Language) Integration Guide**](docs/CQL_INTEGRATION_GUIDE.md)
- 🔌 [**JSON-RPC Server API Reference**](docs/API_REFERENCE.md)

---

## Architecture & Features

### 1. Rust Core Engine (`scid-mgr`)
- **Multi-Format Database Engine**: Native decoding and encoding of SCID **si4** (`.si4`/`.sn4`/`.sg4`), **si5** (`.si5`/`.sn5`/`.sg5`), and direct indexing of **PGN** (`.pgn`) archives.
- **High-Performance In-Memory & Memory-Mapped Indexing**: Sub-millisecond queries, multi-criteria filtering, parallel sorting, and instant statistical aggregation.
- **Full PGN Reconstruction**: Accurate Seven Tag Roster, move lists, variations, annotations/comments, NAGs, and custom FEN starting positions.
- **Advanced Search Engine**:
  - Exact & Partial FEN / Position Board Search across move streams.
  - Piece & Material Combination Search with any-ply / final-position scoping.
  - Fast Header Filtering (Players, Result, ECO, Date range, Event, Site, Status).
- **Database Mutations**:
  - `add_game`: Encodes full PGN into SCID move binary format and updates name tables and index.
  - `update_game`: Replaces existing game tags/moves.
  - `delete_game` / `undelete_game`: Sets standard SCID delete flag.
  - `compact`: Reclaims dead space from updated games.
  - `save`: Atomically writes the index, namebase, and games files to disk.
- **PGN Ingest & Export**: Fast multi-game streaming parser to import/export `.pgn` files.
- **Interactive JSON-RPC Protocol**: Communicates with GUI frontends over `stdin`/`stdout` using NDJSON messages.

### 2. PyQt5 GUI Client (`scripts/scid_gui.py`)
- **True On-Stop Virtual Scrolling**:
  - Passive model with zero pipe calls during rendering.
  - 150ms debounce timer triggers chunk fetching only when scrolling settles.
  - 60+ FPS smooth scrolling without UI freezing.
- **Database Management**:
  - Create new `.si5` or `.si4` database.
  - Open existing databases.
  - Import multi-game PGNs.
  - Export database to PGN.
  - Compact & Save.
- **Search & Filters**:
  - Player (Any, White, Black), Result (`1-0`, `0-1`, `1/2-1/2`, `*`), ECO prefix, Date pattern, Event, Site.
  - Deleted game filtering (Include / Only Deleted).
- **Game Viewer**:
  - Full PGN text with tags and moves.
  - Copy PGN button.
  - Add / Edit / Delete / Undelete game dialogs.
- **Live Protocol Log**:
  - Real-time NDJSON RPC inspection.

---

## Quick Start

### 1. Run the Test Suite
Validates roundtrip creation, game addition, variations, custom FEN, queries, updating, deleting, compacting, saving, and reopening in both `si4` and `si5`:

```powershell
cargo run --release -- test
```

### 2. CLI Commands
```powershell
# Display database statistics
.\target\release\scid-mgr.exe info games\sample.si5

# List games with pagination and filtering
.\target\release\scid-mgr.exe list games\sample.si5 --page 0 --page-size 20

# View game #4 in PGN format
.\target\release\scid-mgr.exe get games\sample.si5 4

# Ultra-Fast Native Rust Bitboard Ingest (280,000 games in ~1.2s at 230,000 games/s)
.\target\release\scid-mgr.exe import my_database.si5 "C:\Users\ASUS\chess\pgn\lichess_elite_2025-11.pgn" --format si5

# Ultra-Fast Binary Move Stream Position Search (280,000 games in ~0.3s)
.\target\release\scid-mgr.exe search-pos my_database.si5 "rnbqkb1r/1p2pppp/p2p1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 6"

# Partial Board / Piece Placement Search (e.g. Single Queen on d4)
.\target\release\scid-mgr.exe search-pos my_database.si5 "8/8/8/8/3Q4/8/8/8"

# High-Speed Material Search (e.g. White Queen vs Black No Queen)
.\target\release\scid-mgr.exe search-mat my_database.si5 --wq 1 --bq 0

# Export SCID database to PGN
.\target\release\scid-mgr.exe export my_database.si5 exported_games.pgn
```

### 3. Launch the PyQt5 GUI
```powershell
python scripts\scid_gui.py
```
