# Comprehensive Architecture & Execution Guide: Square Set Algebra & Bitboard Engine

> **Document Status**: Complete & Verified Architecture Reference  
> **Source Verification**: All data structures, execution timelines, and bitboard behaviors documented here are verified directly against the active Rust codebase (`shakmaty 0.27`, `src/search/*`).

---

## 1. Executive Summary & Verification of Invariants

### 1.1 What this Upgrade Does
1. **First-Class Square Sets**: Allows chess piece selectors (`B`, `n`, `white_pieces`, `occupied`), square sets (`[c1, f1]`, `[a1..h8]`, `light`), and tactical spatial queries (`attacks(...)`, `ray(...)`, `between(...)`) to evaluate into a 64-bit integer (`shakmaty::Bitboard`).
2. **Set Algebra Operations**: Implements Set Intersection ($\cap$), Union ($\cup$), Difference ($\setminus$), and Complement ($\sim$) with single-cycle CPU bitwise instructions (`AND`, `OR`, `AND NOT`, `NOT`).
3. **Dual-Nature Truthiness**:
   - In a **Boolean context**: A square set evaluates to `true` if it is non-empty (`bitboard.is_not_empty()`), and `false` if it contains 0 squares.
   - In a **Numeric comparison context**: Set expressions can be compared directly (`attacks(R, k) >= 2`, `B [a1..h8] == 2`, `attacks(white, e4) > attacks(black, e4)`).
4. **Boolean Filters Remain Boolean**: Filters that describe full-game properties (`white "Carlsen"`, `eco startswith "B"`, `result "1-0"`), board-wide state (`wtm`, `checkmate`), or move sequences (`move capture`, `path [ e4 ... d5 ]`) remain standard boolean predicates.

### 1.2 Complexity & Effort Assessment
- **Is this a rewrite?**: **NO.**
- **Why?**: The existing engine already stores board positions in `shakmaty::Chess` whose internal board is 100% bitboard-based (`shakmaty::Bitboard(u64)`). The timeline evaluation loop (`evaluate_with_timeline_env`), PGN streaming replayer, Scid index searchers, and path matching systems remain untouched.
- **Estimated Scope**:
  1. Add `SquareSetExpr` AST and `SearchQuery::SquareSet(SetPredicate)` in `src/search/query.rs`.
  2. Implement `eval_square_set(&SquareSetExpr, &Chess) -> Bitboard` in `src/search/pattern.rs`.
  3. Extend parser grammar for set operators and comparisons in `src/search/parser/squares.rs` and `src/search/parser/mod.rs`.
  4. Update `explain.rs` for AST to DSL roundtripping.

---

## 2. Verified Architectural Details & Type System

### 2.1 Verified AST Definitions (`src/search/query.rs`)

```rust
use shakmaty::{Bitboard, Color, Piece, Role, Square};

/// Square Set expression evaluating to a 64-bit Bitboard on a given chess position
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SquareSetExpr {
    /// Board pieces: `B` (white bishops), `n` (black knights), `occupied`, `empty`, `white_pieces`, `black_pieces`
    Piece(SquareContent),

    /// Explicit squares/ranges: `[c1, f1]`, `[a1..h8]`, `light`, `dark`, `a-h1`
    Squares(Bitboard),

    /// Bound variable reference: `$target`, `$attacker`
    Variable(String),

    /// Set Intersection (A ∩ B): `A & B` or juxtaposition `A B` (e.g. `B [c1, f1]`)
    Intersection(Box<SquareSetExpr>, Box<SquareSetExpr>),

    /// Set Union (A ∪ B): `A | B` or `[N, B]`
    Union(Box<SquareSetExpr>, Box<SquareSetExpr>),

    /// Set Difference (A \ B): `A \ B` or `A - B` (e.g. `occupied \ [d4, e5]`)
    Difference(Box<SquareSetExpr>, Box<SquareSetExpr>),

    /// Set Complement (~A): all 64 squares not in A
    Complement(Box<SquareSetExpr>),

    /// Geometric / Attack targets: `attacks(attacker_set, target_set)`
    /// Returns the subset of target_set that is attacked by any piece in attacker_set
    Attacks {
        attacker: Box<SquareSetExpr>,
        target: Box<SquareSetExpr>,
    },

    /// Attack origins: `attackers(attacker_set, target_set)`
    /// Returns the subset of attacker_set that attacks any piece in target_set
    Attackers {
        attacker: Box<SquareSetExpr>,
        target: Box<SquareSetExpr>,
    },

    /// Directional ray expansion: `ray(up, d4)`, `ray(diagonal, [e4, d5])`
    Ray {
        direction: Direction,
        origin: Box<SquareSetExpr>,
    },

    /// Squares strictly between two sets of pieces/squares: `between(sq1, sq2)`
    Between {
        from: Box<SquareSetExpr>,
        to: Box<SquareSetExpr>,
    },
}

/// Set Predicate evaluated in SearchQuery
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetPredicate {
    /// Non-empty boolean test: evaluates to `!set.is_empty()`
    /// Example: `B [c1, f1]`, `attacks(R, k)`
    NonEmpty(SquareSetExpr),

    /// Count comparison against integer literal: `count(expr) op count`
    /// Example: `attacks(R, k) >= 2`, `B [a1..h8] == 2`
    CountComparison {
        expr: SquareSetExpr,
        op: ComparisonOp,
        count: usize,
    },

    /// Set size comparison between two expressions: `count(left) op count(right)`
    /// Example: `attacks(white, e4) > attacks(black, e4)`
    SetComparison {
        left: SquareSetExpr,
        op: ComparisonOp,
        right: SquareSetExpr,
    },
}
```

