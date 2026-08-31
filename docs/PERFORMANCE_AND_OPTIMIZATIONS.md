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

## 6. Static Disk-Backed Position Index (`.pos.idx` v2) & Instant Opening Explorer

### Architecture & Format
For sub-millisecond position lookups and live ChessBase/Lichess opening trees without memory overhead:
- **Binary Header (64 bytes, `SCIDPOS2`)**: Stores `db_mtime_secs`, `db_game_count`, `max_ply_depth`, and byte offsets.
- **Sorted Hash Table (16 bytes per unique position)**:
  - `hash: u64` (64-bit Zobrist Hash)
  - `data_offset: u32` (relative offset into data payload)
  - `data_len: u32` (byte length of record payload)
  - Records are sorted strictly by Zobrist hash for **in-place binary search** (`binary_search_by_key`).
- **Variable-Length Data Payload Section**:
  - Encodes total game counts, win/draw/loss counters, sample game ID posting lists, and branching move statistics (UCI, SAN, ELO sums, scores).
- **Zero Heap RAM Overhead (`memmap2`)**:
  - The index is never deserialized into heap memory or HashMaps.
  - Lookups execute directly on the memory-mapped virtual address space, keeping heap memory at **0 MB** extra.
- **Validation**: On open, compares timestamp and game count in $< 0.01\text{ ms}$. Returns `Valid`, `Outdated`, or `Missing`.
- **Lookup Speed**:
  - `query_tree(fen)`: **0.34 ms (348 µs)** average response with win/draw/loss counts, scores, and average ELO ratings.
  - `search_position(fen)`: **< 0.1 ms** instant retrieval of matching game IDs, falling back seamlessly to multi-threaded move-stream scanning if the index is not present.

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

