# 📐 CQLi `line` Filter: Architectural Research & Implementation Plan

This document details the modern specification from the **CQLi Reference Manual ([cql64.com](https://cql64.com/manual/the-line-filter/))**, formal grammar, parameter semantics, and implementation roadmap for the **`line` filter** in `scid-mgr`.

---

## 1. 🔍 Research & Background: Modern CQLi `line` Filter

In CQLi (Modern CQL reference by Robert Gamble), the **`line`** filter searches for a sequential series of positions, starting from the current position, that match a prescribed pattern.

### A. Conceptual Comparison

| Engine / Filter | Navigation Style | Constituent Model | Evaluation Result |
| :--- | :--- | :--- | :--- |
| **`path [...]`** (SCID) | Sequential moves with plies | Raw move tokens with SCID gaps (`...`, `--*`, `--{min, max}`) | Boolean (match found) |
| **`cqlpath { ... }`** (CQL 6.2) | Turnstile ply advance | Interleaved moves & position filters with regex quantifiers `( ... )+` | Boolean (match found) |
| **`line`** (CQLi / Modern CQL) | Position transitions via `-->` or `<--` | Positions evaluated at each transition; quantifiers on positions/moves | **Numeric length** of the longest matching line, or **Boolean truthiness** if length $\ge 1$ within range |

---

## 2. 📋 CQLi `line` Syntax & Grammar

### Canonical Syntax:
```text
line [range_min range_max] [parameters] {--> | <--} constituent1 [{--> | <--} constituent2 ...]
```

> [!NOTE]
> All constituents within a single `line` filter must use the same directional arrow (`-->` forward or `<--` backward). Mixing `-->` and `<--` within the same `line` is invalid.

### Parameters & Modifiers:

| Parameter | Type | Semantics | Example |
| :--- | :--- | :--- | :--- |
| **`[min max]` / `[count]`** | Numeric Range | Restricts the total matched sequence length (number of plies/positions) | `line 5 100 --> check+` |
| **`-->`** | Forward Arrow | Traverses forward to immediate child positions along the game line | `line --> check --> move previous capture --> mate` |
| **`<--`** | Backward Arrow | Traverses backward to parent positions (look-behind) | `mate and line <-- check* <-- Queen` |
| **`firstmatch`** | Optimization | Terminates search immediately after the first valid matching sequence is found | `line firstmatch --> (e4 e5)` |
| **`lastposition`** | State Anchor | Evaluates subsequent filters from the **tail position** of the matching sequence rather than the head | `line lastposition --> check+ and mate` |
| **`nestban`** | Deduplication | Prevents a position from starting a new match if it was already part of an earlier matched line | `line nestban 5 100 --> check+` |
| **`singlecolor`** | Color Scoping | Only considers positions where the side-to-move is the same as the initial starting position | `line singlecolor --> check+` |
| **`primary` / `secondary`** | Tree Traversal | Restricts traversal to primary moves (mainlines) or secondary variations | `line primary --> check+` |
| **`quiet`** | Diagnostics | Suppresses automatic diagnostic/annotated comment generation | `line quiet --> check+` |
| **`nolinearize`** | Variation Mode | Disables linearization of game tree branches | `line nolinearize --> check+` |
| **`nonatomic`** | Execution Mode | Evaluates intermediate states non-atomically | `line nonatomic --> check+` |

---

## 3. 🧩 Constituent Quantifiers & Grouping

Constituents in a `line` filter support regular-expression-like repetition quantifiers:

| Quantifier | Meaning | Example |
| :--- | :--- | :--- |
| **`?`** / **`{?}`** | Match 0 or 1 time | `--> check?` |
| **`*`** / **`{*}`** | Match 0 or more times (greedy) | `--> check*` |
| **`+`** / **`{+}`** | Match 1 or more times (greedy) | `--> check+` |
| **`{min, max}`** | Match between `min` and `max` times | `--> (e4 e5){1, 3}` |
| **`{n}`** | Match exactly `n` times | `--> check{3}` |
| **`( ... )`** | Group multiple constituents into a repeatable sub-chain | `--> ( check --> move previous capture )+` |

---

## 4. 🎯 Expository Examples from CQLi Manual

### 1. Check Resolved by Attacker Capture Followed by Mate
```text
line --> check
     --> move previous capture (flipcolor A attacks k)
     --> mate
```

### 2. Longest Consecutive Check Streak (5 to 100 checks)
```text
line 5 100 nestban --> check+
```

### 3. Checkmate with Look-Behind Queen Checks
```text
mate and line lastposition <-- check* <-- Queen
```

---

## 5. 🏗️ Non-Breaking Architectural Design in `scid-mgr`

```
                  ┌────────────────────────────────────────────────────────┐
                  │                 SearchQuery AST                        │
                  └────────────────────────────────────────────────────────┘
                             │                   │                  │
                ┌────────────┴────┐     ┌────────┴────────┐  ┌──────┴──────────────┐
                │ SearchQuery::   │     │ SearchQuery::   │  │ SearchQuery::       │
                │ Path            │     │ CqlPath         │  │ CqlLine             │
                │ (SCID Moves)    │     │ (CQL 6.2 Path)  │  │ (CQLi Arrow Lines)  │
                └─────────────────┘     └─────────────────┘  └─────────────────────┘
```

### Phase 1: AST Structure (`src/search/query.rs`)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LineDirection {
    Forward,  // -->
    Backward, // <--
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CqlLinePattern {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub direction: LineDirection,
    pub single_color: Option<Option<Color>>,
    pub first_match: bool,
    pub last_position: bool,
    pub nest_ban: bool,
    pub primary_only: bool,
    pub constituents: Vec<CqlPathConstituent>,
}
```

### Phase 2: Parser Grammar (`src/search/parser/moves.rs`)
- Tokenizes and parses `-->` and `<--`.
- Parses length parameters (`min max` or `count`), `nestban`, `singlecolor`, `firstmatch`, `lastposition`.
- Parses constituents linked by arrows.

### Phase 3: Evaluator Engine (`src/search/path.rs`)
- Implements `match_cql_line`:
  - **Forward traversal**: Advances along child plies.
  - **Backward traversal**: Walks backwards from target ply towards root.
  - Returns length of longest match; integrates seamlessly into boolean evaluator and numeric comparison operators (`line --> check+ >= 5`).

### Phase 4: Verification & Docs
- Comprehensive test suite covering forward, backward look-behind, `singlecolor`, and `nestban`.
- Integration into `docs/manual/03_MOVE_AND_PATH_PATTERNS.md` and `docs/SEARCH_ENGINE.md`.
