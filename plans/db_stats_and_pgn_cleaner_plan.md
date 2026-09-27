# Database Statistics & PGN Sanitizer Utility Plan

## 1. Overview & Objective

This plan outlines the architecture, data models, server JSON-RPC commands, CLI integration, and test specifications for two complementary database maintenance features in `scid-mgr`:

1. **Extended Database Statistics & Header Distribution (`stats` / `db_stats`)**:
   - Operates on the **currently opened database** (both SCID `.si5` and indexed `.pgn`).
   - Computes deep database health diagnostics in parallel (< 10 ms across millions of games):
     - **Header Presence & Completeness**: Frequencies and percentages of standard and custom tags (`Date`, `WhiteElo`, `BlackElo`, `ECO`, `Event`, `Site`, `Round`, `FEN`, `TimeControl`, `Annotator`, etc.).
     - **Result Distribution**: Exact breakdown of `1-0`, `0-1`, `1/2-1/2`, and `*`.
     - **Rating / Elo Distributions**: Min, max, average, and rating brackets (`<1500`, `1500-2000`, `2000-2400`, `2400-2600`, `2600+`).
     - **Temporal / Date Distributions**: Earliest, latest, unknown dates, and decadal counts.
     - **Game Lengths**: Average, shortest, and longest game plies.

2. **PGN Sanitization & Cleaning Engine (`clean_pgn` / `clean`)**:
   - Operates on the **currently opened PGN database** in JSON-RPC server mode, or a specified PGN path via CLI.
   - Validates that the target database is a `.pgn` file (rejects `.si5` databases with a clear diagnostic message).
   - High-throughput streaming cleaner/stripper with selectable options:
     - `strip_eval`: Strip engine evaluation comments (`[%eval ...]`, `[%eval #-2]`).
     - `strip_clk`: Strip clock and elapsed move times (`[%clk ...]`, `[%emt ...]`).
     - `strip_comments`: Strip all inline textual annotations (`{ ... }`).
     - `strip_variations`: Strip all sub-variations (`( ... )`).
     - `strip_nags`: Strip Numeric Annotation Glyphs (`$1`, `$2`, `!?`, `??`).
     - `strip_empty_tags`: Remove tags with `"?"` or empty values.
   - Emits streaming progress events (`clean_pgn_progress`) for multi-gigabyte PGN processing.

---

## 2. Architecture & Module Structure

```
src/
├── db/
│   ├── stats.rs              # SCID & binary record statistics engine (Rayon parallel)
│   └── mod.rs
├── pgn_db/
│   ├── stats.rs              # Compact PGN record statistics engine
│   └── mod.rs
├── pgn/
│   ├── cleaner.rs            # Streaming zero-copy PGN token sanitizer
│   ├── mod.rs
│   └── scanner.rs
├── server/
│   ├── handlers/
│   │   ├── stats.rs          # handle_db_stats / extended handle_info_stats
│   │   ├── pgn_ops.rs        # handle_clean_pgn
│   │   └── mod.rs
│   └── mod.rs
└── cli/
    └── commands/
        ├── stats.rs          # `scid-mgr stats <db_path> [--detailed]`
        └── clean.rs          # `scid-mgr clean <pgn_path> [--output <dest>] [flags]`
```

---

## 3. Data Models & Statistics Schema

