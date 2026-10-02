# 🧩 Binary File Formats & Schemas Reference

This document provides a comprehensive technical reference for all binary file formats, companion index files, header layouts, record structures, bit encodings, and magic signatures used in **`scid-mgr`**.

---

## 📑 Summary of All File Formats

| Extension | Magic Signature | Header Size | Record / Entry Size | Purpose |
| :--- | :---: | :---: | :---: | :--- |
| **`.si5`** | SCID v5 header | 128 bytes | 48 bytes (Fixed) | SCID 5 Game Index |
| **`.si4`** | SCID v4 header | 128 bytes | 48 bytes (Fixed) | SCID 4 Game Index |
| **`.sn5` / `.sn4`** | Namebase chunk | Variable | Variable | Deduplicated Namebase (Players, Events, Sites, Rounds) |
| **`.sg5` / `.sg4`** | Game byte stream | None | Variable blob | Huffman-like byte-encoded game move streams & tags |
| **`.pgn.idx`** | `SCIDPGN1` | 64 bytes | 40 bytes (Fixed) | Zero-copy companion index for `.pgn` files |
| **`.boost.idx`** | `SCIDBST1` | 64 bytes | 8 bytes (Dir) + 2 bytes (Move) | **16-bit Search Booster** (Dynamic Search, Tree & Lines) |
| **`.pos.idx`** | `SCIDPOS1` | 64 bytes | Variable (Delta-Varint) | Inverted position index for fast candidate pre-filtering |
| **`.tree.idx`** | `SCIDTRE1` | 64 bytes | Variable (Varint stats) | Precomputed opening tree book & win-rate distributions |
| **`.hot.idx`** | `CHSHOTG1` | 64 bytes | 32B Node / 16B Edge | Common line continuations DAG graph |
| **`.feat.idx`** | `CHSFEAT1` | 64 bytes | 8 bytes (Fixed `u64`) | 47-feature endgame classification bitmasks |

---

## 1. ⚡ 16-Bit Search Booster Index (`.boost.idx`)

The Search Booster is a high-speed accelerator format inspired by flat uncompressed move stream architectures. It encodes entire chess databases into a continuous, 16-bit move stream allowing sub-nanosecond move replaying across millions of games without decompression overhead.

### File Layout

```
+-------------------------------------------------------------+
| 64-Byte BoostHeader (Magic "SCIDBST1", Game Count, Offsets) |
+-------------------------------------------------------------+
| Fixed Directory Table (8 bytes * game_count)                |
|  [Entry 0] [Entry 1] [Entry 2] ... [Entry N-1]             |
+-------------------------------------------------------------+
| Continuous 16-bit Move Payload Array (2 bytes * total_plies)|
|  [Move 0] [Move 1] [Move 2] ... [Move M-1]                 |
+-------------------------------------------------------------+
```

### 64-Byte Header (`BoostHeader`)

| Offset (Bytes) | Field Name | Type | Description |
| :---: | :--- | :---: | :--- |
| `0..8` | `magic` | `[u8; 8]` | Magic ASCII signature `b"SCIDBST1"` |
| `8..12` | `version` | `u32` (LE) | Format version (currently `1`) |
| `12..16` | `header_size` | `u32` (LE) | Fixed header size (`64`) |
| `16..20` | `db_game_count` | `u32` (LE) | Total number of games indexed |
| `20..24` | `flags` | `u32` (LE) | Reserved / format flags (`0`) |
| `24..32` | `total_plies` | `u64` (LE) | Total number of 16-bit moves encoded |
| `32..40` | `db_mtime_secs` | `u64` (LE) | Source database modification timestamp |
| `40..48` | `db_file_size` | `u64` (LE) | Source database file size in bytes |
| `48..56` | `directory_offset` | `u64` (LE) | Byte offset to Game Directory Table (`64`) |
| `56..64` | `payload_offset` | `u64` (LE) | Byte offset to 16-bit Move Payload Array |

### 8-Byte Game Directory Table (`BoostGameEntry`)

Enables $O(1)$ random indexing into the move stream for arbitrary game IDs:

| Offset (Bytes) | Field Name | Type | Description |
| :---: | :--- | :---: | :--- |
| `0..4` | `move_offset` | `u32` (LE) | Zero-based index into the 16-bit move payload array |
| `4..6` | `ply_count` | `u16` (LE) | Number of half-moves in this game |
| `6` | `result` | `u8` | Result code: `0`=*, `1`=1-0, `2`=0-1, `3`=1/2-1/2 |
| `7` | `flags` | `u8` | Bit 0: `is_deleted` (1 if deleted, 0 if active) |

