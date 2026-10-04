# ⚡ Performance & Engineering Optimizations

This document details the architectural techniques and algorithmic optimizations that enable `scid-mgr` to sort, filter, and analyze databases with **over 10 million games** in sub-second times.

---

## 1. Alphabetical Name Rank Arrays (Solving the 10s Sorting Bottleneck)

### The Problem
In standard database sorting, comparing games by White Player requires comparing player strings:
$$\text{Comparisons} = N \log_2 N \approx 10{,}355{,}488 \times 23.3 \approx 241{,}000{,}000 \text{ comparisons}$$
Performing 241 million heap allocations, slice lookups, and UTF-8 string comparisons on a single thread took **> 10.5 seconds**.

### The Solution
SCID databases separate metadata into an **Index File** (`.si5`) and a **Namebase** (`.sn5`). The index only stores a 32-bit `player_id`.

```rust
// 1. Sort unique names ONCE (544,871 players takes ~114 ms):
let mut ranks = vec![0u32; names.len()];
let mut ids: Vec<u32> = (0..names.len() as u32).collect();
ids.par_sort_unstable_by(|&a, &b| names[a as usize].cmp(&names[b as usize]));
for (rank, &id) in ids.iter().enumerate() {
    ranks[id as usize] = rank as u32;
}

// 2. Main sort across 10.35M games:
matched_indices.par_sort_unstable_by(|&a, &b| {
    let ra = ranks[entries[a].white_id as usize];
    let rb = ranks[entries[b].white_id as usize];
    ra.cmp(&rb)
});
```

### Result
- String comparison replaced by a single **32-bit integer register comparison**.
- Combined with Rayon multi-threading, sorting dropped from **10.5 seconds down to ~2.0 seconds** (a **5.2x speedup** on 10.35M games, and **< 40 ms** on 280k games).

---

## 2. Hardware Bitboard Material & Opposite-Colored Bishop Search

### Fast Bitboard Signatures
Each index entry contains compressed bitboard material counts. For specialized endgame searches (e.g. Opposite-Colored Bishops):

- **Light Squares Mask**: `0x55AA55AA55AA55AA`
- **Dark Squares Mask**: `0xAA55AA55AA55AA55`

```rust
let white_bishops_light = (white_bishops & Bitboard::LIGHT_SQUARES).count_ones();
let white_bishops_dark  = (white_bishops & Bitboard::DARK_SQUARES).count_ones();
let black_bishops_light = (black_bishops & Bitboard::LIGHT_SQUARES).count_ones();
let black_bishops_dark  = (black_bishops & Bitboard::DARK_SQUARES).count_ones();

if opposite_bishops {
    let w_on_light_b_on_dark = white_bishops_light == 1 && black_bishops_dark == 1 && white_bishops_dark == 0 && black_bishops_light == 0;
    let w_on_dark_b_on_light = white_bishops_dark == 1 && black_bishops_light == 1 && white_bishops_light == 0 && black_bishops_dark == 0;
    if !w_on_light_b_on_dark && !w_on_dark_b_on_light {
        return false;
    }
}
```
Evaluating bitwise operations via hardware CPU instructions (`POPCNT`) searches **10.35 million games in 2.4 seconds** (~4.2 million games/sec).

---

## 3. Zero-Copy Memory-Mapped I/O (`memmap2`)

Instead of issuing thousands of `seek()` and `read()` OS system calls:
- The `.sg5` / `.sg4` move-stream files are mapped directly into the virtual address space using `memmap2::Mmap`.
- Slicing `&games_mmap[offset .. offset + length]` avoids intermediate buffer copies and kernel-to-user space context switches.
- OS page caching automatically keeps hot games in CPU L3/L2 cache.

---

## 4. 1-Pass Parallel PGN Ingestion with Binary Companion Caching

For plain text `.pgn` files:
1. **Parallel Rayon Chunk Scanner**:
   - Splits multi-gigabyte PGNs into parallel chunk ranges across CPU threads.
   - Scans 9 standardized header tags and records byte offsets.
