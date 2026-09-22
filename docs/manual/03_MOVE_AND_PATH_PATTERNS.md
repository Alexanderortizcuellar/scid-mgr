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
| **`Bg4+`** | Move with Check | White Bishop to `g4` delivering check (`+`) | White Bishop checks on `g4` |
| **`bg4+`** | Move with Check | Black Bishop to `g4` delivering check (`+`) | Black Bishop checks on `g4` |
| **`Rd8#`** | Move with Checkmate | White Rook to `d8` delivering checkmate (`#`) | White Rook checkmates on `d8` |
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
| **`Bxh7+`** | Capture with Check | White bishop captures on `h7` giving check | White Bishop takes on `h7` with check |
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

## 🔍 The Dedicated `move` Keyword Filter

In addition to compact move syntax, `scid-mgr` provides a rich, natural **`move`** keyword filter for querying move origins, targets, pieces, captures, checks, promotions, and legal candidate moves.

### 1. Grammar & Syntax

```text
move [from <origin>] [to <destination>] [piece <piece>] [capture] [check] [mate] [promote <roles>] [legal] [count <op> <n>]
```

### 2. Parameter Clauses & Supported Values

| Clause | Accepts | Description | Examples |
| :--- | :--- | :--- | :--- |
| **`previous`** / **`prev`** | Bare flag | Matches the **incoming move** that led to the current position | `move previous castle`, `check and move previous piece B` |
| **`from`** | Square, Square Set, Piece, or Piece Set | Origin square or moving piece type | `from e2`, `from [e1, e8]`, `from B`, `from [B, N]`, `from A` |
| **`to`** | Square, Square Set, Piece, Piece Set, or Direction | Destination square, target captured piece, or direction vector | `to e4`, `to [c1, g1, c8, g8]`, `to r`, `to [p, r]`, `to a`, `to North 2` |
| **`piece`** | Piece identifier (`Q`, `k`, `P`, etc.) | Explicit piece executing the move | `piece Q`, `piece k`, `piece P` |
| **`capture`** / **`is_capture`** | Bare flag, piece, or piece set | Requires the move to be a capture, optionally filtering the **captured piece** | `move capture`, `move capture p`, `move from R to d5 capture p`, `move capture [q, r]`, `legal capture Q count >= 1` |
| **`check`** / **`is_check`** | Bare flag | Requires the move to deliver check | `move check`, `move from [e1, e8] to [g1, c1] check` |
| **`mate`** / **`checkmate`** | Bare flag | Requires the move to deliver checkmate | `move mate`, `move from Q to f7 mate` |
| **`castle`** / **`castling`** | Bare flag | Requires the move to be castling (`O-O` or `O-O-O`) | `move castle`, `previous castle` |
| **`en_passant`** / **`ep`** | Bare flag | Requires the move to be an en passant capture | `move en_passant`, `move en_passant capture p` |
| **`promote`** / **`promotion`** | Role, Role List, or String | Requires promotion to specific piece(s) | `promote Q`, `promote [Q, R]`, `promote "RBN"` |
| **`legal`** | Flag or with `count` | Asserts candidate legal moves in the position | `move legal count == 0`, `move legal from [e1, e8] to [c1, g1]` |
| **`count`** | `<op> <number>` | Checks number of candidate legal matching moves | `move legal count >= 5` |

### 3. Usage & Query Patterns

* **Captured Piece Filtering (`capture <piece>` or `capture [<pieces>]`):**
  * `move from R to d5 capture p` — Rook moves to `d5` capturing a Black pawn.
  * `move from Q to f7 capture p mate` — Queen captures Black pawn on `f7` delivering checkmate.
  * `move capture [q, r]` — Capture where the victim piece was a Black Queen or Rook.
  * `move capture white_pieces` — Any move capturing a White piece.
  * `move en_passant capture p` — En passant capture of a Black pawn.
  * `legal capture Q count >= 1` — Position where the side to move has 1 or more legal moves capturing a White Queen.
  * `flipcolor { move capture p }` — Symmetrically matches White pawn captures under color flip.

