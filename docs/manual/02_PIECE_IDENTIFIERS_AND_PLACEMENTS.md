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
| **`_`** / **`empty`** | **Empty Squares Set** | `_e4` (Square e4 is empty) / `[A, _]` |
| **`.`** / **`all`** / **`all_squares`** | **Universal Set** (All 64 squares) | `. \ d4 == 63` |
| **`any_piece`** / **`occupied`** | **Any Occupied Square** (either color) | `occupied on d4` |

---

## 🎯 Compact Piece Placement Syntax

Compact placements specify a piece and square directly, following standard **FEN casing** (uppercase for White, lowercase for Black):

```text
Kd4      // White King on d4
qd8      // Black Queen on d8
Bf4      // White Bishop on f4
pe4      // Black Pawn on e4
_d4      // Square d4 is empty
Ae4      // Any White piece on e4
ad4      // Any Black piece on d4
```

> 💡 **Piece Casing Standard**: In accordance with FEN conventions, White pieces are always specified with uppercase letters (`K, Q, R, B, N, P, A`) and Black pieces with lowercase letters (`k, q, r, b, n, p, a`).

You can combine multiple placements seamlessly with `and` or space separation, or in bracketed square set unions:
```text
Kd4 qd8 Pa5
[Bd1, _]     // White Bishop on d1 or empty squares
[A, _]       // Any White piece or empty squares
```

### 4. Bracket Juxtaposition Syntax (`[PieceSet][SquareSet]`)
Juxtaposition of a piece group and a square list or range behaves as a concise syntactic sugar for square set intersection (`&`):
```text
[BQ][a1..a8]       // Equivalent to [BQ] & [a1..a8] (White Bishops or Queens on file a)
[Nn][c3, d5, e4]   // Equivalent to [Nn] & [c3, d5, e4] (Knights on specified squares)
[Pp][d4-e5]        // Equivalent to [Pp] & [d4-e5] (Pawns in central rectangle)
```

---

## 📐 Square Ranges, Areas & Diagonals

You can query individual squares, sets, rectangular board boxes, ranks, files, or collinear diagonals:

| Syntax | Type | Description | Example |
| :--- | :--- | :--- | :--- |
| **`e4`** | Single Square | Target single square | `Q on e4` |
| **`[c3, d5, e4]`** | Square Set | List of specific squares | `Knight on [c3, d5, f3]` |
| **`[Bd1, _]`** | Unified Set | Union of piece placements & empty squares | `attacks(k, [A, _])` |
| **`.`** | Universal Set | All 64 squares on the chessboard | `. & light == 32` |
| **`a1-h2`** / **`c3-f6`** | Rectangular Area | All squares in the bounding box between two corners | `P on a1-h2 == 0` |
| **`a1..h8`** / **`diag:a1-h8`** | Diagonal Ray | Only the collinear diagonal between corners | `B on a1..h8` |
| **`a1-8`** | Full File Range | Squares on file `a` from rank 1 to 8 | `R on a1-8` |
| **`a-h1`** | Full Rank Range | Squares on rank 1 from file `a` to `h` | `R on a-h1 == 0` |
| **`light`** / **`light_squares`** | Color Complex | All 32 light squares | `B on light` |
| **`dark`** / **`dark_squares`** | Color Complex | All 32 dark squares | `b on dark` |

---

## ⚡ First-Class Square Set Algebra

CQLite supports full mathematical square set algebra evaluated directly over 64-bit hardware bitboards. Square sets can be intersected, united, subtracted, complemented, and evaluated in both boolean (non-empty) and numeric comparison contexts.

### 1. Set Operators & Precedence

| Operation | Syntax | Bitboard Code | Description / Example |
| :--- | :--- | :--- | :--- |
| **Union** | `A \| B` | `A \| B` | Squares in A, B, or both $\rightarrow$ `(N \| B) [b5, g5] >= 2` |
| **Difference** | `A \ B` or `A - B` | `A & !B` | Squares in A not in B $\rightarrow$ `(occupied \ [e4, d4]) >= 30` |
| **Intersection** | `A & B` or `A B` | `A & B` | Squares common to both $\rightarrow$ `B & [c1, f1]` or `B [c1, f1]` |
| **Complement** | `~A` or `!A` | `!A` | All 64 squares not in A $\rightarrow$ `~occupied >= 32` |

### 2. Dual-Nature Truthiness

* **Boolean Truthiness**: Evaluates whether the resulting square set is **non-empty** (`count > 0`):
  ```text
  B [c4, g5]                 // True if White has a bishop on c4 OR g5
  occupied & [e4, d4, e5, d5]// True if any piece occupies the center
  ```
* **Numeric Comparisons**: Compares the size of the set using standard comparison operators (`==`, `!=`, `<`, `<=`, `>`, `>=`):
  ```text
  B [c4, g5] == 2            // White has bishops on BOTH c4 and g5
  (N | B) [b5, g5] >= 2      // At least 2 knights or bishops on b5 and g5
  (occupied \ [e4, d4]) >= 30// At least 30 pieces excluding central squares
  ~occupied >= 32            // 32 or more empty squares on the board
  ```

---

## 🔢 Piece Counts & Group Counts

Count total pieces on the board or within a specific square set:

### 1. Simple Piece Counts
```text
queens == 0                  // Queenless positions
rooks >= 3                   // Positions with 3 or more rooks
knights == 4                 // All 4 knights present on board
white_pawns <= 4             // White has 4 or fewer pawns
black_bishops == 2           // Black has a bishop pair
```

### 2. Bracketed Piece Group Counts
Count occurrences of combined piece sets:
```text
[Qq] == 0                    // Zero queens on the board (both sides)
[Rr] >= 3                    // At least 3 rooks on the board
[BNbn] == 0                  // All minor pieces have been traded off
[Aa] on [e4, d4, e5, d5] >= 2 // At least 2 pieces in the center
```

### 3. Light & Dark Square Bishop Counts
```text
white_light_bishops == 1 and black_dark_bishops == 1
light_bishops == 2 and dark_bishops == 0
```

### 4. Set-to-Set Comparisons
Compare square sets and piece sets directly against other sets or the empty set (`[]`):
```text
[Aa] == [KkPp]               // Pure King & Pawn endgame (no queens, rooks, bishops, or knights)
[Aa] == []                   // Empty board
[Qq] == []                   // Queenless endgame
[Kk] == [Pp]                 // Equal number of kings and pawns
[Qq] > [Rr]                  // More queens than rooks on the board
[Nn] > [Bb]                  // More knights than bishops (knight advantage)
[Aa] on light == [KkPp]      // All pieces on light squares are kings or pawns
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
fen "*/*/*/*ppA*/*/*/*/*"   // Black pawns and White piece on 4th/5th ranks
fen "*/*/*/*/4k3/*/*/*"      // Black King on e4
```