2. **Companion Cache (`<file>.pgn.idx`)**:
   - Serializes index structs to disk using `bincode`.
   - Subsequent opens of a 280,000-game PGN take **< 10 ms** instead of re-parsing.

---

## 5. UI Debouncing & Windowed Virtual Table

In PyQt5:
- Rendering 10 million rows directly into a widget causes catastrophic GUI freezing.
- `VirtualScidTableModel` only requests 50–100 visible rows at a time.
- A **150 ms single-shot debounce timer** ensures requests are only sent when fast scrolling pauses, maintaining constant **60 FPS** UI responsiveness.

---

## 6. Static Disk-Backed Position Index (`.pos.idx` v3) & Instant Opening Explorer

### Architecture & Format
For sub-millisecond position lookups and live ChessBase/Lichess opening trees without memory overhead:
- **Binary Header (64 bytes, `SCIDPOS5` v3)**: Stores `db_mtime_secs`, `db_size_bytes`, `db_game_count`, `max_ply_depth`, `unique_positions`, `index_offset`, and `data_offset`.
- **Sorted Hash Table (12 bytes per unique position)**:
  - `hash: u64` (64-bit Zobrist Hash)
  - `data_offset: u32` (relative offset into data payload)
  - Records are sorted strictly by Zobrist hash for **in-place binary search** (`binary_search_by_key`).
- **Compact Delta-Varint Posting Lists & Singleton Encoding**:
  - Moves with 1 game use a compact singleton outcome byte + varint ID.
  - Multi-game moves encode outcome sums, `id_count`, and sorted monotonically ascending delta-varints (`write_delta_game_ids`).
- **Zero-Allocation Fast Skipping (`skip_varints`)**:
  - When querying the opening explorer without explicit filter lists, `decode_position_payload` decodes only the requested sample game IDs (default 20) and rapidly skips remaining varints in the memory map without heap allocation.
- **Zero Heap RAM Overhead (`memmap2`)**:
  - The index is never deserialized into heap memory or HashMaps.
  - Lookups execute directly on the memory-mapped virtual address space, keeping heap memory at **0 MB** extra.
- **Validation**: On open, compares header magic, version, timestamp, and game count in $< 0.01\text{ ms}$. Returns `Valid`, `Outdated`, or `Missing`.
- **Lookup Speed**:
  - `query_tree(fen)`: **< 0.05 ms** average response with win/draw/loss counts, score percentages, and move statistics.
  - `search_position(fen)`: **< 0.01 ms** instant retrieval of matching game IDs, falling back seamlessly to multi-threaded move-stream scanning if the index is not present.

---

## 7. SCID Binary Tag Parser & Resilient Move-Stream Decoder

### Specification-Compliant Extra Tag Skipping
SCID games store optional extra tags (such as `WhiteFideId`, `BlackFideId`, `EventDate`, `Annotator`, etc.) at the start of each `.sg5`/`.sg4` game blob before the flags and move bytes:
- **`0x00`**: Marks the end of the extra tags section.
- **`0xff` (255)**: Legacy 3-byte packed EventDate without length byte.
- **`0x01..=0xf0` (1..240)**: Custom tag where the byte value is the ASCII tag name length, followed by 1-byte value length and value bytes.
- **`0xf1..=0xfe` (241..254)**: Standard pre-defined 1-byte tag code, followed directly by 1-byte value length and value bytes (no tag name).

```rust
#[inline]
pub fn skip_extra_tags(blob: &[u8], cursor: &mut usize) -> bool {
    while *cursor < blob.len() {
        let name_code = blob[*cursor];
        *cursor += 1;
        if name_code == 0 {
            return true;
        }
        if name_code == 255 {
            *cursor += 3;
            continue;
        }
        if name_code <= 240 {
            *cursor += name_code as usize;
        }
        if *cursor >= blob.len() {
            return false;
        }
        let value_len = blob[*cursor] as usize;
        *cursor += 1 + value_len;
    }
    false
}
```

