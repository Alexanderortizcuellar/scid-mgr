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
