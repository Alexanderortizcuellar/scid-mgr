# 📈 Chapter 9: Common Continuations Engine & Sequence Analysis

The **Common Continuations Engine** is a high-performance sequence analyzer and game continuation explorer in `scid-mgr`. It calculates the most frequent multi-move continuation paths and branches from any chess position (full FEN or starting board), providing game counts, percentage shares, win/draw/loss distributions, and score statistics.

---

## ⚡ Architecture: Dual Execution Modes

The engine operates under two complementary modes:

### 1. Precomputed Memory-Mapped Graph (`.hot.idx`)
* **Zero-Allocation Binary Graph**: Serialized directed acyclic graph storing precomputed position nodes, outgoing move edges, and frequency statistics.
* **Instant Lookup**: Traverses top continuation lines in sub-milliseconds without disk I/O or position re-parsing.
* **Striped Parallel Builders**: Index builder processes massive PGN or native SCID databases across multiple worker threads, writing atomic `.tmp` files.

#### Binary Graph Layout:
* **Header (96 bytes)**: Magic bytes (`CHSHOTG1`), version, total nodes, total edges, max ply, flags, and direct offset index tables.
* **Node Table (32 bytes per node)**: Zobrist hash, occurrence counts, white wins, draws, black wins, edge count, and first-edge offset.
* **Edge Table (16 bytes per edge)**: Move key (16-bit packed move), target node index, and traversal count.

### 2. On-The-Fly Candidate-Accelerated Dynamic Search
* **Zero Index Prerequisite**: Analyzes any custom or deep position directly from the database without requiring a prebuilt `.hot.idx` file.
* **Companion `.pos.idx` Inverted Index Acceleration**: Quickly narrows down the game search space using candidate game posting lists before replaying moves in parallel.

---

## 🖥️ Command-Line Interface (CLI)

### 1. Querying Continuations
```bash
# Query continuations from starting position
scid-mgr continuations database.si5

# Query continuations from specific FEN position
scid-mgr continuations database.si5 "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3"

# Specify search depth, max lines, and minimum game threshold
scid-mgr continuations database.si5 --depth 12 --lines 10 --min-games 5 --min-pct 1.0
```

### 2. Building the `.hot.idx` Index
```bash
# Build hot continuations graph up to 16 plies (8 full moves)
scid-mgr build continuations database.si5 --max-ply 16 --min-games 3

# Build for PGN database
scid-mgr build continuations games.pgn --max-ply 20 --min-games 5
```

---

## 💬 Interactive REPL

Inside the `scid-mgr` REPL, inspect continuations dynamically:

```text
scid-mgr> .continuations
scid-mgr> .continuations r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3
```

---

## 🔌 JSON-RPC Server API

### `continuations`
Retrieves top continuation sequences for a position.
* **Params**:
  * `fen`: `string` (position FEN, defaults to starting board)
  * `max_depth`: `number` (optional, default: `8` plies)
  * `max_lines`: `number` (optional, default: `10` lines)
  * `min_games`: `number` (optional, default: `1`)
  * `min_percentage`: `number` (optional, default: `0.0`)
  * `include_tree`: `boolean` (optional, default: `false`)
* **Returns**:
  ```json
  {
    "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
    "total_games_processed": 10000,
    "games_reaching_position": 4850,
    "lines": [
      {
        "moves": ["c5", "Nf3", "d6", "d4", "cxd4", "Nxd4"],
        "formatted": "1... c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4",
        "games": 1820,
        "percentage": 37.5,
        "white_wins": 680,
        "draws": 590,
        "black_wins": 550
      }
    ]
  }
  ```

### `build_continuations`
Triggers background creation of the `.hot.idx` file and streams `build_continuations_progress` events.
* **Params**:
  * `output_path`: `string` (optional target `.hot.idx` path)
  * `max_ply`: `number` (maximum plies to index, e.g. `16`)
  * `min_games`: `number` (minimum games for inclusion, e.g. `2`)
* **Events Emitted**:
  ```json
  {
    "event": "build_continuations_progress",
    "data": {
      "phase": "Indexing games",
      "games_processed": 50000,
      "total_games": 100000,
      "nodes_indexed": 125400,
      "percent": 50.0
    }
  }
  ```

---

## 🎨 SCID GUI Integration

The graphical interface includes a dedicated **📈 Continuations** tab:
1. **Interactive Chessboard**: Play moves directly on the board to recalculate continuation trees in real-time.
2. **Parameters Bar**: Live adjustment of maximum depth (plies), maximum lines, minimum games, and minimum percentage filters.
3. **Continuations Table**: Displays ranking, full SAN move sequence, total games, share percentage, White win %, Draw %, Black win %, and calculated score.
4. **Interactive Line Play & Export**: Double-click any line or click **▶ Play Line** to step through the sequence on the board; click **📋 Copy Moves** to copy SAN notation to clipboard.
5. **Hot Index Builder Dialog**: Access the **⚡ Build .hot.idx** action from the toolbar or the Tools menu to generate binary graph files with real-time progress bars.
