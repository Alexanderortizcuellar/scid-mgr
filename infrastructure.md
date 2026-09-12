# SCID-MGR Companion Index Infrastructure

This document provides a comprehensive technical specification of the high-performance binary companion index formats used in **`scid-mgr`**:
1. **`.pos.idx`** — Inverted Position Search Booster (Magic: `SCIDPOS5`, Version `5`)
2. **`.tree.idx`** — Opening Tree & Move Statistics Index (Magic: `SCIDTRE1`, Version `1`)

---

## 1. Architectural Overview & Decoupling Rationale

In modern chess database management, candidate position searching and opening repertoire exploration have fundamentally different access patterns and data requirements:

```
                                  [ Raw Database (.si5 / .si4 / .pgn) ]
                                                    │
                   ┌────────────────────────────────┴────────────────────────────────┐
                   ▼                                                                 ▼
   ┌───────────────────────────────┐                                 ┌───────────────────────────────┐
   │     Position Search Booster   │                                 │       Opening Tree Stats      │
   │           (.pos.idx)          │                                 │          (.tree.idx)          │
   ├───────────────────────────────┤                                 ├───────────────────────────────┤
   │ • Pure Inverted Index         │                                 │ • Repertoire Move Statistics  │
   │ • ZobristHash ➔ [Game IDs]    │                                 │ • ZobristHash ➔ [W/D/L, Elos] │
   │ • Zero-Byte Inlined Singletons│                                 │ • Instant Move Tree Rendering │
   │ • Accelerated Candidate Filter│                                 │ • Zero Game ID overhead       │
   └───────────────────────────────┘                                 └───────────────────────────────┘
```

### Why Separate the Indexes?
- **Position Search (`.pos.idx`)** requires knowing **which games** reached a specific position, so the search engine can filter millions of candidate games in $< 0.01\text{ ms}$ without reading game bodies. It does not require move branching stats, Elo averages, or win/loss percentages.
- **Opening Explorer (`.tree.idx`)** requires knowing **move branching frequencies**, $W/D/L$ outcome rates, and average player ratings for every legal continuation from a position. Storing millions of individual Game IDs in the tree index causes unnecessary bloat.

By decoupling them into specialized binary files, both indexes achieve optimal memory-mapped efficiency and cache locality.

---

## 2. Position Search Booster (`.pos.idx` — Version 5)

### 2.1 File Structure

A `.pos.idx` file is divided into three sequential contiguous sections:

```
+───────────────────────────────────────────────────────────────────────────+
|                           Header (64 bytes)                               |
+───────────────────────────────────────────────────────────────────────────+
|                 Directory Index Table (Sorted Array of Entries)           |
|                [ Zobrist Hash (8 bytes) | Data Offset (4 bytes) ]         |
|    * If single game (Singleton): data_offset = INLINE_FLAG | game_id      |
|      (Requires 0 bytes of payload in the data section!)                   |
+───────────────────────────────────────────────────────────────────────────+
|                 Postings Data Payload (Delta-Varint Compressed)           |
|            [ count: varint | delta_game_id: varint | ... ]                |
+───────────────────────────────────────────────────────────────────────────+
```

### 2.2 Header Layout (64 Bytes)

| Offset | Field Name | Type | Description |
| :--- | :--- | :--- | :--- |
| `0x00..0x08` | `magic` | `[u8; 8]` | Magic identifier: `b"SCIDPOS5"` |
| `0x08..0x0C` | `version` | `u32` | Format version (Current: `5`) |
| `0x0C..0x10` | `flags` | `u32` | Format feature flags (Reserved: `0`) |
| `0x10..0x18` | `db_mtime_secs` | `u64` | Source database file modification timestamp (UNIX epoch) |
| `0x18..0x20` | `db_size_bytes` | `u64` | Source database file size in bytes |
| `0x20..0x28` | `db_game_count` | `u64` | Total number of games indexed |
| `0x28..0x2C` | `max_ply_depth` | `u32` | Maximum indexing ply depth (e.g. `24`) |
| `0x2C..0x30` | `unique_positions` | `u32` | Total number of unique Zobrist position keys |
| `0x30..0x38` | `index_offset` | `u64` | Byte offset of Directory Index Table (`0x40`) |
| `0x38..0x40` | `data_offset` | `u64` | Byte offset of Postings Data Payload |

### 2.3 Directory Index Table Entry (12 Bytes) & Inlined Singletons Optimization

