# 🗺️ SCID-MGR Search Engine Development Roadmap

This roadmap tracks the ongoing design, implementation, and optimization of the `scid-mgr` modular chess search engine. Features are categorized into progressive phases.

---

## 📊 Progress Dashboard

- [x] **Core Search Submodule Architecture** (`src/search/`)
- [x] **Metadata & Header Predicates** (White, Black, Elo, EloDiff, Date, Result, ECO, Custom Tags)
- [x] **Position & Board Patterns** (Exact FEN, Piece Placement, Zobrist Hash, Square Contents, Attacks, Piece Counts)
- [x] **Move & Path Patterns** (Single moves, Consecutive sequences, Themed/gapped paths)
- [x] **Hardware Bitboard Material Predicates** (Piece counts, Material point diff, Opposite/Same-colored bishops)
- [x] **Hardware Bitboard Pawn Structure Predicates** (Passed, Isolated, Doubled, Backward pawns, Pawn Islands)
- [x] **Human-Friendly Query DSL Parser** (CQL-like syntax parser in `src/search/parser.rs`)
- [x] **Composite Boolean Logic & Scoping** (`And`, `Or`, `Not`, `PlyRange`, `Occurrences`)
- [x] **Search Engine Reference Documentation** (`docs/SEARCH_ENGINE.md`)
- [x] **Phase 1: Tactical Motifs & Positional Primitives**
- [x] **Phase 2: Native SCID Binary Adapter, Transformations & Annotations**
- [ ] **Phase 3: Cost-Based Query Optimizer & Index Integration**
- [ ] **Phase 4: Piece Maneuvers & King Safety**
- [ ] **Phase 5: JSON-RPC & GUI Frontend Integration**

---

## 🚀 Detailed Phase Breakdown

### 🎯 Phase 1: Tactical Motifs & Positional Primitives

- [x] **Pawn Structures** (Implemented & Bitboard Accelerated):
  - [x] `PassedPawns { color, op, count }` (pawn with no opposing pawns on the same or adjacent files ahead).
  - [x] `IsolatedPawns { color, op, count }` (pawn with no friendly pawns on adjacent files).
  - [x] `DoubledPawns { color, op, count }` (two or more pawns on the same file).
  - [x] `BackwardPawns { color, op, count }` (pawn behind neighboring friendly pawns that cannot advance safely).
  - [x] `PawnIslands { color, op, count }` (number of distinct pawn groups).
- [x] **Tactical Relationships**:
  - [x] `Pin { pinner, pinned, target }` (absolute pin on King or relative pin on Queen/Rook).
  - [x] `Fork { attacker, targets, min_targets }` (simultaneous attack on 2+ high-value pieces).
  - [x] `DiscoveredAttack { color, is_check }` & `DiscoveredCheck`.
  - [x] `TrappedPiece(Role, Color)` (piece with 0 legal / safe departure squares).
  - [x] `Skewer { attacker, front, rear }` (higher value piece in front).
- [x] **Positional Squares & Files**:
  - [x] `Outpost { piece, square }` (knight/bishop protected by friendly pawn on 4th/5th/6th rank, immune to enemy pawn eviction).
  - [x] `OpenFile { file, semi_open_for }` (full open or semi-open file).
  - [x] `RookOnSeventh { color }` (rook occupying 7th rank for White / 2nd rank for Black).
  - [x] `Distance { sq1, sq2, op, distance }` (Chebyshev spatial distance between pieces or squares).

---

### ⚡ Phase 2: Native SCID Binary Adapter, Transformations & Annotations

- [x] **Zero-Copy SCID Adapter** (`src/search/scid_adapter.rs`):
  - [x] Direct evaluation of `(&IndexEntry, &NameTables, &[u8] blob)` without intermediate PGN string allocations.
  - [x] Direct binary move streaming from memory-mapped `.sg4`/`.sg5` files using `decode_raw_move`.
- [x] **Rayon Parallel Execution**:
  - [x] Parallel chunk evaluation across all CPU cores (`ScidSearchAdapter::search_parallel`).
