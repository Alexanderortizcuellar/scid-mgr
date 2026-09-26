# ♟️ Endgame Taxonomy & Feature Index (`.feat.idx`) Specification

The **Endgame Engine** classifies games into standardized endgame categories and calculates endgame popularity distributions across chess databases:
1. **Catalog-Driven Taxonomy**: 47 standardized endgame definitions based on international classification (GBR codes, FCE, Dvoretsky, de la Villa) embedded directly into the binary from YAML.
2. **Ultra-Compact Binary Feature Index (`.feat.idx`)**: 8 bytes per game (`u64` endgame bitmask). A 1,000,000-game database requires only **~8 MB** of index storage.
3. **Endgame Popularity Analytics**: Computes occurrence counts, percentage shares, win/draw/loss distributions, and category breakdowns (Pawn, Rook, Bishop, Knight, Mixed Minor, Queen, Rook vs Minor, Queen vs Pieces) for the entire database or games reaching a specific opening/middlegame position.
4. **Instant Bitwise Feature Filtering**: $O(1)$ bitwise lookup to find all games reaching specific endgame types (e.g., Rook + Pawn vs Rook, Opposite-Colored Bishops, King & Pawn vs King).

---

## 💾 Binary Format Specification: `.feat.idx`

The `.feat.idx` file consists of a fixed **64-byte Header** followed by a contiguous array of **8-byte `GameFeatureRecord` entries** (one entry per game in the database).

```
+-------------------------------------------------------------+
| Header (64 Bytes)                                           |
| - magic: b"CHSFEAT1" (8B)                                   |
| - version: u32 (4B)                                         |
| - catalog_version: u32 (4B)                                 |
| - game_count: u32 (4B)                                      |
| - endgame_bit_count: u16 (2B)                               |
| - record_size: u16 (2B, = 8)                                |
| - db_mtime_secs: u64 (8B)                                   |
| - db_file_size: u64 (8B)                                    |
| - created_timestamp: u64 (8B)                               |
| - _reserved: [u8; 16] (16B)                                 |
+-------------------------------------------------------------+
| GameFeatureRecord 0 (8 Bytes: u64 endgame_bits)             |
+-------------------------------------------------------------+
| GameFeatureRecord 1 (8 Bytes: u64 endgame_bits)             |
+-------------------------------------------------------------+
| ...                                                         |
+-------------------------------------------------------------+
| GameFeatureRecord [game_count - 1] (8 Bytes)                |
+-------------------------------------------------------------+
```

### 1. Header Layout (64 Bytes)

| Offset | Field | Type | Description |
| :--- | :--- | :--- | :--- |
| `0x00..0x08` | `magic` | `[u8; 8]` | Magic identifier: `b"CHSFEAT1"` (`0x43, 0x48, 0x53, 0x46, 0x45, 0x41, 0x54, 0x31`) |
| `0x08..0x0C` | `version` | `u32` | Binary format version (currently `1`) |
| `0x0C..0x10` | `catalog_version` | `u32` | Taxonomy catalog version (currently `2`) |
| `0x10..0x14` | `game_count` | `u32` | Total number of games indexed |
| `0x14..0x16` | `endgame_bit_count`| `u16` | Number of active endgame bits (currently `47`) |
| `0x16..0x18` | `record_size` | `u16` | Size of each record in bytes (`8`) |
| `0x18..0x20` | `db_mtime_secs` | `u64` | UNIX timestamp (seconds) of source database modification |
| `0x20..0x28` | `db_file_size` | `u64` | Size in bytes of source database file |
| `0x28..0x30` | `created_timestamp`| `u64` | UNIX timestamp (seconds) when index was generated |
| `0x30..0x40` | `_reserved` | `[u8; 16]` | Reserved for alignment and future expansion |

### 2. Record Layout (8 Bytes per Game)

Each record is a single little-endian `u64` bitmask where bit $k \in [0, 63]$ is set to `1` if endgame feature $k$ occurred at any ply in the game:

```rust
#[repr(C)]
pub struct GameFeatureRecord {
    pub endgame_bits: u64,
}
```

---

## 🏛️ Endgame Taxonomy & Feature Mapping (47 Standard Features)

The catalog classifies endgames across 8 major categories:

| Bit | Feature ID | Name | GBR Code | Category |
| :--- | :--- | :--- | :--- | :--- |
| **0** | `END_PAWN_KP_K` | King and Pawn vs King | `0000.10` | `PAWN` |
| **1** | `END_PAWN_KP_KP` | King and Pawn vs King and Pawn | `0000.11` | `PAWN` |
| **2** | `END_PAWN_K2P_K` | King and 2 Pawns vs King | `0000.20` | `PAWN` |
| **3** | `END_PAWN_K2P_KP` | King and 2 Pawns vs King and Pawn | `0000.21` | `PAWN` |
| **4** | `END_PAWN_MULTI` | Multi-Pawn Endgame | `0000` | `PAWN` |
| **5** | `END_ROOK_R_R` | Bare Rooks | `0400.00` | `ROOK` |
| **6** | `END_ROOK_RP_R` | Rook and Pawn vs Rook (Lucena / Philidor) | `0400.10` | `ROOK` |
| **7** | `END_ROOK_RP_RP` | Rook and Pawn vs Rook and Pawn | `0400.11` | `ROOK` |
| **8** | `END_ROOK_R2P_R` | Rook and 2 Pawns vs Rook | `0400.20` | `ROOK` |
| **9** | `END_ROOK_R2P_RP` | Rook and 2 Pawns vs Rook and Pawn | `0400.21` | `ROOK` |
| **10** | `END_ROOK_R3P_R2P` | Rook and 3 Pawns vs Rook and 2 Pawns | `0400.32` | `ROOK` |
| **11** | `END_ROOK_R4P_R3P` | Rook and 4 Pawns vs Rook and 3 Pawns | `0400.43` | `ROOK` |
| **12** | `END_ROOK_R_NP_R` | Rook and Pawns vs Lone Rook | `0400` | `ROOK` |
| **13** | `END_ROOK_R_R_GENERAL`| General Single Rook Endgame | `0400` | `ROOK` |
| **14** | `END_ROOK_2R_2R` | Double Rook Endgame | `0800.00` | `ROOK` |
| **15** | `END_BISHOP_BP_B` | Bishop and Pawn vs Bishop | `0010.10` | `BISHOP` |
| **16** | `END_BISHOP_SCB` | Same-Colored Bishops | `0020` | `BISHOP` |
| **17** | `END_BISHOP_OCB` | Opposite-Colored Bishops | `0020` | `BISHOP` |
| **18** | `END_BISHOP_OCB_1P`| Opposite-Colored Bishops with 1-Pawn Advantage | `0020` | `BISHOP` |
| **19** | `END_BISHOP_OCB_2P`| Opposite-Colored Bishops with 2-Pawn Advantage | `0020` | `BISHOP` |
| **20** | `END_BISHOP_2B_K` | Two Bishops vs Bare King | `0020.00` | `BISHOP` |
| **21** | `END_KNIGHT_N_N` | Bare Knights | `0001.00` | `KNIGHT` |
| **22** | `END_KNIGHT_NP_N` | Knight and Pawn vs Knight | `0001.10` | `KNIGHT` |
| **23** | `END_KNIGHT_N2P_N` | Knight and 2 Pawns vs Knight | `0001.20` | `KNIGHT` |
| **24** | `END_KNIGHT_BN_K` | Bishop and Knight vs Bare King | `0011.00` | `KNIGHT` |
| **25** | `END_MIXED_B_N` | Bishop vs Knight | `0011` | `MINOR_MIXED` |
| **26** | `END_MIXED_BP_N` | Bishop and Pawn vs Knight | `0011.10` | `MINOR_MIXED` |
| **27** | `END_MIXED_NP_B` | Knight and Pawn vs Bishop | `0011.01` | `MINOR_MIXED` |
| **28** | `END_MIXED_2B_2N` | Two Bishops vs Two Knights | `0022` | `MINOR_MIXED` |
| **29** | `END_QUEEN_Q_Q` | Bare Queens | `1000.00` | `QUEEN` |
| **30** | `END_QUEEN_QP_Q` | Queen and Pawn vs Queen | `1000.10` | `QUEEN` |
| **31** | `END_QUEEN_QP_QP` | Queen and Pawn vs Queen and Pawn | `1000.11` | `QUEEN` |
| **32** | `END_QUEEN_Q2P_Q` | Queen and 2 Pawns vs Queen | `1000.20` | `QUEEN` |
| **33** | `END_QUEEN_Q_P` | Queen vs Advanced Pawn | `1000.01` | `QUEEN` |
| **34** | `END_QUEEN_2Q_2Q` | Double Queen Endgame | `2000.00` | `QUEEN` |
| **35** | `END_ROOK_VS_BISHOP`| Rook vs Bishop | `0410` | `ROOK_VS_MINOR` |
| **36** | `END_ROOK_VS_KNIGHT`| Rook vs Knight | `0401` | `ROOK_VS_MINOR` |
| **37** | `END_ROOK_VS_2MINORS`| Rook vs 2 Minor Pieces | `0420` / `0411` | `ROOK_VS_MINOR` |
| **38** | `END_ROOK_MINOR_RB_R`| Rook and Bishop vs Rook | `0410.00` | `ROOK_VS_MINOR` |
| **39** | `END_ROOK_MINOR_RN_R`| Rook and Knight vs Rook | `0401.00` | `ROOK_VS_MINOR` |
| **40** | `END_QUEEN_VS_ROOK` | Queen vs Rook with Optional Pawns | `1400` | `ROOK_VS_MINOR` |
| **41** | `END_QUEEN_VS_R_NO_P`| Queen vs Lone Rook | `1400.00` | `ROOK_VS_MINOR` |
| **42** | `END_QUEEN_VS_2ROOKS`| Queen vs 2 Rooks | `1800` | `QUEEN_VS_PIECES` |
| **43** | `END_QUEEN_VS_ROOK_MINOR`| Queen vs Rook and Minor Piece | `1410` / `1401` | `QUEEN_VS_PIECES` |
| **44** | `END_QUEEN_VS_2MINORS`| Queen vs 2 Minor Pieces | `1020` / `1011` | `QUEEN_VS_PIECES` |
| **45** | `END_QUEEN_VS_3MINORS`| Queen vs 3 Minor Pieces | `1030` / `1021` | `QUEEN_VS_PIECES` |
| **46** | `END_QUEEN_ROOK_VS_QUEEN_ROOK`| Queen and Rook vs Queen and Rook | `4400` | `QUEEN_VS_PIECES` |

