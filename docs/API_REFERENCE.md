# 🔌 JSON-RPC Server API Reference

The `scid-mgr` server communicates over standard input and standard output (`stdin` / `stdout`) using JSON Lines.

---

## 1. Request & Response Format

### Request Message
```json
{
  "id": 1,
  "command": "query_games",
  "params": {
    "page": 0,
    "page_size": 50,
    "player": "Kasparov",
    "sort_by": "date",
    "sort_asc": false
  }
}
```

### Response Message
```json
{
  "id": 1,
  "status": "ok",
  "data": {
    "page": 0,
    "page_size": 50,
    "total": 5309,
    "games": [ ... ]
  },
  "error": null
}
```

---

## 2. Server Commands

### `open`
Opens a database on disk (`.si5`, `.si4`, or `.pgn`).
- **Params**:
  - `path`: `string` (absolute or relative path)
- **Returns**: Database metadata (`stats`).

### `query_games` (or `get_games`)
Queries, filters, sorts, and paginates games.
- **Params**:
  - `page`: `number` (0-indexed, default: 0)
  - `page_size`: `number` (default: 100)
  - `player`: `string` (matches White or Black)
  - `white`: `string`
  - `black`: `string`
  - `result`: `string` (`"1-0"`, `"0-1"`, `"1/2-1/2"`)
  - `eco`: `string` (e.g. `"B90"`)
  - `date`: `string` (e.g. `"1999"`)
  - `event`: `string`
  - `site`: `string`
  - `sort_by`: `string` (`"date"`, `"white"`, `"black"`, `"white_elo"`, `"black_elo"`, `"eco"`, `"result"`, `"event"`, `"site"`, `"id"`)
  - `sort_asc`: `boolean` (default: `true`)
  - `fen`: `string` (exact or partial board placement)
  - `material`: `object` (`MaterialFilter`)
- **Returns**: `{ page, page_size, total, games: [...] }`

### `get_pgn`
Retrieves the standard PGN text for a specific game.
- **Params**:
  - `index`: `number` (0-indexed game ID)
- **Returns**: `{ index, pgn }`

### `search_position`
Performs Zobrist-hashed binary position search. If `.pos.idx` is valid and loaded, returns in sub-millisecond instant lookup time (< 0.1 ms); otherwise falls back seamlessly to multi-threaded move-stream scanning.
- **Params**:
  - `fen`: `string` (full FEN)
  - `max_ply`: `number` (optional maximum search depth)
- **Returns**: `{ matches: [{ game_id, ply }], total_games_searched, elapsed_ms }`

### `opening_tree` (or `query_tree`)
Queries the instant Opening Tree / Explorer for any board position (FEN or starting board).
- **Params**:
  - `fen`: `string` (optional FEN position; defaults to starting board)
  - `max_sample_games`: `number` (optional, default: `20`; limits sample game IDs returned per move; use `0` for pure stats)
  - `include_all_game_ids`: `boolean` (optional, default: `false`; decodes and returns the full posting list of game IDs for all moves)
  - `use_search_results`: `boolean` (optional; if `true`, calculates stats strictly for the current filtered search results)
  - `game_ids`: `number[]` (optional; calculates stats strictly for an explicit list of game IDs)
  - `filter`: `GameFilter` (optional; dynamically filters games by player, date, ECO, rating, etc. before computing position tree)
- **Returns**: `{ fen, total_games, white_pct, draw_pct, black_pct, moves: [{ san, uci, total_games, white_pct, draw_pct, black_pct, avg_white_elo, avg_black_elo, sample_game_ids }], sample_game_ids }`

### `pos_index_status`
Checks the companion `.pos.idx` index status (`valid`, `outdated`, `missing`) and game counts.
- **Returns**: `{ status: "valid" | "outdated" | "missing", header: {...}, loaded: boolean, unique_positions: number }`

### `build_pos_index`
Constructs or rebuilds the companion `.pos.idx` (v3) index in parallel across all CPU cores. Emits streaming `build_pos_index_progress` events.
- **Params**:
  - `max_ply`: `number` (optional, default: 24 plies / 12 moves)
  - `max_games`: `number` (optional, default: 0 for all games in inverted index)
  - `min_games`: `number` (optional, default: 1; minimum game occurrences for a position to be indexed, filtering out rare one-off positions)
  - `threads`: `number` (optional worker thread count)
- **Returns**: `{ status: "valid", unique_positions: number, elapsed_ms: number, diagnostics: {...} }`

### `diag_pos_idx`
Scans and analyzes the companion `.pos.idx` memory map, calculating Delta-Varint vs Roaring Bitmap posting list distributions, compression savings, and move bucket metrics.
- **Returns**: `{ total_game_sets, delta_varint_count, roaring_count, bytes_adaptive, bucket_1_10, bucket_11_100, bucket_101_1k, ... }`

### `search_material`
Searches by bitboard piece count and opposite/same-colored bishops.
- **Params**:
  - `white_rooks`, `black_rooks`, etc.: `number`
  - `opposite_bishops`: `boolean`
  - `same_bishops`: `boolean`
- **Returns**: `{ matches: [game_ids], match_count, total_games, elapsed_ms }`

### `benchmark`
Runs comprehensive multi-threaded benchmarks on the opened database.
- **Params**:
  - `heavy`: `boolean` (optional, default `false`)
- **Returns**: `BenchmarkReport` object with detailed timings for open, index, sort, filter, and search.

### `import_pgn`
Imports games from a `.pgn` file into the active database. Emits streaming progress events (`import_progress`).

### `export_pgn`
Exports the database to a `.pgn` file at ultra-fast speeds. Emits streaming progress events (`export_progress`).

### `add_game`, `update_game`, `delete_game`, `undelete_game`, `compact`, `save`
Mutation commands for editing games, marking deletions, reclaiming dead space, and writing companion files.
