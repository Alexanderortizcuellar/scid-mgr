# 🏹 Chapter 3: Move & Path Patterns

This chapter documents move matching, path sequences, the quiet move separator (`--`), capture syntax (`x` and `[x]`), promotions, and path gap quantifiers.

---

## 📌 Move Pattern Grammar & Separators

`scid-mgr` uses a clean separator grammar:
* **`--`**: Separator for quiet moves (source to destination).
* **`x`** / **`[x]`**: Separator for captures (attacker to target/destination).
* **`=`**: Promotion specifier.

---

## 🎯 Move Syntax Guide

| Syntax | Type | Description | Matches |
| :--- | :--- | :--- | :--- |
| **`Nf3`** | SAN Move | Standard SAN Knight move to `f3` | White Knight to `f3` |
| **`h7`** / **`e4`** | SAN Pawn | Bare square pawn move | Pawn advances to `h7` / `e4` |
| **`Ph7`** | Explicit Pawn | White pawn to `h7` | White Pawn moves to `h7` |
| **`ph7`** | Explicit Pawn | Black pawn to `h7` | Black Pawn moves to `h7` |
| **`Ph6--h7`** | Piece & Source | White pawn on `h6` moves to `h7` | `h6` $\rightarrow$ `h7` by White pawn |
| **`P--h7`** | Piece to Square | White pawn moves to `h7` | White pawn advances to `h7` |
| **`Nf3--g5`** | Source Piece & Square | White knight on `f3` moves to `g5` | `Nf3` $\rightarrow$ `g5` |
| **`h4--h5`** | Square to Square | Move from `h4` to `h5` | `h4` $\rightarrow$ `h5` |
| **`N--[e4, d5]`** | Destination Set | Knight moves to `e4` or `d5` | Knight to `e4` or `d5` |
| **`Bc2xh7`** | Capture with Origin | Bishop on `c2` captures on `h7` | `Bc2` captures on `h7` |
| **`Bxh7`** | White Capture | White bishop captures on `h7` | White Bishop captures on `h7` |
| **`bxh7`** | Black Capture | Black bishop captures on `h7` | Black Bishop captures on `h7` |
| **`Bxph7`** | Target Piece Type | White bishop captures Black pawn on `h7` | Bishop captures black pawn on `h7` |
| **`Bx[q,r]h7`** | Multi-Piece Target | Bishop captures Black Queen or Rook on `h7` | Bishop captures Q/R on `h7` |
| **`Pxr`** | Piece-on-Piece | White pawn captures Black rook anywhere | Pawn takes Rook |
| **`pxN`** | Piece-on-Piece | Black pawn captures White knight anywhere | Black pawn takes Knight |
| **`Pe7xd8=Q`** | Capture-Promotion | Pawn on `e7` captures on `d8` $\rightarrow$ Queen | `e7xd8=Q` |
| **`P[x]d8=Q`** | Capture-Promotion | White pawn captures on `d8` $\rightarrow$ Queen | Any pawn capture on `d8` $\rightarrow$ Q |
| **`P[x]a=Q`** | Capture-Promotion | White pawn captures Black piece $\rightarrow$ Queen | Any capture of Black piece $\rightarrow$ Q |
| **`Pxr=Q`** | Target Promotion | White pawn captures Black rook $\rightarrow$ Queen | Pawn takes Rook $\rightarrow$ Q |
| **`P--h8=Q`** | Promotion to Square | White pawn advances to `h8` $\rightarrow$ Queen | `h8=Q` |
| **`P--=Q`** | Pawn Promotion | White pawn promotes to Queen anywhere | Any White pawn promotion to Q |
| **`A--=Q`** | Color Promotion | Any White piece/pawn promotes to Queen | White promotion to Q |
| **`--=R`** | Underpromotion | Promotes to Rook (any side) | `=R` underpromotion |
| **`--="RBN"`** | Underpromotion Set | Promotes to Rook, Bishop, or Knight | Underpromotion to R, B, or N |

---

## 🛣️ Path Sequences & Gaps (`path [...]` / `line [...]`)

Paths match move sequences across a game's timeline:

### 1. Gap Quantifiers

| Token | Quantifier | Description | Example |
| :--- | :--- | :--- | :--- |
| **`...`** / **`--*`** / **`*`** | `*` (0 or more) | Matches 0 or more intermediate plies | `path [e4 ... d5 ... Pe7xd8=Q]` |
| **`--+`** / **`+`** | `+` (1 or more) | Requires at least 1 intermediate ply | `path [e4 --+ d5]` |
| **`--{min, max}`** | Explicit Range | Allowed intermediate plies between moves | `path [e4 --{0, 2} e5]` |
| **`--{n}`** | Exact Gap | Exactly `n` intermediate plies | `path [e4 --{1} d6]` |

---

## 🎯 Verified Examples

### 1. Famous Greek Gift Sacrifice Sequence
```text
path [Bxh7 kxh7 ... Ng5+ kg8 ... Qh5]
```

### 2. Underpromotion Query
```text
path [P--="RBN"]
```

### 3. Move Disambiguation & Pawn Advancement
```text
path [Ph6--h7 ... Ph7--h8=Q]
```

### 4. Anchoring Move Paths to Specific Positions
Search for a positional theme and the exact tactical move played from that position:
```text
Bc4 bb6 Pa5 and path [bxf2+ Kxf2]
```
