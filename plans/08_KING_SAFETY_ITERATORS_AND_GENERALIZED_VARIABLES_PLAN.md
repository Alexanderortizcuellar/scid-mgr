# 🛡️ Architectural Plan: King Safety Evaluation, Board Iterators & Generalized Variable Binding

> **Status:** 📋 Design / Architecture Draft  
> **Target Version:** `0.8.x`  
> **Key Topics:** `king_safety` heuristic models, `for_square` / `for_piece` universal quantifiers ($\exists$, $\forall$), Generalized Variable System (`let $var = ...`) for all value/filter types.

---

## 📌 Executive Summary

This plan addresses three major areas for SCID-MGR's language and evaluation engine:
1. **`king_safety` Filter**: Evaluating structural and tactical threats around the king (pawn shield integrity, open files, attacker/defender balance, storming pawns, pin rays). *Note: Contains open architectural decisions regarding subjective scoring vs. objective structural metrics.*
2. **Quantified Board Iterators (`for_square` / `for_piece`, `any_...`, `all_...`)**: Iterating over square sets and active pieces with first-order logic ($\exists$ exists, $\forall$ for all).
3. **Generalized Variable Binding System**: Extending variables beyond piece bindings (`$p = piece ...`) to capture **any expression return type** (`SquareSet`, `Integer`, `Float`, `Boolean`, `Piece`, `Square`, `MovePattern`).

---

## 1. 🛡️ `king_safety` Filter Design & Nuances

### 1.1 The Challenge of Subjectivity in King Safety
As noted in design discussions, assigning a simple `0..100` score to King Safety is inherently subjective across different chess engines and human heuristics. What looks like a loose king could be completely safe if counterplay or piece activity is high.

### 1.2 Proposed Hybrid Model: Objective Component Filters + Composite Metric

To resolve this nuance, `king_safety` should support both **objective structural attributes** (unambiguous chess facts) and an **optional composite score**:

```cql
// 1. Objective Shield Inspections (Unambiguous)
king_safety white {
    shield_pawns <= 1          // Missing 2 or 3 shield pawns in front of the castled king
    open_files >= 1            // Open or semi-open files directly in the king's zone
    fianchetto_hole == true    // Missing fianchetto bishop on g2/b2 with g3/b3 pushed
}

// 2. Tactical Pressure & Attackers (Count-based)
king_safety black {
    attackers >= 3             // 3 or more enemy pieces exerting direct pressure on king ring
    defenders <= 1             // Only 1 or fewer friendly pieces within 2 squares of the king
    pinned_rays >= 1           // King is aligned on a pin ray with an enemy queen/rook/bishop
}

// 3. Composite Heuristic Score (Evaluated across 0..100 or penalty units)
king_safety white < 40         // Composite safety index
```

### 1.3 Bitboard Calculation Pipeline
King safety checks will execute via bitboards in $< 25\text{ ns}$ per position:
* **King Zone Mask**:
  $$\text{Zone}_{\text{inner}} = \text{KingAttacks}(\text{sq})$$
  $$\text{Zone}_{\text{outer}} = \text{Expand}(\text{Zone}_{\text{inner}}) \setminus \text{Zone}_{\text{inner}}$$
* **Attacker Weight Matrix**:
  $$\text{AttackPower} = 4 \times \text{Queens} + 3 \times \text{Rooks} + 2 \times \text{Minors}$$
* **Shield Matrix**: Bitwise AND against expected pawn files ($f, g, h$ for O-O, $a, b, c$ for O-O-O).

---

## 2. 🔁 Quantified Board Iterators (`for_square` & `for_piece`)

### 2.1 Motivation
Allowing users to express $\exists$ ("there exists at least one") and $\forall$ ("for all") conditions across boards without hardcoding 64 separate checks.

### 2.2 Grammar & Syntax

```cql
// Existential Quantifier (any / exists) - Default behavior
for_square $sq in <SquareSet> { <Condition> }
for_piece $p in <PieceFilter> { <Condition> }

// Explicit Universal Quantifier (all / forall)
all_squares $sq in <SquareSet> { <Condition> }
all_pieces $p in <PieceFilter> { <Condition> }
```

### 2.3 Example Use Cases