### Resilient Move Stepping
When scanning across massive 10M+ game databases with multi-threading, custom variants or corrupted move streams are safeguarded with fast bitboard move validation (`pos.is_legal(&mv)`) before in-place application:
- Prevents panics from illegal King moves or variant end conditions.
- Seamlessly scans across **10.35 million games in ~14 seconds** without building an index, or in **< 1 ms** with the static disk-backed `.pos.idx`.

---

## 8. Real-Time Streaming Search Progress System

For large multi-gigabyte collections (e.g. 10.35M games in `LumbrasGigaBase_OTB.si5`), full parallel linear scans take a few seconds. To ensure a smooth, non-blocking user experience in both GUI and headless pipelines:

### 1. Zero-Contention Chunked Search & Progress Callbacks
- Rayon parallel iterators use `entries.par_chunks(chunk_size)` (scaled to ~1% increments or 50,000 games).
- Thread-safe `AtomicUsize` counters track games scanned and matches found without heap allocation or mutex contention on the inner loop.
- The search engine fires streaming events per chunk:
  ```json
  {"event": "search_progress", "data": {"scanned": 5281254, "total": 10355488, "matches": 165936, "percent": 51.0}}
  ```

### 2. Live GUI Search Progress Dialog (`SearchProgressDialog`)
- Shows animated progress bar (0% - 100%).
- Displays live games scanned count (`scanned / total (percent%)`).
- Real-time matches counter (`🎯 Matches found: X`).
- Live scanning speed estimation (e.g. `⚡ Scanning Speed: ~750,000 games/sec`).
- Seamlessly auto-closes once the query results arrive and populate the table view.

---

## 9. Inverted Position Index Candidate Acceleration

### Candidate Filtering Pipeline
When a query contains an exact board position (`filter.fen`), `scid-mgr` uses `.pos.idx` as an inverted posting list accelerator:

```text
GUI search parameters (FEN + Header criteria)
        │
        ▼
Position Zobrist hash
        │
        ▼
.pos.idx binary search (< 0.05 ms)
        │
        ├── Position found in index:
        │       │
        │       ▼
        │   Candidate Game IDs (e.g. 40,852 games)
        │       │
        │       ▼
        │   Parallel metadata filter on candidates (.si5 / .pgn headers)
        │       │
        │       ▼
        │   Final Matching Game IDs (< 2 ms)
        │
        └── Position not found or non-exact (e.g. piece placement):
                │
                ▼
            Full database move-stream scan fallback (~1,450 ms)
```

### Comparative Benchmarks (1.49M Games in `twchess/data/database.si5`)
| Query / Position Scenario | Approach A (Full DB Scan) | Approach B (Candidate-Accelerated) | Speedup Factor | Combined FEN + Header Filter |
|:---|:---:|:---:|:---:|:---:|
| **Sicilian Najdorf** (Tabiya, 40,852 games) | 1,455.45 ms | **1.30 ms** | **1,115.3x faster** | **2.59 ms** |
| **French Defense** (1.e4 e6 2.d4 d5, 68,818 games) | 1,312.20 ms | **2.00 ms** | **656.6x faster** | **3.75 ms** |
| **1.e4** (Large Candidate Set, 708,913 games / ~45% of DB) | 748.14 ms | **17.38 ms** | **43.0x faster** | **26.63 ms** |

Correctness is strictly maintained: all candidate-accelerated queries match the full-scan result sets 100%, and unindexed positions gracefully fall back to move-stream parsing.

---

## 10. Monotonic Piece-Count Pruning (Search Booster)

### The Mathematical Invariant
In standard chess, pieces are captured and never re-enter the board. 
- **Standard Captures**: Decreases victim's piece count by 1 ($N \to N-1$).
- **En Passant**: Decreases victim's pawn count by 1 ($N \to N-1$).
- **Quiet Promotions (e.g. `e8=Q`)**: Replaces 1 pawn with 1 piece of the same color ($N \to N$, net change 0).
- **Capture Promotions (e.g. `exd8=Q`)**: Removes opponent piece and promotes pawn ($N \to N-1$).

