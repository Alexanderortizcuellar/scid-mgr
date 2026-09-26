# 📈 Chapter 9: Common Continuations Engine & Sequence Analysis

The **Common Continuations Engine** is a high-performance sequence analyzer and game continuation explorer in `scid-mgr`. It calculates the most frequent multi-move continuation paths and branches from any chess position (full FEN or starting board), providing game counts, percentage shares, win/draw/loss distributions, and score statistics.

---

## ⚡ Architecture: Dual Execution Modes

The engine operates under two complementary modes:

### 1. Precomputed Memory-Mapped Graph (`.hot.idx`)
* **Zero-Allocation Binary Graph**: Serialized directed acyclic graph storing precomputed position nodes, outgoing move edges, and frequency statistics.
* **Instant Lookup**: Traverses top continuation lines in sub-milliseconds without disk I/O or position re-parsing.
* **Striped Parallel Builders**: Index builder processes massive PGN or native SCID databases across multiple worker threads, writing atomic `.tmp` files.

#### Binary Graph Format (`.hot.idx`):

```
+-------------------------------------------------------------+
| Header (96 Bytes)                                           |
| - magic: b"CHSHOTG1" (8B)                                   |
| - version: u32 (4B)                                         |
| - flags: u32 (4B)                                           |
| - db_mtime_secs: u64 (8B)                                   |
| - db_file_size: u64 (8B)                                    |
| - db_game_count: u64 (8B)                                   |
| - max_ply: u32 (4B)                                         |
| - min_games: u32 (4B)                                       |
| - node_count: u32 (4B)                                      |
| - edge_count: u32 (4B)                                      |
| - hash_count: u32 (4B)                                      |
| - _reserved: u32 (4B)                                       |
| - nodes_offset: u64 (8B)                                    |
| - edges_offset: u64 (8B)                                    |
| - hashes_offset: u64 (8B)                                   |
| - created_timestamp: u64 (8B)                               |
+-------------------------------------------------------------+
| Nodes Array [HotNode; node_count] (24 Bytes per node)       |
+-------------------------------------------------------------+
| Edges Array [HotEdge; edge_count] (24 Bytes per edge)       |
+-------------------------------------------------------------+
| Hash Map Entries [HotHashEntry; hash_count] (12 Bytes each) |
+-------------------------------------------------------------+
```

##### 1. Header Layout (96 Bytes)

| Offset | Field | Type | Description |
| :--- | :--- | :--- | :--- |
| `0x00..0x08` | `magic` | `[u8; 8]` | Magic identifier: `b"CHSHOTG1"` |
| `0x08..0x0C` | `version` | `u32` | Format version (currently `1`) |
| `0x0C..0x10` | `flags` | `u32` | Reserved flags |
| `0x10..0x18` | `db_mtime_secs` | `u64` | Source database timestamp (seconds) |
| `0x18..0x20` | `db_file_size` | `u64` | Source database file size in bytes |
| `0x20..0x28` | `db_game_count` | `u64` | Total games in source database |
| `0x28..0x2C` | `max_ply` | `u32` | Maximum indexing ply depth (e.g. 24) |
| `0x2C..0x30` | `min_games` | `u32` | Minimum games threshold for inclusion |
| `0x30..0x34` | `node_count` | `u32` | Total unique position nodes in graph |
| `0x34..0x38` | `edge_count` | `u32` | Total directed transition edges |
| `0x38..0x3C` | `hash_count` | `u32` | Total hash table lookup entries |
| `0x3C..0x40` | `_reserved` | `u32` | Reserved alignment padding |
| `0x40..0x48` | `nodes_offset` | `u64` | Byte offset to `HotNode` array |
| `0x48..0x50` | `edges_offset` | `u64` | Byte offset to `HotEdge` array |
| `0x50..0x58` | `hashes_offset`| `u64` | Byte offset to sorted `HotHashEntry` array |
| `0x58..0x60` | `created_timestamp`| `u64`| UNIX timestamp (seconds) when index was built |

##### 2. Node Layout (`HotNode`, 24 Bytes)

| Offset | Field | Type | Description |
| :--- | :--- | :--- | :--- |
| `0x00..0x04` | `first_edge` | `u32` | Index into edge array of the first outgoing move |
| `0x04..0x06` | `edge_count` | `u16` | Number of outgoing move branches |
| `0x06..0x08` | `_padding` | `u16` | Alignment padding |
| `0x08..0x0C` | `total_games`| `u32` | Total games reaching this position |
| `0x0C..0x10` | `white_wins` | `u32` | Games won by White (1-0) |
| `0x10..0x14` | `draws` | `u32` | Games drawn (1/2-1/2) |
| `0x14..0x18` | `black_wins` | `u32` | Games won by Black (0-1) |

##### 3. Edge Layout (`HotEdge`, 24 Bytes)

| Offset | Field | Type | Description |
| :--- | :--- | :--- | :--- |
| `0x00..0x02` | `packed_move` | `u16` | 16-bit packed move encoding (from_sq, to_sq, promo) |
| `0x02..0x04` | `_padding` | `u16` | Alignment padding |
| `0x04..0x08` | `target_node` | `u32` | `NodeId` of resulting position (or `NO_NODE`) |
| `0x08..0x0C` | `total_games` | `u32` | Games continuing with this move |
| `0x0C..0x10` | `white_wins` | `u32` | White wins after this move |
| `0x10..0x14` | `draws` | `u32` | Draws after this move |
| `0x14..0x18` | `black_wins` | `u32` | Black wins after this move |

##### 4. Hash Index Entry (`HotHashEntry`, 12 Bytes, Packed)

| Offset | Field | Type | Description |
| :--- | :--- | :--- | :--- |
| `0x00..0x08` | `hash` | `u64` | 64-bit Zobrist hash of position |
| `0x08..0x0C` | `node_id` | `u32` | Index into `HotNode` table |

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