### 16-Bit Move Encoding (`BoostMove`)

Moves are stored as compact 16-bit unsigned integers without variable-length delimiters:

```
 15          12 11            6 5             0
+--------------+---------------+---------------+
|  Flags (4b)  | From Sq (6b)  |  To Sq (6b)   |
+--------------+---------------+---------------+
```

- **Bits 0..=5 (6 bits)**: Destination square index (`0..=63`, where `a1=0`, `h8=63`).
- **Bits 6..=11 (6 bits)**: Origin square index (`0..=63`).
- **Bits 12..=15 (4 bits)**: Move category flags:
  - `0x0`: Quiet normal move (e.g. `e2-e3`, `Nf3`)
  - `0x1`: Double pawn push (e.g. `e2-e4`, `d7-d5`)
  - `0x2`: King-side castling (`O-O`, `to` normalized to `g1`/`g8`)
  - `0x3`: Queen-side castling (`O-O-O`, `to` normalized to `c1`/`c8`)
  - `0x4`: Standard piece capture
  - `0x5`: En passant capture
  - `0x8` / `0xC`: Knight promotion (quiet / capture)
  - `0x9` / `0xD`: Bishop promotion (quiet / capture)
  - `0xA` / `0xE`: Rook promotion (quiet / capture)
  - `0xB` / `0xF`: Queen promotion (quiet / capture)

---

## 2. 🗃️ PGN Companion Index (`.pgn.idx`)

A zero-copy companion index file built for raw text `.pgn` files to support instant opening, pagination, sorting, and header filtering without rescanning text.

### File Layout

```
+-------------------------------------------------------------+
| 64-Byte PgnIndexHeader (Magic "SCIDPGN1", Record Counts)    |
+-------------------------------------------------------------+
| Deduplicated Namebase Chunk (Null-delimited UTF-8 strings)   |
+-------------------------------------------------------------+
| Fixed-Length Compact Game Records (40 bytes * game_count)   |
+-------------------------------------------------------------+
```

### 40-Byte Compact Game Record (`CompactPgnRecord`)

| Offset (Bytes) | Field Name | Type | Description |
| :---: | :--- | :---: | :--- |
| `0..8` | `file_offset` | `u64` (LE) | Byte offset of the game start tag `[` in the `.pgn` file |
| `8..12` | `game_len` | `u32` (LE) | Total byte length of the game in the `.pgn` file |
| `12..16` | `white_id` | `u32` (LE) | String table ID for White player |
| `16..20` | `black_id` | `u32` (LE) | String table ID for Black player |
| `20..24` | `event_id` | `u32` (LE) | String table ID for Event |
| `24..28` | `site_id` | `u32` (LE) | String table ID for Site |
| `28..30` | `white_elo` | `u16` (LE) | White Elo rating (`0` if missing) |
| `30..32` | `black_elo` | `u16` (LE) | Black Elo rating (`0` if missing) |
| `32..35` | `date_packed` | `[u8; 3]` | Packed date representation (Year, Month, Day) |
| `35..38` | `eco_packed` | `[u8; 3]` | Packed ECO code (e.g. `B85`) |
| `38` | `result_code` | `u8` | `1`=1-0, `2`=0-1, `3`=1/2-1/2, `0`=* |
| `39` | `flags` | `u8` | Bit flags (e.g. custom FEN start) |

> [!NOTE]
> **Namebase Deduplication & Ultra-Large Collections**:
> The string table maps player, event, and site strings to `u32` IDs. When ingesting raw online exports (such as Lichess monthly dumps where `[Site "https://lichess.org/<game_id>"]` is unique per game), normalizing or stripping unique site IDs prior to indexing prevents allocating tens of millions of unique URL strings in the namebase dictionary.

---

## 3. 🎯 Inverted Position Index (`.pos.idx` / `.scidpos5`)

Inverted hash index providing sub-millisecond candidate game pre-filtering for board positions.

### File Layout

```
+-------------------------------------------------------------+
| 64-Byte PositionIndexHeader (Magic "SCIDPOS1")              |
+-------------------------------------------------------------+
| 256 Stripe Directory Table (Stripe Offset, Bucket Count)     |
+-------------------------------------------------------------+
| Hash Buckets & Delta-Varint Posting Lists                   |
|  [Zobrist Hash (8B)] [Game Count (Varint)] [Game ID Diffs]  |
+-------------------------------------------------------------+
```