---

## ⚡ Performance Characteristics

- **Sub-Millisecond Aggregations**: The 8-byte record representation allows evaluating entire databases using vectorised bitwise `popcnt` operations in under 5 ms for 1,000,000 games.
- **Compact Storage Footprint**:
  - 100,000 games: **800 KB**
  - 1,000,000 games: **8.0 MB**
  - 10,000,000 games: **80.0 MB**
- **Zero Allocation Memory Mapping**: `MmapFeatureIndex` maps directly from the OS page cache with zero deserialization overhead.

---

## 🖥️ CLI & REPL Commands

```bash
# Query endgame popularity report for an entire database
scid-mgr endgames database.si5

# Query endgames reaching a specific position (e.g. Sicilian Defense)
scid-mgr endgames database.si5 --fen "r1bqkbnr/pp1ppppp/2n5/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3"

# Filter by endgame category
scid-mgr endgames database.si5 --category ROOK

# Filter by specific feature ID with sample games
scid-mgr endgames database.si5 --feature END_ROOK_RP_R --samples 10

# Build companion .feat.idx index in parallel across all CPU cores
scid-mgr build endgames database.si5

# REPL Command
scid-mgr> .endgames [FEN]
```

---

## 🔌 JSON-RPC API Endpoints

### 1. `endgames`
Calculates endgame popularity distribution and statistics.
```json
{
  "id": 1,
  "command": "endgames",
  "params": {
    "fen": "r1bqkbnr/pp1ppppp/2n5/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3",
    "category": "ROOK",
    "feature_id": "END_ROOK_RP_R",
    "max_samples": 20
  }
}
```

### 2. `build_endgames`
Builds or rebuilds the `.feat.idx` index in parallel across worker threads. Emits `build_endgames_progress` events.
```json
{
  "id": 2,
  "command": "build_endgames",
  "params": {
    "output_path": "path/to/custom.feat.idx"
  }
}
```