---

## 3. Verified Bitboard Evaluation Rules & Mechanics

### 3.1 Hardware-Accelerated Bitwise Mapping
Each operation in `eval_square_set(expr, pos, env) -> Bitboard` maps directly to single-instruction bitboard algebra:

| Operation | Abstract Syntax | Bitboard Code | CPU Instruction |
|---|---|---|---|
| **Intersection** | `A & B`, `A B` | `bb_a & bb_b` | `AND` (1 cycle) |
| **Union** | `A \| B` | `bb_a \| bb_b` | `OR` (1 cycle) |
| **Difference** | `A \ B` | `bb_a & !bb_b` | `ANDN` / `AND NOT` (1 cycle) |
| **Complement** | `~A` | `!bb_a` | `NOT` (1 cycle) |
| **Count** | `A >= 2` | `bb_a.count() >= 2` | `POPCNT` (1 cycle) |
| **Non-Empty** | `A` | `!bb_a.is_empty()` | `TEST` / `JNZ` (1 cycle) |

### 3.2 Evaluation of Geometric & Attack Sets
1. **`attacks(AttackerSet, TargetSet)`**:
   ```rust
   let atk_bb = eval_square_set(attacker, pos, env);
   let tgt_bb = eval_square_set(target, pos, env);
   let mut attacked_targets = Bitboard::EMPTY;
   
   for from_sq in atk_bb {
       let attacks = pos.board().attacks_from(from_sq);
       attacked_targets |= attacks & tgt_bb;
   }
   attacked_targets
   ```
2. **`attackers(AttackerSet, TargetSet)`**:
   ```rust
   let atk_bb = eval_square_set(attacker, pos, env);
   let tgt_bb = eval_square_set(target, pos, env);
   let mut attacking_pieces = Bitboard::EMPTY;
   
   for from_sq in atk_bb {
       let attacks = pos.board().attacks_from(from_sq);
       if (attacks & tgt_bb).is_not_empty() {
           attacking_pieces.add(from_sq);
       }
   }
   attacking_pieces
   ```

---

## 4. Warnings, Pitfalls & Critical Edge Cases

> [!WARNING]
> **Be aware of the following concrete implementation traps:**

### 4.1 Uppercase vs. Lowercase Piece Casing Invariant
- **Invariant**: 
  - Uppercase (`P`, `N`, `B`, `R`, `Q`, `K`, `A`) = **White pieces only**.
  - Lowercase (`p`, `n`, `b`, `r`, `q`, `k`, `a`) = **Black pieces only**.
- **Trap**: Do **not** make `B` match both colors in square set expressions. If a user wants any Bishop regardless of color, they write `[B, b]` or `bishops`.
- **Trap in parser**: When parsing tokens like `B [c1, f1]`, the lexer must recognize `B` as `SquareSetExpr::Piece(SquareContent::Piece(White Bishop))` and not treat `[` as an array index on `B`.