```cql
// Example 1: Trapped pieces (Find any Black rook that has 0 legal moves)
for_piece $r in black R {
    mobility $r == 0
}

// Example 2: Complete piece concentration
// (Every white minor piece is located on ranks 1..4)
all_pieces $p in white [N, B] {
    square $p in [a1..h4]
}

// Example 3: Over-attacked square
// (Find any square in Black's camp attacked by at least 3 White pieces and defended by 0 Black pieces)
for_square $sq in [a6..h8] {
    attacks from white to $sq >= 3 and
    attacks from black to $sq == 0
}

// Example 4: Double-pinning piece
// (A piece that is pinning at least 2 distinct enemy targets)
for_piece $p in white [Q, R, B] {
    count(pins by $p) >= 2
}
```

---

## 3. 📦 Generalized Variable Binding System (`let $var = ...`)

### 3.1 Current Limitation vs. Generalized Architecture
Currently, variables are primarily bound during piece placement (e.g. `$q = piece Q on e4`).  
In the generalized architecture, **any expression that yields a value** can be assigned to a variable and reused downstream.

### 3.2 Supported Variable Types & Expressions

| Value Type | Syntax Example | Operations Supported |
| :--- | :--- | :--- |
| **`SquareSet` (Bitboard)** | `let $weak_sqs = [a6..h8] & attacks from white` | `count($weak_sqs)`, `in $weak_sqs`, set union/diff |
| **`Square`** | `let $k_sq = square of white K` | `rank($k_sq)`, `file($k_sq)`, `distance($k_sq, $target)` |
| **`Piece`** | `let $attacker = piece on e4` | `color($attacker)`, `type($attacker)`, `mobility($attacker)` |
| **`Integer` / `Float`** | `let $delta = material white - material black` | Arithmetic (`+`, `-`, `*`, `/`), comparisons (`<`, `>=`, `==`) |
| **`Boolean`** | `let $is_endgame = (material white + material black) < 20` | `if ($is_endgame) { ... }`, `and`, `or`, `not` |
| **`MovePattern`** | `let $killer_move = move from $p to $sq` | `play $killer_move { mate }` |

### 3.3 Example Query with Generalized Variables

```cql
// 1. Store White King's position
let $wk = square of white K

// 2. Compute the set of squares attacked by Black around the White King
let $danger_zone = king_zone($wk) & attacks from black

// 3. Match if White is defending under severe danger zone pressure
count($danger_zone) >= 3 and
material white >= material black
```

---

## 4. 🛠️ AST & Engine Integration

### 4.1 AST Modifications in `src/search/query.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchQuery {
    // ... Existing AST variants ...

    /// Quantified iteration over square sets
    ForSquare {
        var_name: String,
        square_set: Box<SquareSetExpr>,
        quantifier: QuantifierMode, // Any (Exists) vs All (Universal)
        body: Box<SearchQuery>,
    },

    /// Quantified iteration over board pieces
    ForPiece {
        var_name: String,
        piece_filter: Box<SearchQuery>,
        quantifier: QuantifierMode,
        body: Box<SearchQuery>,
    },

    /// Generalized variable binding
    LetBinding {
        var_name: String,
        expression: ValueExpression,
        body: Box<SearchQuery>,
    },

    /// King safety inspection
    KingSafety {
        color: Color,
        conditions: Vec<KingSafetyCondition>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuantifierMode {
    Any, // True if at least 1 element matches
    All, // True if all elements match
}
```

### 4.2 Variable Environment (`VarEnv`)
The search evaluator already passes an evaluation environment down the tree. We extend `VarEnv` to store heterogeneous values:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    SquareSet(BitBoard),
    Square(Square),
    Piece(Piece),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Move(MoveRecord),
}

pub struct VarEnv {
    bindings: HashMap<String, RuntimeValue>,
}
```

---

## 5. 📋 Implementation Roadmap

1. **Phase 1: Generalized Variable Binding Engine (`VarEnv` & `let $var = ...`)**
   - Implement `RuntimeValue` enum and dynamic resolution in evaluator.
   - Support `SquareSet`, `Integer`, `Square`, `Piece`, and `Boolean` bindings.
2. **Phase 2: Quantified Iterators (`for_square` & `for_piece`)**
   - Implement parser for `for_square $sq in ...` and `for_piece $p in ...`.
   - Implement short-circuit evaluation for `Any` (early exit on `true`) and `All` (early exit on `false`).
3. **Phase 3: King Safety Bitboard Heuristics**
   - Implement King Ring bitboard generation and pawn shield masks.
   - Expose objective properties: `shield_pawns`, `open_files`, `attackers`, `defenders`.
   - Formulate optional composite scoring model with tunable weights.