- [x] **Single Position Transformations & Board Symmetries** (`src/search/transform.rs`):
  - [x] Horizontal file mirror (`flip:horizontal`), vertical mirror (`flip:vertical`), 180° rotation (`flip:rotate`).
  - [x] Color inversion perspective flip (`flip:color`).
  - [x] Multi-square piece matching (`piece wn on [d4, b4, c4]`).
- [x] **Comments & Move Annotations** (`src/search/annotation.rs`):
  - [x] Comment substring and regex searching (`comment:"novelty"`).
  - [x] Numeric Annotation Glyphs matching (`nag:!`, `nag:??`, `nag:[1, 3]`).
  - [x] Comment strip and extraction utilities (`AnnotationManager`).
- [ ] **Inverted Index Candidate Pruning**:
  - [ ] Connect AST position/material criteria with `.pos.idx` / `.scidpos5` inverted bitboard keys.

---

### 📝 Phase 3: Text-Based Query DSL & Cost-Based Optimizer

- [x] **Lightweight Query DSL Parser (`src/search/parser/`)**:
  - Human-readable CQLite query syntax:
    ```text
    player "Kasparov" elo >= 2700 eco "B88" tag "TimeControl" == "300+0"
    path [e4 e5 Nf3 d6] consecutive:true
    pin [B, n, k]
    [Qq] == 0 and [RBN] == 2
    move A-- or move --=R or move pxN=q
    ```
  - [x] Full boolean composition (`and`, `or`, `not`, nested parentheses, implicit multiline ANDs).
  - [x] Header predicates & custom tag search (`tag "Key" == "Val"`, `header "Key" contains "Val"`, SCID extra tag decoder).
  - [x] Bracketed piece group sums (`[Qq] == 0`, `[RBN] == 2`, `[KkQq] == 2`).
  - [x] Wildcard move patterns (`A--`, `a--`, `R--`, `--=R`, `--="RBN"`, `pxN=q`, `px=q`, `_--`).
  - [x] Light / dark square piece modifiers (`dark queen`, `light [B, b]`, `light white_pieces >= 6`).
  - [x] Piece power point comparison expressions (`white_power > black_power`, `power >= 78`).
  - [x] Rich parse diagnostics with source offset indicators.
- [ ] **Cost-Based Query Planner**:
  - Re-orders AST predicates to execute the lowest-cost filters first:
    1. Header checks (`IndexEntry` in RAM) — Cost: $O(1)$
    2. Inverted bitboard index (`.pos.idx`) — Cost: $O(\log N)$
    3. Binary move replay (`&blob` streaming) — Cost: $O(M)$ where $M = \text{number of moves}$.

---

### ♟️ Phase 4: Piece Maneuvers, King Safety & Annotations

- [ ] **Piece Trajectory & Maneuver Tracking**:
  - [ ] Search for piece routes across plies (e.g. `Knight: b1 -> d2 -> f1 -> e3 -> d5`).
- [ ] **King Safety & Pawn Storms**:
  - [ ] Castling status (opposite-side castling games).
  - [ ] Pawn storms (advancing pawns against enemy castled king).
  - [ ] King shelter pawn shield integrity.
- [ ] **NAGs & Annotations**:
  - [ ] Filter by move annotations (`!` good move, `?` mistake, `??` blunder, `!!` brilliant, `N` novelty).
  - [ ] Time trouble detection (clock annotations).

---

### 🔌 Phase 5: JSON-RPC & GUI Frontend Integration

- [ ] **JSON-RPC Endpoints**:
  - [ ] `search_query`: Execute raw AST or DSL queries over opened SCID / PGN databases.
  - [ ] `search_cancel`: Cancel running background searches cooperatively.
- [ ] **GUI Query Builder**:
  - [ ] Visual query builder tab in Python GUI (Filter by headers, pieces on board, move sequences, tactics).
  - [ ] Interactive results table highlighting the exact matching plies and moves.