The Directory Table is stored as a strictly sorted array of `SortedIndexEntry` structs:

```rust
#[repr(C, packed)]
pub struct SortedIndexEntry {
    pub hash: u64,         // 64-bit Zobrist position hash
    pub data_offset: u32,  // Relative byte offset in Payload OR Inlined Game ID
}
```

#### Inlined Singleton Tagged Pointer (`INLINE_FLAG = 0x8000_0000`):
In large chess databases, **~80% of unique positions appear in exactly 1 game** (singletons).
- If a position occurs in **exactly 1 game**:
  $$\text{data\_offset} = \text{0x8000\_0000} \;\vert\; \text{game\_id}$$
  The Game ID is stored directly inside the directory table entry. **0 bytes are allocated in the payload section**.
- If a position occurs in **$> 1$ games**:
  $$\text{data\_offset} = \text{offset in data payload}$$
  The delta-varint compressed list of Game IDs is decoded from the payload.

### 2.4 Multi-Game Postings Payload Encoding (Delta-Varint)

For multi-game positions ($\text{count} > 1$):
1. **`count`** (`varint`): Number of games reaching this position.
2. **Game IDs Sequence** (repeated `count` times):
   - **`delta_game_id`** (`varint`): Difference between current `game_id` and preceding `game_id` ($\Delta = \text{game\_id}_i - \text{game\_id}_{i-1}$, with $\text{game\_id}_0 = 0$).

---

## 3. Opening Tree Stats Index (`.tree.idx` — Version 1)

### 3.1 File Structure

A `.tree.idx` file stores opening tree branches, win/draw/loss counts, and Elo rating sums for every indexed position:

```
+───────────────────────────────────────────────────────────────────────────+
|                           Header (64 bytes)                               |
+───────────────────────────────────────────────────────────────────────────+
|                 Directory Index Table (Sorted Array of Entries)           |
|                [ Zobrist Hash (8 bytes) | Data Offset (4 bytes) ]         |
+───────────────────────────────────────────────────────────────────────────+
|                      Tree Node Statistics Payload                         |
|   [ total_games: varint | white_wins: varint | black_wins: varint ]       |
|   [ move_count: varint ]                                                  |
|   ┌─ Move Branch Record ──────────────────────────────────────────────┐   |
|   │ • packed_move: u16 (from_sq, to_sq, promo)                        │   |
|   │ • white_wins: varint | draws: varint | black_wins: varint         │   |
|   │ • avg_white_elo: u16 | avg_black_elo: u16                         │   |
|   └───────────────────────────────────────────────────────────────────┘   |
+───────────────────────────────────────────────────────────────────────────+
```

### 3.2 Header Layout (64 Bytes)

| Offset | Field Name | Type | Description |
| :--- | :--- | :--- | :--- |
| `0x00..0x08` | `magic` | `[u8; 8]` | Magic identifier: `b"SCIDTRE1"` |
| `0x08..0x0C` | `version` | `u32` | Format version (Current: `1`) |
| `0x0C..0x10` | `flags` | `u32` | Format feature flags (Reserved: `0`) |
| `0x10..0x18` | `db_mtime_secs` | `u64` | Source database file modification timestamp |
| `0x18..0x20` | `db_size_bytes` | `u64` | Source database file size in bytes |
| `0x20..0x28` | `db_game_count` | `u64` | Total number of games indexed |
| `0x28..0x2C` | `max_ply_depth` | `u32` | Maximum indexing ply depth (e.g. `24`) |
| `0x2C..0x30` | `unique_positions` | `u32` | Total number of unique position nodes |
| `0x30..0x38` | `index_offset` | `u64` | Byte offset of Directory Index Table (`0x40`) |
| `0x38..0x40` | `data_offset` | `u64` | Byte offset of Tree Node Payload |

### 3.3 Packed Move Representation (`PackedMove` — 16 Bits)

Move branches are encoded as a compact 16-bit integer:

$$\text{Bit Layout: } \underbrace{\text{from\_square}}_{6\text{ bits (0..63)}} \;\vert\; \underbrace{\text{to\_square}}_{6\text{ bits (0..63)}} \;\vert\; \underbrace{\text{promotion}}_{3\text{ bits (0=None, 1=N, 2=B, 3=R, 4=Q)}}$$

### 3.4 Tree Node Payload Encoding