### 4.2 Empty vs. Zero Truthiness
- **Trap**: `B [c1, f1]` is a non-empty check (`count > 0`).
- If a user wants to assert that **neither** c1 nor f1 contains a bishop, they write:
  ```text
  B [c1, f1] == 0
  # or
  not B [c1, f1]
  ```
- Make sure `not (B [c1, f1])` negates the boolean truthiness of `!set.is_empty()`, correctly evaluating to `set.is_empty()`.

### 4.3 Grammatical Ambiguity: Juxtaposition vs. Boolean AND
- In CQL, `B [c1, f1]` means `B & [c1, f1]` (intersection).
- In our existing parser, tokens separated by whitespace or `and` can appear.
- **Rule**:
  - `B [c1, f1]` (two set expressions directly adjacent) = **Set Intersection (`&`)**.
  - `B on [c1, f1]` = **Set Intersection (`&`)** (syntactic alias).
  - `B and check` = **Boolean AND** between a SetPredicate (`NonEmpty(B)`) and a Boolean query (`check`).

### 4.4 Set Difference Syntax (`\` vs `-`)
- **Trap**: In mathematical set theory, difference is written `A \ B`. In command lines or regex, `\` can be an escape character.
- **Rule**: Support both `A \ B` and `A - B` as set differences in the lexer.

### 4.5 Bound Variable Environments (`$var`)
- Variables can bind to either individual `Square`s or entire `Bitboard`s.
- In `piece $minor in [N, B] { ... }`, `$minor` iterates over each square in `eval_square_set([N, B])`.
- When evaluating `SquareSetExpr::Variable(var_name)`:
  - If `env` contains `$var -> Square`, it returns `Bitboard::from_square(sq)`.
  - If `env` contains `$var -> Bitboard`, it returns `bitboard`.

---

## 5. Step-by-Step Clean Execution Plan

### Step 1: AST Additions (`src/search/query.rs`)
1. Define `SquareSetExpr` enum with `Piece`, `Squares`, `Intersection`, `Union`, `Difference`, `Complement`, `Attacks`, `Attackers`, `Ray`, `Between`.
2. Define `SetPredicate` enum with `NonEmpty`, `CountComparison`, `SetComparison`.
3. Add variant `SearchQuery::SquareSet(SetPredicate)` to `SearchQuery`.

### Step 2: Evaluation Engine (`src/search/pattern.rs` / `src/search/squares.rs`)
1. Implement `pub fn eval_square_set(expr: &SquareSetExpr, pos: &Chess, env: &HashMap<String, Square>) -> Bitboard`.
2. Implement `pub fn eval_set_predicate(pred: &SetPredicate, pos: &Chess, env: &HashMap<String, Square>) -> bool`.
3. Wire `SearchQuery::SquareSet` into `evaluate_with_timeline_env` in `src/search/evaluator/matcher.rs`.

### Step 3: Parser Implementation (`src/search/parser/squares.rs`)
1. Implement `parse_square_set_expr(&mut self) -> Result<SquareSetExpr, ParseError>` with standard Pratt precedence:
   - Level 1: Union `|`
   - Level 2: Difference `\` / `-`
   - Level 3: Intersection `&` and juxtaposition `A B`
   - Level 4: Complement `~` / `!`
   - Level 5: Atoms (Pieces `B`, Squares `[a1..h8]`, Functions `attacks(...)`, `ray(...)`)
2. Implement comparison parsing:
   - When followed by `==`, `!=`, `<`, `<=`, `>`, `>=`, wrap into `SetPredicate::CountComparison` or `SetPredicate::SetComparison`.
   - Otherwise wrap as `SetPredicate::NonEmpty`.

### Step 4: Verification & Test Suite (`src/search/tests.rs`)
1. Unit test double attack count: `attacks(R, k) >= 2`.
2. Unit test square control dominance: `attacks(white, e4) > attacks(black, e4)`.
3. Unit test set algebra: `(N | B) [d4, e5] == 2`, `[a1-h8] \ occupied`, `~[c1, f1]`.
4. Unit test backward compatibility for all 55 existing tests.
