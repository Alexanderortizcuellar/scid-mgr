# 🧱 Chapter 4: Bitboard Pawn Structure Analyzer

The pawn structure evaluator uses bitboard acceleration to analyze passed, isolated, doubled, backward pawns, and pawn islands.

---

## 📌 Keywords & Predicates

| Keyword | Description | Syntax |
| :--- | :--- | :--- |
| **`passed_pawns`** / **`passed`** | Passed pawns count for a color | `passed_pawns [white/black] [op] [count]` |
| **`isolated_pawns`** / **`isolated`** | Isolated pawns count for a color | `isolated_pawns [white/black] [op] [count]` |
| **`doubled_pawns`** / **`doubled`** | Doubled pawns count (pawns beyond 1st on a file) | `doubled_pawns [white/black] [op] [count]` |
| **`backward_pawns`** / **`backward`** | Backward pawns count | `backward_pawns [white/black] [op] [count]` |
| **`pawn_islands`** / **`islands`** | Number of pawn islands (contiguous file groups) | `pawn_islands [white/black] [op] [count]` |

---

## 🔍 How Each Pawn Motif is Evaluated

### 1. Passed Pawns
A pawn on `(file, rank)` is **passed** if there are:
* **No enemy pawns** in front on the same file.
* **No enemy pawns** in front on the adjacent left or right files.
```text
passed_pawns white >= 2
passed_pawns black == 0
```

### 2. Isolated Pawns
A pawn is **isolated** if there are **no friendly pawns** on either adjacent file (left or right).
```text
isolated_pawns white == 1
isolated black == 0
```

### 3. Doubled Pawns
Calculates `count(pawns_on_file) - 1` for each file. If a file has 2 pawns, that counts as 1 doubled pawn; 3 pawns count as 2.
```text
doubled_pawns white == 0     # Clean pawn structure (no doubled pawns)
doubled_pawns black >= 2     # Compromised pawn structure
```

### 4. Backward Pawns
A pawn is **backward** if:
* It has no friendly pawns behind it or beside it on adjacent files to support it.
* The square directly in front of it is controlled by an enemy pawn.
```text
backward_pawns black >= 1
```

### 5. Pawn Islands
Counts the number of connected file clusters containing friendly pawns. Fewer pawn islands generally signify a sounder endgame structure.
```text
pawn_islands white <= 2 and pawn_islands black >= 3
```

---

## 🎯 Verified Examples

### 1. Outside Passed Pawn Endgame
```text
passed_pawns white >= 1 and passed_pawns black == 0 and [Qq] == 0
```

### 2. Isolated Queen Pawn (IQP) Search
```text
Pd4 and isolated white == 1 and white_pawns >= 5
```

### 3. Crippled Pawn Majority
```text
doubled_pawns black >= 1 and isolated black >= 1
```
