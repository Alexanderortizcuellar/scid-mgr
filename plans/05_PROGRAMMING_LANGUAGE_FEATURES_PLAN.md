# 💻 Scripting Engine & Programming Language Extensions Plan

> **Design & Engineering Specification Document**  
> **Target**: Language & Scripting Evolution  
> **Status**: Concept & Architecture Planning  

---

## 1. Executive Summary

This plan specifies the evolution of the SCID-MGR chess search engine into a complete **Turing-complete Chess Scripting Language**. It introduces control flow (`if`/`else`, `for`, `while`), user-defined functions (`fn`), a rich type system (`String`, `Array`, `Square`, `Piece`, `SquareSet`), object methods (`.upper()`, `.contains()`, `.distance_to()`), and a mathematical standard library (`max`, `min`, `avg`, `sum`, `sqrt`, `abs`).

---

## 2. Control Flow Architecture

### 2.1. Conditionals (`if` / `else if` / `else`)
Allows branching evaluation based on dynamic position or header metrics:
```text
if move_number <= 20 {
    fork(knight, queen, rook)
} else if move_number <= 40 {
    piece [R, Q] on rank 7
} else {
    passed_pawns >= 2
}
```

### 2.2. Loops (`for` & `while`)

#### A. Collection `for` Loop
Iterates over square sets, piece arrays, or integer ranges:
```text
# Loop over key squares
for $sq in [d4, e4, d5, e5] {
    piece N on $sq and attacks($sq, k)
}

# Loop over piece types
for $p in [N, B, R] {
    pin($p, king)
}

# Loop over ply ranges
for $ply in 1..20 {
    check
}
```

#### B. Bounded `while` Loop
```text
while legal > 0 {
    # Loop body with maximum iteration safeguard (e.g. max 1,000 iterations)
}
```

---

## 3. User-Defined Functions (`fn`)

Users can define reusable tactical and geometric building blocks with parameters and return expressions:

```text
# Simple tactical macro
fn greek_gift() {
    path [Bxh7+ Kxh7 Ng5+ Kg8 Qh5]
}

# Parameterized geometric function
fn battery($slider, $target_file) {
    piece $slider on file $target_file and piece $slider count >= 2
}

# Invocation in search queries
greek_gift() and avg_elo >= 2600
battery(R, "e") and move e5
```

---

## 4. Type System & Built-in Methods

### 4.1. Supported Data Types
* **Primitives**: `bool`, `int`, `float`, `string`
* **Chess Primitives**: `square`, `piece`, `color`, `role`
* **Collections**: `array` / `list`, `squareset`

### 4.2. Object Methods by Type

#### A. String Methods (`$str.*`)
* `.upper()`, `.lower()`: Case conversion.
* `.startswith(prefix)`, `.endswith(suffix)`: Prefix/suffix testing.
* `.contains(substr)`: Substring match.
* `.replace(old, new)`: String substitution.
* `.len()`: String length.
* `.trim()`: Whitespace stripping.

```text
event.lower().contains("world championship") and white.endswith("ov")
```

#### B. Square Methods (`$sq.*`)
* `.rank()`: Integer rank ($1 \dots 8$).
* `.file()`: Character/index file ($a \dots h$).
* `.is_light()`, `.is_dark()`: Square color testing.
* `.distance_to($other_sq)`: Chebyshev / Manhattan distance.

```text
K.square().distance_to(k.square()) <= 2
```

#### C. Piece Methods (`$piece.*`)
* `.color()`: Color of piece (`white` / `black`).
* `.role()`: Role of piece (`king`, `queen`, `rook`, `bishop`, `knight`, `pawn`).
* `.value()`: Standard centipawn/material point value.

#### D. Array & SquareSet Methods (`$arr.*` / `$set.*`)
* `.len()`, `.count()`: Element count.
* `.is_empty()`: Emptiness check.
* `.contains(item)`: Inclusion check.
* `.first()`, `.last()`: Boundary elements.

---

## 5. Mathematical & Statistical Standard Library (`stdlib`)

Built-in mathematical operations available for Elo computations, material differences, and move numbers:

| Function | Signature | Description | Example |
| :--- | :--- | :--- | :--- |
| **`max`** | `max(a, b, ...)` | Maximum value | `max(white_elo, black_elo) >= 2800` |
| **`min`** | `min(a, b, ...)` | Minimum value | `min(white_elo, black_elo) >= 2600` |
| **`abs`** | `abs(n)` | Absolute value | `abs(white_elo - black_elo) <= 50` |
| **`sum`** | `sum(list)` | Sum of numbers | `sum([white_knights, white_bishops]) >= 3` |
| **`avg`** | `avg(list)` | Average of numbers | `avg(white_elo, black_elo) >= 2700` |
| **`sqrt`** | `sqrt(n)` | Square root | `sqrt(material_diff) <= 2` |
| **`clamp`**| `clamp(x, min, max)` | Bounds a value | `clamp(move_number, 1, 40)` |

---

## 6. Rust Interpreter Architecture

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Square(shakmaty::Square),
    Piece(shakmaty::Piece),
    SquareSet(shakmaty::Bitboard),
    Array(Vec<ScriptValue>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    If {
        condition: Box<Expression>,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
    For {
        var_name: String,
        iterable: Box<Expression>,
        body: Vec<Statement>,
    },
    While {
        condition: Box<Expression>,
        body: Vec<Statement>,
    },
    FunctionDef {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    Query(SearchQuery),
}
```

---

## 7. Implementation Milestones

1. **Phase 1**: Math stdlib (`max`, `min`, `abs`, `sum`, `avg`, `sqrt`) in expressions.
2. **Phase 2**: String and Square methods (`.lower()`, `.contains()`, `.distance_to()`).
3. **Phase 3**: Conditionals (`if`/`else`) and user-defined functions (`fn`).
4. **Phase 4**: Iterators and `for`/`while` loops with safety bounds.
