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
Performs Zobrist-hashed binary position search.
- **Params**:
  - `fen`: `string` (full FEN)
  - `max_ply`: `number` (optional maximum search depth)
- **Returns**: `{ matches: [{ game_id, ply }], total_games_searched, elapsed_ms }`

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
