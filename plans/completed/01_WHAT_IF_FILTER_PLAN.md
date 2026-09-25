# 🔮 `what_if` Hypothetical Engine & Board Mutator Specification

> **Feature Plan & Architecture Design Document**  
> **Target Version**: Next Sprint  
> **Author**: Antigravity & User  
> **Status**: Completed (Implemented & Tested)  

---

## 1. Executive Overview

The `what_if` (or `whatif`) filter is a **hypothetical board mutator and sandbox execution engine**. Unlike `play` (which exclusively simulates legal candidate moves from the current position), `what_if` allows speculative board modifications, piece removals, arbitrary transfers, null moves (passing), turn switches, and multi-move variation rollouts within an isolated `{ ... }` scope.

---

## 2. Core Design Principles

1. **Strict Scoped Isolation**:
   - `what_if <mutation> { <sub_query> }` creates an isolated clone of the board (`pos.clone()`).
   - Mutations are applied inside the sandbox.
   - `<sub_query>` is evaluated against the mutated position.
   - Upon exiting `{ ... }`, the sandbox is destroyed and evaluation reverts back to the current position.
2. **Arbitrary Query Composability**:
   - Any valid filter (`play`, `parent`, `child`, `fork`, `pin`, `material`, `checkmate`, etc.) can be nested inside `what_if { ... }`.
3. **Zero Impact on Real Timeline**:
   - Never mutates the underlying PGN/SCID game record.
4. **Legality & Validation Safeguards**:
   - Board mutations that leave impossible states (e.g. 0 Kings, or King captured) fail safely or are rejected by the validator.

---

## 3. Syntax & Mutation Primitives

### 3.1. Piece Removal (`remove` / `without`)
Removes one or more pieces from specific squares or square sets:
```text
what_if remove f6 { play { mate } }
what_if remove [f6, g7] { play from Q { mate } }
what_if without knight on f6 { check }
```

### 3.2. Null Move / Threat Detection (`pass` / `null_move`)
Switches the turn without moving any pieces to check for immediate threats or Zugzwang:
```text
what_if pass { play { mate } }
what_if null_move { play queen { fork(queen, king, rook) } }
```

### 3.3. Arbitrary Piece Transfer (`move <from> to <to>`)
Moves a piece from one square to another, bypassing normal movement rules:
```text
what_if move f3 to d5 { fork(knight, queen, rook) }
what_if move g1 to h8 { mate }
```

### 3.4. Piece Insertion (`add <piece> on <square>`)
Places a new piece onto an empty square:
```text
what_if add Q on e5 { mate }
what_if add [B, N] on [d4, e4] { check }
```

### 3.5. Square & Color Swapping (`swap` / `swap_color`)
Swaps pieces between two squares or inverts a piece's color:
```text
# Swap the pieces on square 1 and square 2:
what_if swap g1 f1 { not check }
what_if swap d5 d1 { fork(queen, king, rook) }

# Invert color of piece on square (White <-> Black):
what_if swap_color c4 { is_attacked(k) }
```

### 3.6. Turn Switch (`turn <color>`)
Explicitly sets whose turn it is to move:
```text
what_if turn white { play { mate } }
what_if turn black { play { check } }
```

### 3.7. Multi-Move Hypothetical Sequence
Simulates playing a sequence of moves (even if not played in the game):
```text
what_if [e4 e5 Qh5] { attacks(queen, f7) }
```

---

## 4. AST Representation & Parser Plan

### 4.1. AST Enum Extension (`src/search/query.rs`)
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum BoardMutation {
    RemoveSquares(Vec<shakmaty::Square>),
    Pass,
    Transfer {
        from: shakmaty::Square,
        to: shakmaty::Square,
    },
    AddPiece {
        piece: shakmaty::Piece,
        square: shakmaty::Square,
    },
    SetTurn(shakmaty::Color),
    MoveSequence(Vec<MovePattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchQuery {
    // ... existing variants ...
    WhatIf {
        mutation: BoardMutation,
        query: Box<SearchQuery>,
    },
}
```

### 4.2. Parser Grammar (`src/search/parser/mod.rs`)
```text
WhatIfExpr := ("what_if" | "whatif") MutationSpec "{" QueryExpr "}"

MutationSpec :=
    | "remove" SquareSet
    | "without" PieceSpec "on" SquareSet
    | "pass" | "null_move"
    | "turn" ("white" | "black" | "w" | "b")
    | "move" Square "to" Square
    | "add" Piece "on" Square
    | "[" MovePattern+ "]"
```

---

## 5. Evaluation Mechanics (`src/search/evaluator/matcher.rs`)

```rust
SearchQuery::WhatIf { mutation, query } => {
    let mut sandbox_pos = pos.clone();
    let mutation_ok = apply_board_mutation(&mut sandbox_pos, mutation);
    if !mutation_ok {
        return false;
    }
    matches_single_ply(query, &sandbox_pos, ply, last_move)
}
```

---

## 6. Real-World Chess Use Cases

| Use Case | Example Query |
| :--- | :--- |
| **Overloaded Defender Detection** | `what_if remove f6 { play { mate } } and not play { mate }` |
| **Unstoppable Threat / Forcing Attack** | `what_if pass { play { mate } }` |
| **Zugzwang Search** | `what_if pass { not is_attacked(k) } and play { is_attacked(k) }` |
| **Ideal Outpost Potential** | `what_if move N to d5 { fork(N, k, q) }` |
| **Tactical Defense Verification** | `what_if remove d8 { attacks(R, d8) }` |

---

## 7. Implementation Checklist

- [ ] Add `BoardMutation` and `SearchQuery::WhatIf` to `src/search/query.rs`.
- [ ] Add `what_if` keyword and mutation specifier parsers in `src/search/parser/`.
- [ ] Add `apply_board_mutation` engine with legality validation in `src/search/evaluator/matcher.rs`.
- [ ] Add explanation support in `src/search/explain.rs`.
- [ ] Add test suite in `src/search/tests.rs` (zugzwang, overloaded piece removal, threat detection).
- [ ] Update documentation in `docs/manual/`.