Each position node encodes:
1. **`total_games`** (`varint`): Total database games reaching this board state.
2. **`white_wins`** (`varint`): Number of White wins ($1-0$).
3. **`black_wins`** (`varint`): Number of Black wins ($0-1$).
   *(Draws are implicitly derived as $\text{draws} = \text{total\_games} - (\text{white\_wins} + \text{black\_wins})$).*
4. **`move_count`** (`varint`): Number of distinct legal continuation moves played.
5. **For each continuation move**:
   - `packed_move` (`u16`, 2 bytes): 16-bit packed move.
   - `white_wins` (`varint`): Moves resulting in White win.
   - `draws` (`varint`): Moves resulting in Draw.
   - `black_wins` (`varint`): Moves resulting in Black win.
   - `avg_white_elo` (`u16`, 2 bytes): Average White player Elo rating for this move.
   - `avg_black_elo` (`u16`, 2 bytes): Average Black player Elo rating for this move.

---

## 4. Key Algorithms & Execution Pipeline

### 4.1 Zero-Copy Lookup Pipeline ($< 0.01\text{ ms}$)

```
   Target FEN ──► Zobrist64 Hash
                       │
                       ▼
       Binary Search on Mapped Directory Table (Sorted Array)
                       │
                       ▼
       [ Found SortedIndexEntry { hash, data_offset } ]
                       │
         ┌─────────────┴─────────────┐
         ▼                           ▼
 [.pos.idx Lookup]           [.tree.idx Lookup]
 If data_offset & 0x80000000: Decode Move Stats Payload
 ➔ Return inlined Game ID     ➔ Return Repertoire Table
 Else: Decode Delta-Varints
```

### 4.2 Dynamic Fallback Mechanism
If `.tree.idx` is absent or the user navigates beyond `max_ply_depth`:
- The engine seamlessly invokes `TreeIndex::calculate_tree_for_scid` or `TreeIndex::calculate_tree_for_pgn`.
- Decodes games dynamically using fast streaming bytecode decoders without crashing or failing.

---

## 5. CLI & JSON-RPC API Reference

### 5.1 CLI Commands

```bash
# Build Search Booster (.pos.idx)
scid-mgr build-pos-idx <DB_PATH> [--max-ply 24] [--min-games 1] [--threads 8]

# Build Opening Tree Index (.tree.idx)
scid-mgr build-tree <DB_PATH> [--max-ply 24] [--min-games 1] [--threads 8]

# Diagnostics & Health Scan
scid-mgr diag-pos-idx <DB_PATH>
scid-mgr diag-tree-idx <DB_PATH>

# Query Opening Tree directly
scid-mgr tree <DB_PATH> [FEN]
```

### 5.2 JSON-RPC Server Methods

| Method | Parameters | Description |
| :--- | :--- | :--- |
| `pos_index_status` | `{}` | Returns `.pos.idx` validity (`valid`, `outdated`, `missing`) and unique position count. |
| `tree_index_status` | `{}` | Returns `.tree.idx` validity (`valid`, `outdated`, `missing`) and unique position count. |
| `build_pos_index` | `{"max_ply": 24, "min_games": 1, "threads": 8}` | Asynchronously builds `.pos.idx` with streaming `build_pos_index_progress` events. |
| `build_tree` | `{"max_ply": 24, "min_games": 1, "threads": 8}` | Asynchronously builds `.tree.idx` with streaming `build_tree_progress` events. |
| `opening_tree` | `{"fen": "...", "use_search_results": false}` | Queries `.tree.idx` ($< 0.01\text{ ms}$) or dynamically computes tree on filtered subsets. |
| `search_position` | `{"fen": "...", "mode": "exact"}` | Queries `.pos.idx` for instant candidate game acceleration. |

---

## 6. Benchmarks & Size Optimization (1.51M Games `master.pgn`)

| Metric | `--min-games 1` (All) | `--min-games 2` ($\ge 2\times$) | `--min-games 3` ($\ge 3\times$) |
| :--- | :--- | :--- | :--- |
| **Search Booster (`.pos.idx` v5)** | **211.21 MB** (was 278.74 MB) | **118.42 MB** | **94.10 MB** |
| **Inlined Singletons** | **7,932,479 (82.3%)** | — | — |
| **Opening Tree (`.tree.idx`)** | **234.21 MB** | **54.68 MB** (-77%) | **33.71 MB** (-86%) |
| **Unique Positions** | 9,643,084 | 1,710,605 | 930,922 |
| **Lookup Latency** | $< 0.01\text{ ms}$ | $< 0.01\text{ ms}$ | $< 0.01\text{ ms}$ |