* **Previous / Incoming Move Inspection (`previous` / `prev`):**
  * `move previous castle` (or `previous castle`) — Position immediately after castling occurred.
  * `check and move previous piece B` — Position in check where the giving move was made by a Bishop.
  * `mate and move previous capture p` — Checkmate delivered by capturing a pawn.
* **Piece to Target Piece (Captures):**
  * `move from B to r` — White Bishop captures a Black Rook anywhere.
  * `move from A to a` — Any White piece captures any Black piece.
  * `move from [B, N] to [p, r]` — White minor piece captures Black pawn or rook.
* **Piece / Square to Destination Square:**
  * `move from e2 to e4` — Move from `e2` to `e4`.
  * `move from [e1, e8] to [c1, g1, c8, g8]` — Any castling move (White or Black).
  * `move piece Q to [d8, e8]` — Queen moves to `d8` or `e8`.
* **Candidate Legal Move Counting & Mate-in-1 (`move legal ... count`):**
  * `move legal mate count >= 2` — Positions where the side to move has **2 or more different moves delivering mate-in-1** (overkill / dual solutions).
  * `move legal mate count == 1` — Positions with **exactly 1 unique mate-in-1** move available.
  * `move legal mate count >= 1` (or `move legal mate`) — Any position where the side to move has at least one immediate mating move.
  * `move legal piece Q mate count >= 2` — Multiple Queen moves delivering mate-in-1.
  * `move legal piece N mate count >= 1` — Mate-in-1 delivered by a Knight.
  * `move legal capture mate count >= 1` — Mate-in-1 delivered by a capture.
  * `move legal capture Q count >= 1` — Immediate capture of a Queen available.
  * `move legal en_passant mate count >= 1` — Mate-in-1 delivered by an en passant capture.
  * `move legal castle mate count >= 1` — Mate-in-1 delivered by castling.
  * `move legal count == 0` — Positions with 0 legal moves (checkmate or stalemate).

* **Combining with Board State Conditions:**
  * To assert both the captured piece and destination square specifically:
    * Move filter with `capture`: **`move from Q to f7 capture p`**
    * Compact syntax: **`Qxpf7`** (White Queen captures Black pawn on `f7`)
    * Conjunction: **`pf7 and move from Q to f7 capture`**

> [!NOTE]
> In `move from ... to ...`, the **`from`** and **`to`** arguments accept either **squares** (e.g. `f7`) or **piece specifiers** (e.g. `p`, `r`), not composite placement strings like `pf7`. If you want to specify that the piece on `f7` is a pawn, combine with a placement filter (`pf7 and move from Q to f7 capture`) or use the compact move syntax (`Qxpf7`).

---

## 👑 Check, Checkmate & Cross-Check Patterns Showcase

This section collects real-world query patterns for rare and spectacular tactical checkmating situations:

### 1. Check Answered with Mate (Cross-Check Counter-Mate)
Matches positions where a player is placed in **check**, and immediately delivers **checkmate** on the very next move:
* `check and move mate` — (Recommended) Current position is in check, and outgoing move delivers mate.
* `cqlpath { check mate }` — 1-ply transition from check directly to mate.
* `line --> check --> mate` — Forward CQLi line from check to mate.
* `check and move piece Q mate` — Check answered with a Queen checkmate.
* `check and move piece N mate` — Check answered with a Knight checkmate.
* `check and move capture mate` — Check answered by capturing a piece and delivering checkmate.

### 2. Checkmate Delivered by Special Moves
* `move en_passant mate` — Checkmate delivered by an en passant capture.
* `move castle mate` — Checkmate delivered by castling (`O-O` or `O-O-O`).
* `move promote Q mate` — Checkmate delivered by pawn promotion to Queen.
* `move from [e1, e8] mate` — Checkmate delivered by a King move (discovered checkmate).

### 3. Mate-in-1 Candidate Counting
* `move legal mate count >= 2` — Overkill positions where the player has **2+ different moves** delivering mate-in-1.
* `move legal mate count == 1` — Positions with **exactly 1 unique** mate-in-1 move.
* `move legal en_passant mate count >= 1` — Positions having a legal en passant checkmate available.

