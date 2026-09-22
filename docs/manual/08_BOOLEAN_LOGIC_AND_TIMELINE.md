# ⏱️ Chapter 8: Boolean Logic, Timeline & Annotations

This chapter covers logical combinators (`and`, `or`, `not`), timeline scopes (`ply in ...`, `move_number`, `occurrences`), and move annotation filters (comments and NAGs).

---

## 📌 Boolean Combinators

| Operator | Description | Precedence | Example |
| :--- | :--- | :--- | :--- |
| **`and`** | Logical Conjunction (both conditions must match) | Higher | `player "Kasparov" and result "1-0"` |
| **`or`** | Logical Disjunction (either condition matches) | Lower | `eco "B90" or eco "B92"` |
| **`not`** | Logical Negation (condition must NOT match) | Highest unary | `not check and legal == 0` |
| **`( ... )`** | Grouping Parentheses | Overrides precedence | `(white "Karpov" or white "Kasparov") and date >= "1985"` |

---

## 🎯 Game-Level Conjunction vs. Position-Anchored Evaluation

Understanding how boolean `and` behaves when combining **moves/paths** with **positional predicates** (like `attacks`, `pin`, `fork`, `check`) is essential:

### 1. Game-Level Conjunction (Default `and`)
When combining top-level path/move patterns with positional motifs using `and`, the engine evaluates each condition across the entire game timeline:
```cql
path [e4] and attacks(R, b)
```
* **How it evaluates:**
  1. `path [e4]` checks if `1. e4` was played in the game.
  2. `attacks(R, b)` scans for **any ply** in the game where a White Rook attacks a Black Bishop.
  3. The query matches if **both** occurred at some point in the game (they do not have to occur at the same ply).

---

### 2. Position-Anchored Conjunction
To anchor the positional condition to a **specific ply relative to a move**, use one of the following precise patterns:

| Desired Evaluation Point | Syntax | Example |
| :--- | :--- | :--- |
| **Immediately After a Move** | `move previous <move>` | `attacks(R, b) and move previous e4`<br>*(Asserts attack right after `e4` is played)* |
| **Immediately Before a Move** | `move <move>` | `attacks(R, b) and move e4`<br>*(Asserts attack on board when next move is `e4`)* |
| **Interleaved in a Sequence** | `cqlpath { <move> { <filter> } }` | `cqlpath { e4 { attacks(R, b) } }`<br>*(Advances by `e4`, then asserts attack)* |
| **Scoped to a Game Phase** | `ply in min..max { <filter> }` | `path [e4] and ply in 1..20 { attacks(R, b) }`<br>*(Opening phase attack)* |

---

## ⏳ Timeline Scopes & Restrictions

Scope search conditions to specific game phases, ply ranges, or move numbers:

### 1. Ply Range Filter (`ply in min..max { ... }`)
Restrict a sub-query to only evaluate between specific plies (1 move = 2 plies):
```text
ply in 1..20 {
    # Opening phase tactical motifs
    fork(knight, queen, rook)
}
```

### 2. Move Number Filter (`move_number [op] [num]`)
Match conditions at specific 1-indexed full move numbers:
```text
move_number <= 10 and queens == 0    # Early queen trade (by move 10)
move_number >= 40 and total_power <= 12 # Deep endgame
```

### 3. Occurrence Count Filter (`occurrences min..max { ... }` or `occurrences >= N { ... }`)
Require a pattern to occur at least $N$ times throughout the game:
```text
occurrences >= 3 {
    check
}  # Games with at least 3 separate checks
```

---

## 🧭 Parent & Child Scoping Filters (`parent { ... }` & `child { ... }`)

The `parent` and `child` filters allow relative temporal queries by shifting the evaluation scope forward or backward relative to the current position:

* **`parent { <filter> }`**: Evaluates the enclosed filter(s) against the **parent position** (`ply - 1`, the board state immediately before the current move).
* **`child { <filter> }`**: Evaluates the enclosed filter(s) against the **child position** (`ply + 1`, the board state resulting from the next move played in the game).

Once execution exits the `{ ... }` block, evaluation reverts back to the current position. Arbitrary nesting and combining with boolean logic is supported.

### Syntax & Common Patterns

| Query Pattern | Description |
| :--- | :--- |
| `mate and parent { not check }` | Matches checkmate positions delivered from an un-checked state (giving mate). |
| `check and parent { check }` | Consecutive checks (double/consecutive checking moves). |
| `child { mate }` | Finds the setup move/position immediately before checkmate is delivered. |
| `parent { wtm } and child { mate }` | Combines conditions across past, present, and future plies. |
| `child { child { mate } }` | Mate in 2 (grandchild position is mate). |
| `parent { parent { ... } }` | Arbitrary multi-ply historical lookback. |

---

## 🔮 Hypothetical Move Simulation & Outcomes (`leads_to` & `play`)

The engine can simulate all candidate legal moves from the current position and evaluate hypothetical future board states that *could* arise—even if they were not the move actually played in the game:

### Syntax & Patterns

1. **`legal <move_filter> leads_to { <outcome_query> }`** (or `move legal ... leads_to { ... }` / `leadsto`):
   Filters legal moves matching `<move_filter>`, simulates playing each move, and tests if the resulting position satisfies `<outcome_query>`.
2. **`play [legal] <move_filter> { <outcome_query> }`**:
   Equivalent convenient prefix syntax for hypothetical move simulation.

### Examples

| Query Pattern | Description |
| :--- | :--- |
| `legal promote B leads_to { stalemate }` | Finds positions where underpromoting to a Bishop leads to stalemate. |
| `play promote Q { stalemate }` | Finds positions where promoting to a Queen results in stalemate. |
| `legal promote B leads_to { stalemate } and legal promote Q leads_to { mate }` | Classic underpromotion study: promoting to Bishop stalemates, promoting to Queen checkmates. |
| `play legal from e4 { check }` | Positions where any legal move from e4 delivers check. |
| `legal count == 1 leads_to { mate }` | Positions with a unique (exactly 1) legal move delivering checkmate (mate in 1). |
| `play q { fork(queen, king, rook) }` | Positions where playing a legal queen move creates a fork on king and rook. |

---

## 💬 Comment & Annotation (NAG) Filters

Query move text commentary and standard Numeric Annotation Glyphs (NAGs):

### 1. Move Comment Searches
Search PGN move comments for annotations, analysis keywords, or engine evaluations:
```text
comment contains "blunder"
comment contains "??"
comment contains "+-"
comment ~ "eval: \\+?[3-9]\\.[0-9]+"
```

### 2. NAG Annotation Filters
Query standard chess annotations:
* `$1` = `!` (Good move)
* `$2` = `?` (Poor move / mistake)
* `$3` = `!!` (Brilliant move)
* `$4` = `??` (Blunder)
* `$5` = `!?` (Interesting move)
* `$6` = `?!` (Dubious move)

```text
nag $3                 # Games containing brilliant moves (!!)
nag $4 and nag $2      # Games with both blunders (??) and mistakes (?)
```

---

## 🎯 Verified Examples

### 1. Early Tactical Miniature
```text
move_number <= 25 and result "1-0" and path [Bxh7 kxh7]
```

### 2. Endgame King Activity
```text
move_number >= 35 and [Qq] == 0 and distance(K, k) <= 3
```

### 3. Comprehensive Master Game Search
```text
avg_elo >= 2650 and (eco startswith "B" or eco startswith "E") and occurrences >= 2 { pin(bishop, knight, king) }
```
