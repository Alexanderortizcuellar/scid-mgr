# 📋 Pending Tasks, Roadmap & Deprecation Schedule

This document tracks upcoming architectural features, server improvements, API unifications, and scheduled deprecations for `scid-mgr`.

---

## 🚀 Active Roadmap & Upcoming Tasks

### 1. Unified Reference Command (`reference` / `query_reference`)
- [x] **Implementation**: Implemented single unified endpoint for the Reference Tab covering all 4 widgets in one pass:
  - **Tree**: Opening move statistics (`white_pct`, `draw_pct`, `black_pct`, `white_wins`, `draws`, `black_wins`, `sample_game_ids`).
  - **Continuations**: Multi-ply variation lines.
  - **Endgames**: Pawn / piece endgame frequency distributions.
  - **Games List**: In-memory reference search session creation (`ref_X` in the 3-slot LRU pool) with first page of games returned.
- [x] **Granular Control via Flags**:
  - `include_tree` (boolean, default: `true`)
  - `include_continuations` (boolean, default: `false`)
  - `include_endgames` (boolean, default: `false`)
  - `include_games` (boolean, default: `true`, returns `search_id` and initial page slice)
- [x] **Optimization**:
  - Unfiltered fast-path: Reads from `.tree.idx`, `.hot.idx`, `.feat.idx`, `.pos.idx` in < 0.2 ms.
  - Filtered / dynamic pass: Evaluates candidate `game_ids` once via `.boost.idx` and reuses the candidate vector for tree, continuations, and endgames simultaneously.

---

## 📦 Scheduled Deprecations & Cleanups

### 1. Legacy 2-Step `include_continuations` in `opening_tree`
- **Current Status**: Maintained for full backward compatibility with existing GUI versions.
- **Pending Task**: Once the frontend GUI migrates to the unified `reference` command (which provides `include_continuations: true` alongside `include_tree`, `include_endgames`, and `include_games`), remove `include_continuations` from `opening_tree` so that `opening_tree` focuses solely on 1-ply move branch statistics.
- **Action Item**:
  - [ ] Keep `handle_opening_tree` backward-compatible for now.
  - [ ] Deprecate and remove after GUI migration.

---

## 🔍 Standalone Endpoint Decoupling

For optimal client efficiency, the standalone endpoints remain available for on-demand widget fetches (e.g. when a user unhides or docks an individual panel without re-running the full reference command):
- `opening_tree`: Pure 1-ply tree statistics for a position or `search_id`.
- `continuations`: Dedicated multi-ply continuation line lookups for a position or `search_id`.
- `endgames`: Endgame taxonomy breakdown for a position or `search_id`.
- `query_games`: Virtual pagination and multi-column sorting through `main` or `reference` search sessions.