### 4. Move Sequences with Timeline Gaps
* `path [O-O-O ... --#]` — Long castle played, followed later in the game by any checkmate.
* `path [Qb8+ ... Rd8#]` — Queen sacrifice check, followed later by Rook checkmate.
* `cqlpath { O-O-O ... mate }` — Long castle followed later by checkmate using `cqlpath`.

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

## ⚡ CQL 6.2 Path & Turnstile Sequences (`cql_path` / `cqlpath` / `turnstile` / `sequence` / `seq`)

`scid-mgr` fully supports CQL 6.2 path filter sequences. Unlike classical move-only paths, **`cqlpath`** sequences allow interleaving **Move Constituents** (which advance the ply timeline) and **Positional Filter Constituents** (which assert board states at the current position without advancing the move index).

### 1. Constituent Types

| Constituent | Syntax | Description | Example |
| :--- | :--- | :--- | :--- |
| **Move Constituent** | Standard SAN or Move Specifier | Advances the position by matching move; supports `+` (check), `#` (mate), and `check`/`mate`/`not check` keywords | `Qb8+`, `Nxb8`, `Rd8#`, `Bxh7 check` |
| **Filter Keyword** | `check`, `mate`, `not check`, `wtm`, `btm` | Asserts current board state without consuming a ply | `cqlpath { e4 not check e5 }` |
| **Filter Block** | `{ <sub-query> }` | Asserts general positional conditions at the current ply | `sequence { e4 { [p] on e7 } e5 }` |
| **Chain** | `( <constituent> ... )` | Groups constituent sequence together | `turnstile { (Bxh7+ kxh7)+ }` |
| **Repetitions** | `*`, `+`, `?`, `{min, max}` | Quantifies consecutive repetitions of constituents or chains | `(e4 e5)+`, `check?`, `{1, 3}` |

### 2. Suffixes and Check/Mate Matching
* **`+` / `check`**: Matches moves giving check (e.g. `Qb8+`, `Bxh7 check`).
* **`#` / `mate`**: Matches moves delivering checkmate (e.g. `Rd8#`, `Qf7 mate`).
* **`not check`**: Asserts that a move does not give check (e.g. `e4 not check`).

### 3. Examples

#### Morphy's Opera Game Checkmate Finish
```text
cqlpath { Qb8+ Nxb8 Rd8# }
```

#### Repeating Sacrifice Chain
```text
turnstile { (Bxh7+ kxh7)+ }
```

#### Interleaved Positional Condition
```text
sequence { e4 { [p] on e7 } e5 { [p] on e5 } }
```

---

## 📐 CQLi `line` Filter (`line -->` / `line <--` / `cqlline`)

In modern CQLi, the **`line`** filter searches for sequential transitions between positions along forward (`-->`) or backward (`<--`) arrows.

### 1. Canonical Syntax
```text
line [range_min range_max] [parameters] {--> | <--} constituent1 [{--> | <--} constituent2 ...]
```

### 2. Modifiers & Parameters

| Modifier | Description | Example |
| :--- | :--- | :--- |
| **`[min max]` / `[count]`** | Restricts matched sequence length (number of plies/steps) | `line 5 100 --> check+` |
| **`-->`** | Forward traversal along child plies | `line --> check --> move previous capture --> mate` |
| **`<--`** | Backward look-behind traversal to parent plies | `mate and line lastposition <-- check* <-- Queen` |
| **`firstmatch`** | Terminates search immediately after the first valid match | `line firstmatch --> (e4 e5)` |
| **`lastposition`** | State anchor: reports/anchors subsequent filters from sequence tail | `line lastposition <-- check* and mate` |
| **`nestban`** | Deduplication: prevents already-matched plies from starting new matches | `line 5 100 nestban --> check+` |
| **`singlecolor`** | Color scoping: only considers positions with the same side-to-move | `line singlecolor --> check+` |

### 3. Expository Examples

#### Consecutive Check Streak (5 to 100 checks)
```text
line 5 100 nestban --> check+
```

#### Check Resolved by Capture Followed by Mate
```text
line --> check --> move previous capture --> mate
```

#### Checkmate with Look-Behind Queen Checks
```text
mate and line lastposition <-- check* <-- Queen
```

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