### Delta-Varint Compression
- Game IDs within each posting list are sorted in ascending order: `[g0, g1, g2, g3]`.
- Encoded as variable-length delta differences: `[g0, g1 - g0, g2 - g1, g3 - g2]`.
- Achieves **~1.2 to 1.8 bytes per game reference**, saving over 70% RAM/disk space.

---

## 4. 🌳 Opening Tree Index (`.tree.idx`)

Precomputed opening tree index storing move statistics, win-rate distributions, and average Elo ratings for board positions.

### File Layout

```
+-------------------------------------------------------------+
| 64-Byte TreeIndexHeader (Magic "SCIDTRE1")                   |
+-------------------------------------------------------------+
| 256 Stripe Index Table (Byte Offsets & Counts per Stripe)   |
+-------------------------------------------------------------+
| Position Nodes with Variable-Length Binary Payloads:        |
|  - Total Games, White Wins, Black Wins (Varint)             |
|  - Move Count (Varint)                                      |
|  - For each move:                                           |
|      * Packed Move (2 bytes: From, To, Promo)               |
|      * Move White Wins, Draws, Black Wins (Varint)          |
|      * Avg White Elo, Avg Black Elo (2 bytes LE each)       |
+-------------------------------------------------------------+
```

---

## 5. 📈 Common Continuations Graph Index (`.hot.idx`)

Directed Acyclic Graph (DAG) index representing common line continuations and branching paths.

### File Layout

```
+-------------------------------------------------------------+
| 64-Byte HotGraphHeader (Magic "CHSHOTG1")                   |
+-------------------------------------------------------------+
| 256 Stripe Table                                            |
+-------------------------------------------------------------+
| Continuous Node Array (32 bytes per node):                  |
|  - Zobrist Hash (u64)                                       |
|  - Total Occurrences (u32)                                  |
|  - Edge Slice (Start Index: u32, Length: u32)               |
+-------------------------------------------------------------+
| Continuous Edge Array (16 bytes per edge):                  |
|  - Packed Move (u16)                                        |
|  - Target Node ID (u32)                                     |
|  - Edge Game Frequency (u32)                                |
+-------------------------------------------------------------+
```

---

## 6. ♟️ Endgame Taxonomy Index (`.feat.idx`)

Ultra-compact fixed-record index classifying games into **47 distinct endgame categories** based on piece counts and pawn structures.

### File Layout

```
+-------------------------------------------------------------+
| 64-Byte FeatureIndexHeader (Magic "CHSFEAT1")               |
+-------------------------------------------------------------+
| Fixed 8-Byte Bitmask Table (8 bytes * game_count):          |
|  [u64 Feature Bitmask 0] [u64 Feature Bitmask 1] ...        |
+-------------------------------------------------------------+
```

- **Bit `i` set to 1**: Indicates the game reached Endgame Feature `i` (e.g. `END_PAWN_KP_K`, `END_ROOK_R_R`, `END_BISHOP_OCB`).
- Evaluated with single SIMD bitwise instructions (`AND`, `POPCNT`) across millions of games in < 1 ms.

---

## 7. 💾 SCID Native Database Formats (`.si5` / `.si4`, `.sn5` / `.sn4`, `.sg5` / `.sg4`)

### SCID 48-Byte Index Record (`IndexEntry`)

| Offset (Bytes) | Field Name | Type | Description |
| :---: | :--- | :---: | :--- |
| `0..6` | `offset` | `u48` (LE) | Byte offset into the games file (`.sg5` / `.sg4`) |
| `6..10` | `length` | `u32` (LE) | Length of the game blob in bytes |
| `10..14` | `white_id` | `u32` (LE) | Player ID for White in namebase |
| `14..18` | `black_id` | `u32` (LE) | Player ID for Black in namebase |
| `18..22` | `event_id` | `u32` (LE) | Event ID in namebase |
| `22..26` | `site_id` | `u32` (LE) | Site ID in namebase |
| `26..30` | `round_id` | `u32` (LE) | Round ID in namebase |
| `30` | `result` | `u8` | Game result code |
| `31..33` | `eco_code` | `u16` (LE) | Encoded ECO code |
| `33..36` | `date` | `[u8; 3]` | Encoded year, month, day |
| `36..38` | `white_elo` | `u16` (LE) | White Elo rating |
| `38..40` | `black_elo` | `u16` (LE) | Black Elo rating |
| `40` | `flags` | `u8` | Bit flags (deleted status, non-standard start) |
| `41..48` | `_reserved` | `[u8; 7]` | Reserved / padding |
