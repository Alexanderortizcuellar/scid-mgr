# ♟️ Chapter 2: Piece Identifiers, Placements & Board Patterns

This chapter covers piece specifiers, compact placement syntax, square sets, rectangular/diagonal area ranges, piece counts, and full/wildcard FEN patterns.

---

## 📌 Piece Symbols & Identifiers

In CQLite, piece symbols can specify color, piece role, or color sets:

| Symbol | Meaning | Example |
| :--- | :--- | :--- |
| **`P`**, **`N`**, **`B`**, **`R`**, **`Q`**, **`K`** | **White Pieces** (Pawn, Knight, Bishop, Rook, Queen, King) | `Kd4` (White King on d4) |
| **`p`**, **`n`**, **`b`**, **`r`**, **`q`**, **`k`** | **Black Pieces** (Pawn, Knight, Bishop, Rook, Queen, King) | `qd8` (Black Queen on d8) |
| **`A`** / **`white_pieces`** / **`white`** | **Any White Piece** | `Ae4` (Any White piece on e4) |
| **`a`** / **`black_pieces`** / **`black`** | **Any Black Piece** | `ad4` (Any Black piece on d4) |
| **`_`** / **`empty`** | **Empty Square** | `_e4` (Square e4 is empty) |
| **`any_piece`** / **`occupied`** | **Any Occupied Square** (either color) | `occupied on d4` |

---

## 🎯 Compact Piece Placement Syntax

Compact placements specify a piece and square directly:

```text
Kd4      # White King on d4
qd8      # Black Queen on d8
Bf4      # White Bishop on f4
pe4      # Black Pawn on e4
_d4      # Square d4 is empty
Ae4      # Any White piece on e4
ad4      # Any Black piece on d4
wpd4     # White Pawn on d4 (explicit prefix)
bke8     # Black King on e8 (explicit prefix)
```

You can combine multiple placements seamlessly with `and` or space separation:
```text
Kd4 qd8 Pa5
```

---

## 📐 Square Ranges, Areas & Diagonals

You can query individual squares, sets, rectangular board boxes, ranks, files, or collinear diagonals:

| Syntax | Type | Description | Example |
| :--- | :--- | :--- | :--- |
| **`e4`** | Single Square | Target single square | `Q on e4` |
| **`[c3, d5, e4]`** | Square Set | List of specific squares | `Knight on [c3, d5, f3]` |
| **`a1-h2`** / **`c3-f6`** | Rectangular Area | All squares in the bounding box between two corners | `P on a1-h2 == 0` |
| **`a1..h8`** / **`diag:a1-h8`** | Diagonal Ray | Only the collinear diagonal between corners | `B on a1..h8` |
| **`a1-8`** | Full File Range | Squares on file `a` from rank 1 to 8 | `R on a1-8` |
| **`a-h1`** | Full Rank Range | Squares on rank 1 from file `a` to `h` | `R on a-h1 == 0` |
| **`light`** / **`light_squares`** | Color Complex | All 32 light squares | `B on light` |
| **`dark`** / **`dark_squares`** | Color Complex | All 32 dark squares | `b on dark` |

---

## 🔢 Piece Counts & Group Counts

Count total pieces on the board or within a specific square set:

### 1. Simple Piece Counts
```text
queens == 0                  # Queenless positions
rooks >= 3                   # Positions with 3 or more rooks
knights == 4                 # All 4 knights present on board
white_pawns <= 4             # White has 4 or fewer pawns
black_bishops == 2           # Black has a bishop pair
```

### 2. Bracketed Piece Group Counts
Count occurrences of combined piece sets:
```text
[Qq] == 0                    # Zero queens on the board (both sides)
[Rr] >= 3                    # At least 3 rooks on the board
[BNbn] == 0                  # All minor pieces have been traded off
[Aa] on [e4, d4, e5, d5] >= 2 # At least 2 pieces in the center
```

### 3. Light & Dark Square Bishop Counts
```text
white_light_bishops == 1 and black_dark_bishops == 1
light_bishops == 2 and dark_bishops == 0
```

---

## 🏁 Board States, Turns & Legal Moves

| Keyword | Description | Example |
| :--- | :--- | :--- |
| **`wtm`** / **`turn white`** | White to move | `wtm and check` |
| **`btm`** / **`turn black`** | Black to move | `btm and checkmate` |
| **`check`** | King is in check | `check` |
| **`checkmate`** | Position is checkmate | `checkmate` |
| **`stalemate`** | Position is stalemate | `stalemate` |
| **`legal`** / **`legal count`** | Number of legal moves in position | `legal == 0` or `legal count >= 30` |

---

## 🌐 Full & Wildcard FEN Patterns

Match exact or partial board setups with standard `fen` / `position` keywords:

```text
fen "r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/2N2N2/PPPP1PPP/R1BQK2R w KQkq - 4 4"
```

### Rank Wildcards (`*`, `?`, `A`, `a`):
* `*` for an entire rank matches any rank configuration.
* `*` within a rank matches any piece sequence.
* `A` matches **any White piece**.
* `a` matches **any Black piece**.

```text
fen "*/*/*/*ppA*/*/*/*/*"   # Black pawns and White piece on 4th/5th ranks
fen "*/*/*/*/4k3/*/*/*"      # Black King on e4
```