Therefore, for any chess game, total piece count per side is strictly monotonically non-increasing:
$$\text{WhitePieces}(t) \ge \text{WhitePieces}(t+1) \quad \text{and} \quad \text{BlackPieces}(t) \ge \text{BlackPieces}(t+1)$$

### Early Loop Termination
When evaluating any target position with $T_{\text{white}}$ and $T_{\text{black}}$ pieces:
```rust
replay.apply_move(m);
if replay.white_pieces < target_w || replay.black_pieces < target_b {
    break; // Impossible to ever reach target position in any subsequent ply
}
```

### Benchmark Impact
- **LumbrasGigaBase (10.35M games)**: Scans for opening positions (e.g., Alapin Sicilian `2. c3 Nf6`) dropped from **1,043 ms down to 371 ms** (a **~2.8x speedup**), while preserving 100% exact matching accuracy (56,458 matching games).

---

## 11. Ultra-Large PGN Ingestion & Preprocessing Guide (20M+ Games / 40+ GB)

### Architecture & Format Capacity
The `.pgn.idx` format uses 64-bit offsets and 40-byte compact records:
- **Max File Size**: Up to 16 Exabytes (`u64` file offset).
- **Max Game Text Size**: Up to 4 GB (`u32` length).
- **Index Footprint**: $20{,}000{,}000 \text{ games} \times 40\text{ bytes} = \mathbf{800\text{ MB}}$ memory-mapped array.

### Raw PGN Pitfalls & Best Practices

#### 1. The Unique `[Site ...]` URL Bottleneck
- **Problem**: Lichess monthly dumps embed a unique game URL in every game, e.g. `[Site "https://lichess.org/abcd1234"]`.
- **Impact**: In a 20-million-game database, deduplicating the string pool creates **20 million heap-allocated strings** in RAM (~1.5 GB), causing heavy garbage collection and slow startup deserialization in `PgnNameTables`.
- **Solution / Preprocessing**: Normalize `[Site ...]` to `[Site "lichess.org"]` or strip unique game IDs prior to indexing. This collapses 20,000,000 unique strings down to **1 single string**.

#### 2. Clock & Evaluation Comment Overhead
- **Problem**: Raw Lichess PGNs contain move comments after every half-move (`[%clk 0:03:00]`, `[%eval 0.12]`).
- **Impact**: Inflates a 20-million-game PGN from **~12–14 GB up to 44 GB** (>65% of the file is comment text).
- **Solution / Preprocessing**: Strip `{ [%clk ...] }` and `{ [%eval ...] }` blocks. This shrinks disk I/O and speeds up move scanning and booster building by **3x to 4x**.

---

## 12. Robust Game Boundary Resolution & Header-Driven Indexing

### The Non-Standard PGN Boundary Problem
Standard PGN specifies the Seven Tag Roster (STR) beginning with `[Event ...]`. However, real-world databases (such as `master.pgn` with 1.54M games) frequently contain games missing `[Event ...]` and beginning directly with `[Site ...]`, `[Date ...]`, or `[White ...]`.

Naive byte searching for `[Event ` caused game index drift (1-game discrepancy), where secondary index builders merged games missing `[Event ` into adjacent games.

### The Solution: Tag-State-Machine & `.pgn.idx` Header Offset Ingestion
1. **Tag-State-Machine**: Recognizes any bracketed tag at column 0 (`is_pgn_tag_line`), consuming tag blocks and then move text until the next tag block.
2. **Direct Offset Reuse**: Secondary index builders (`.boost.idx`, `.feat.idx`) query `PgnDatabaseWrapper::open()`, directly reusing the pre-indexed `(entry.offset, entry.length)` pairs from `.pgn.idx` instead of rescanning the raw PGN text.

### Benchmark Impact
- **`master.pgn` (1.35 GB / 1,538,320 games)**: Boundary discovery dropped from **4,391.15 ms down to 89.58 ms** (**49.0x speedup**), completely eliminating the 4.4s disk scan before parallel move encoding and guaranteeing 100% database synchronization.