### 3.1. Database Statistics Model (`DatabaseStatistics`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStatistics {
    pub total_games: usize,
    pub format: String, // "si5" | "si4" | "pgn"
    pub results: ResultDistribution,
    pub headers: HeaderDistribution,
    pub ratings: RatingStatistics,
    pub dates: DateDistribution,
    pub moves: MoveLengthStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultDistribution {
    pub white_wins: usize,
    pub white_wins_pct: f64,
    pub black_wins: usize,
    pub black_wins_pct: f64,
    pub draws: usize,
    pub draws_pct: f64,
    pub ongoing_or_unknown: usize,
    pub ongoing_or_unknown_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDistribution {
    pub white_elo_count: usize,
    pub white_elo_pct: f64,
    pub black_elo_count: usize,
    pub black_elo_pct: f64,
    pub both_elo_count: usize,
    pub both_elo_pct: f64,
    pub eco_count: usize,
    pub eco_pct: f64,
    pub date_count: usize,
    pub date_pct: f64,
    pub event_count: usize,
    pub site_count: usize,
    pub round_count: usize,
    pub custom_fen_count: usize,
    pub extra_tags: HashMap<String, usize>, // Tag name -> occurrences
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingStatistics {
    pub min_elo: u16,
    pub max_elo: u16,
    pub avg_white_elo: f64,
    pub avg_black_elo: f64,
    pub bracket_under_1500: usize,
    pub bracket_1500_2000: usize,
    pub bracket_2000_2400: usize,
    pub bracket_2400_2600: usize,
    pub bracket_2600_plus: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateDistribution {
    pub earliest_date: Option<String>,
    pub latest_date: Option<String>,
    pub decade_counts: HashMap<String, usize>, // e.g. "1990s" -> 4500
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveLengthStatistics {
    pub min_plies: usize,
    pub max_plies: usize,
    pub avg_plies: f64,
}
```

---

## 4. PGN Sanitizer & Cleaner Engine

### 4.1. Cleaning Configuration

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PgnCleanOptions {
    pub strip_eval: bool,        // Strip [%eval ...]
    pub strip_clk: bool,         // Strip [%clk ...] and [%emt ...]
    pub strip_comments: bool,    // Strip { ... } entirely
    pub strip_variations: bool,  // Strip ( ... ) sub-variations
    pub strip_nags: bool,        // Strip $1, $2, etc.
    pub strip_empty_tags: bool,  // Strip [Tag "?"]
}
```

### 4.2. Streaming State Machine
The cleaner operates as a byte-level zero-allocation streaming filter:
1. **Tag Section**: Parses tags line by line. Drops empty tags if `strip_empty_tags` is active; preserves required standard headers.
2. **Move Section**:
   - Tracks nested comment depth `{ ... }` and nested variation depth `( ... )`.
   - If `strip_eval` or `strip_clk` is set but `strip_comments` is false, parses interior comment annotations using fast regex/byte-matching and strips only the matching `[%eval ...]` / `[%clk ...]`, leaving textual notes intact.
   - If `strip_comments` is true, skips all bytes inside `{}`.
   - If `strip_variations` is true, skips all bytes inside `()`.
   - If `strip_nags` is true, skips `$digit+` tokens.
   - Normalizes whitespace to prevent double spaces from removed tokens.

---

## 5. JSON-RPC Server Commands

### 5.1. `db_stats` / `stats`
- **Params**:
  - `detailed`: `boolean` (optional, default `true`)
- **Execution**:
  - Runs on the currently opened `DatabaseBackend` (`Scid` or `Pgn`).
  - Returns `DatabaseStatistics`.

### 5.2. `clean_pgn` / `clean`
- **Params**:
  - `output_path`: `string` (destination file path; if omitted or equal to source, writes to a temporary file and replaces atomically)
  - `options`: `PgnCleanOptions`
- **Validation**:
  - Checks if the currently opened database is a `DatabaseBackend::Pgn`.
  - If a `DatabaseBackend::Scid` is open, returns error: `"clean_pgn is only supported for PGN databases. SCID databases already store compact binary records."`
- **Progress Events**: Emits `{"event": "clean_pgn_progress", "data": {"scanned_bytes": N, "total_bytes": M, "percent": 54.2}}`.
- **Returns**:
  ```json
  {
    "status": "ok",
    "data": {
      "input_path": "sample.pgn",
      "output_path": "sample_cleaned.pgn",
      "original_size_bytes": 10485760,
      "cleaned_size_bytes": 4194304,
      "reduction_percent": 60.0,
      "games_cleaned": 50000,
      "duration_ms": 142
    }
  }
  ```

---

## 6. CLI Commands

### 6.1. `scid-mgr stats <db_path>`
Displays formatted terminal tables of:
- Result distribution & percentages.
- Header completeness checklist.
- Rating brackets & averages.
- Decade timeline.

### 6.2. `scid-mgr clean <pgn_path>`
- Checks file extension / header magic to verify it is a valid `.pgn` file.
- Flags:
  - `--output <path>` / `-o <path>`: Destination path.
  - `--strip-eval`: Remove `[%eval]` annotations.
  - `--strip-clk`: Remove `[%clk]` / `[%emt]` clock times.
  - `--strip-comments`: Remove all `{}` comments.
  - `--strip-variations`: Remove all `()` sub-lines.
  - `--strip-nags`: Remove `$N` glyphs.
  - `--all`: Enable all stripping flags.

---

## 7. Implementation Steps

1. **Step 1: Database Statistics Module**
   - Implement `ScidDatabaseWrapper::statistics()` and `PgnDatabaseWrapper::statistics()` using Rayon parallel iterators.
   - Aggregate Elo, results, dates, and header presence in $O(N)$ with minimal allocations.

2. **Step 2: PGN Cleaning & Sanitizing Engine**
   - Implement `src/pgn/cleaner.rs` with streaming state-machine byte filter.
   - Write unit tests verifying that move numbers, SAN moves, and essential tags remain 100% valid after stripping.

3. **Step 3: Server Handlers & Routing**
   - Wire `db_stats` / `stats` in `src/server/handlers/stats.rs`.
   - Wire `clean_pgn` in `src/server/handlers/pgn_ops.rs` with backend validation and progress reporting.
   - Connect handlers in `src/server/mod.rs`.

4. **Step 4: CLI Integration**
   - Add `stats` and `clean` subcommands to `src/cli/`.
   - Provide clear error reporting when `clean` is invoked on non-PGN files.

5. **Step 5: Testing & Verification**
   - Integration tests in `tests/test_stats_and_cleaner.rs`.
   - Verify size reduction and syntax validity on sample PGNs.
   - Update `docs/API_REFERENCE.md`.
