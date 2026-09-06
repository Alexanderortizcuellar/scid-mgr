# 🏗️ Architecture & End-to-End Workflow

This document explains step-by-step how `scid-mgr` operates, from loading binary files off NVMe/SSD storage to serving high-speed search and query requests over JSON-RPC.

---

## 1. High-Level Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│               Frontend / Client (e.g. Reference PyQt5 Test GUI)        │
│  - VirtualScidTableModel (Windowed Chunk Cache, Smooth Scroll)         │
│  - AdvancedSearchDialog (Board Editor, Bitboard Material, Metadata)    │
│  - OpeningTreeWidget (Instant Move Tree & Diagnostics)                 │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ JSON-RPC (stdin / stdout pipes)
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                     scid-mgr Rust Server Engine                        │
│  ┌──────────────────────────────┬───────────────────────────────────┐  │
│  │   DatabaseBackend::Scid      │      DatabaseBackend::Pgn         │  │
│  │   - In-Memory IndexEntry     │      - Parallel Chunk Parser      │  │
│  │   - Resolved NameTables      │      - Companion Binary .idx      │  │
│  │   - Games Memory-Map (Mmap)  │      - Memory-Mapped Raw Slicing  │  │
│  │   - OnceLock Rank Tables     │      - Rayon Parallel Sorting     │  │
│  └──────────────────────────────┴───────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                     Search & Analytics Core                      │  │
│  │   - Bitboard Material Search (Hardware light/dark squares)       │  │
│  │   - Zobrist Hash & Partial Board Placement Search                │  │
│  │   - Ultra-Fast PGN Exporter / Streaming Engine                   │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Zero-Copy Disk I/O
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        Physical Storage Files                          │
│  - SCID v5:  .si5 (Index), .sn5 (Names Journal), .sg5 (Move Stream)   │
│  - SCID v4:  .si4 (Index), .sn4 (Names Table),   .sg4 (Move Stream)   │
│  - PGN:      .pgn (Text),  .pgn.idx (Bincode Companion Cache)         │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Step-by-Step Workflow

### Step 1: Database Ingestion & Initialization
When `scid-mgr` opens a database (`.si5`, `.si4`, or `.pgn`):

1. **Format Detection**:
   - Checks file extension and companion files.
2. **For SCID Databases (`.si5` / `.si4`)**:
   - **Index File (`.si5`/`.si4`)**: Read into a contiguous vector of fixed-size `IndexEntry` structs in RAM. For 10.35 million games, this takes only **~680 ms**.
   - **Namebase (`.sn5`/`.sn4`)**: Reconstructed into resolved neutral `NameTables` vectors (Players, Events, Sites, Rounds).
   - **Games Move-Stream (`.sg5`/`.sg4`)**: Memory-mapped using `memmap2::Mmap`. Move data is not loaded into memory until a specific game is decoded, ensuring low RAM usage (~580 MB for 10.35M games).
3. **For Plain Text PGN Files (`.pgn`)**:
   - Checks for companion `.pgn.idx`. If present, loads instantly (< 10 ms).
   - If not present, spawns Rayon threads across all CPU cores to parse headers in 1 pass (~250,000 games/sec) and saves the `.pgn.idx` cache.

---

### Step 2: Querying, Filtering & Search
When a user searches or filters (e.g. `Kasparov` + `B90` + `Opposite Bishops`):

1. **Parallel Predicate Filtering**:
   - If no filters are active, `(0..total).collect()` yields all indices in ~3 ms.
   - If filters are active, `entries.par_iter().enumerate().filter_map(...)` evaluates bitwise criteria across all CPU cores in parallel.
2. **Name Resolution Optimization**:
   - String matching on player names is performed **once on the unique name table** (e.g. 544,000 strings), generating a compact boolean bitmask `player_matches: Vec<bool>`.
   - Each game's `white_id` and `black_id` are tested via $O(1)$ integer array lookup `player_matches[white_id]`, eliminating redundant string comparisons across 10 million games.
3. **Bitboard Material Search**:
   - Uses pre-extracted bitboard signatures in the index to filter endgames and piece compositions in microseconds without decoding move streams.

---

### Step 3: High-Performance Parallel Sorting
When the user clicks a column header (e.g. `White Player`):

1. **Lazy Rank Table Generation**:
   - On the first sort by name, the engine sorts the unique names once (`544k` items in ~114 ms) and populates a `OnceLock<Vec<u32>>` alphabetical rank array.
2. **Integer Register Comparisons**:
   - During the 10.35M game sort, comparing player names is reduced to comparing two `u32` rank values (`ranks[entry_a.white_id] <=> ranks[entry_b.white_id]`).
3. **Rayon Parallel Quicksort**:
   - `par_sort_unstable_by` partitions the index array across all CPU cores, sorting **10.35 million items in under 2 seconds**.

---

### Step 4: Windowed Pagination & GUI Streaming
1. **Window Slicing**:
   - The backend extracts only the requested page slice (e.g. 50 games: `page * 50 .. (page + 1) * 50`).
2. **JSON-RPC Transmission**:
   - Emits a JSON payload with game summaries (`ID`, `White`, `Black`, `WhiteElo`, `BlackElo`, `Date`, `ECO`, `Result`, `Event`, `Site`).
3. **PyQt5 Virtual Table Model**:
   - `VirtualScidTableModel` maintains a local chunk cache.
   - When the user scrolls, a **150 ms debounce timer** ensures requests are only sent when scrolling pauses, maintaining a rock-solid **60 FPS** UI response.

---

### Step 5: On-Demand Game Decoding & PGN Reconstruction
When the user selects a game row:

1. **Zero-Copy Slice**:
   - Extracts the exact game byte slice from the memory-mapped `games.sg5` or `.pgn` file.
2. **Huffman / Bitstream Move Decoding**:
   - `chess-scid-rw::pgn_build` decodes the binary move tokens into Standard Algebraic Notation (SAN), reconstructing comments, variations, NAGs, and header tags.
3. **Instant Display**:
   - The full PGN text is rendered in the `PGN Game Text` viewer with zero perceptible latency (< 0.5 ms).
