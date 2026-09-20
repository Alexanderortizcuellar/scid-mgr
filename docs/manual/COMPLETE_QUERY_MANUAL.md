# 📖 SCID-MGR Complete Query Language (CQLite) Reference Manual

> **SCID-MGR Search Engine Specification, Syntax Guide & Reference Manual**  
> *Compatible with both raw PGN files and SCID binary database formats (`.si4`/`.si5`).*

---

# Table of Contents
1. [Chapter 1: Header Filters & Metadata](#-chapter-1-header-filters--metadata)
2. [Chapter 2: Piece Identifiers, Placements & Board Patterns](#-chapter-2-piece-identifiers-placements--board-patterns)
3. [Chapter 3: Move & Path Patterns](#-chapter-3-move--path-patterns)
4. [Chapter 4: Bitboard Pawn Structure Analyzer](#-chapter-4-bitboard-pawn-structure-analyzer)
5. [Chapter 5: Tactical & Geometric Motifs](#-chapter-5-tactical--geometric-motifs)
6. [Chapter 6: Material Balance & Piece Power](#-chapter-6-material-balance--piece-power)
7. [Chapter 7: Board Transformations & Symmetries](#-chapter-7-board-transformations--symmetries)
8. [Chapter 8: Boolean Logic, Timeline & Annotations](#-chapter-8-boolean-logic-timeline--annotations)

---

# 🏷️ Chapter 1: Header Filters & Metadata

Header filters query game metadata, player names, tournament information, ratings, and custom PGN/SCID tags.

## 📌 Keywords & Supported Operators

| Keyword | Description | Supported Operators | Example |
| :--- | :--- | :--- | :--- |
| **`player`** | Matches either White or Black player | `==`, `!=`, `contains`, `has`, `startswith`, `endswith`, `~` (regex), `:` | `player "Kasparov"` |
| **`white`** | White player name | `==`, `!=`, `contains`, `has`, `startswith`, `endswith`, `~`, `:` | `white "Carlsen, Magnus"` |
| **`black`** | Black player name | `==`, `!=`, `contains`, `has`, `startswith`, `endswith`, `~`, `:` | `black "Nakamura"` |
| **`white_elo`** / **`whiteelo`** | White's Elo rating | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `white_elo >= 2750` |
| **`black_elo`** / **`blackelo`** | Black's Elo rating | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `black_elo >= 2700` |
| **`elo`** / **`any_elo`** / **`anyelo`** | Either player's Elo rating | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `elo >= 2800` |
| **`avg_elo`** / **`avgelo`** | Average Elo of both players | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `avg_elo >= 2650` |
| **`elo_diff`** / **`elodiff`** | Absolute Elo difference `\|White - Black\|` | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `elo_diff >= 200` |
| **`raw_elo_diff`** | Signed Elo difference `(White - Black)` | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `raw_elo_diff >= 100` |
| **`result`** | Game outcome (`"1-0"`, `"0-1"`, `"1/2-1/2"`, `"*"`) | `==`, `!=`, `:` | `result "1-0"` |
| **`eco`** | ECO opening code prefix or pattern | `==`, `!=`, `startswith`, `has`, `~`, `:` | `eco "B80"` or `eco startswith "E"` |
| **`date`** | Date or year range (`YYYY`, `YYYY.MM`, `YYYY.MM.DD`) | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `date >= "2015"` |
| **`event`** | Tournament / event name | `==`, `!=`, `contains`, `has`, `startswith`, `~`, `:` | `event "World Championship"` |
| **`site`** | Venue / location | `==`, `!=`, `contains`, `has`, `startswith`, `~`, `:` | `site "Wijk aan Zee"` |
| **`round`** | Round number/identifier | `==`, `!=`, `contains`, `:` | `round "1"` |
| **`tag`** / **`header`** / **`custom`** | Arbitrary standard or custom PGN / SCID tag | `==`, `!=`, `contains`, `has`, `~`, `:` | `tag "Annotator" contains "Nunn"` |

## 💡 String & Comparison Operators

* **`==`** or **`:`**: Exact equality (case-insensitive by default) $\rightarrow$ `white == "Kasparov"`
* **`!=`**: Not equal to $\rightarrow$ `result != "1/2-1/2"`
* **`contains`** or **`has`**: Substring search $\rightarrow$ `event contains "Candidates"`
* **`startswith`**: Prefix match $\rightarrow$ `eco startswith "B"`
* **`endswith`**: Suffix match $\rightarrow$ `white endswith "ov"`
* **`~`** or **`regex(...)`**: Regular expression match $\rightarrow$ `player ~ "(?i)alexander.*"`

---

# ♟️ Chapter 2: Piece Identifiers, Placements & Board Patterns

This chapter covers piece specifiers, compact placement syntax, square sets, rectangular/diagonal area ranges, piece counts, and full/wildcard FEN patterns.

## 📌 Piece Symbols

* **`P`**, **`N`**, **`B`**, **`R`**, **`Q`**, **`K`**: **White Pieces**
* **`p`**, **`n`**, **`b`**, **`r`**, **`q`**, **`k`**: **Black Pieces**
* **`A`** / **`white_pieces`** / **`white`**: **Any White Piece**
* **`a`** / **`black_pieces`** / **`black`**: **Any Black Piece**
* **`_`** / **`empty`**: **Empty Square**
* **`any_piece`** / **`occupied`**: **Any Occupied Square**

## 🎯 Compact Piece Placement

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

## 📐 Square Ranges, Areas & Diagonals

* **`e4`**: Single square $\rightarrow$ `Q on e4`
* **`[c3, d5, e4]`**: Square set $\rightarrow$ `Knight on [c3, d5, f3]`
* **`a1-h2`** / **`c3-f6`**: Rectangular bounding box area $\rightarrow$ `P on a1-h2 == 0`
* **`a1..h8`** / **`diag:a1-h8`**: Collinear diagonal ray $\rightarrow$ `B on a1..h8`
* **`a1-8`**: Full file range $\rightarrow$ `R on a1-8`
* **`a-h1`**: Full rank range $\rightarrow$ `R on a-h1 == 0`
* **`light`** / **`light_squares`**: All 32 light squares $\rightarrow$ `B on light`
* **`dark`** / **`dark_squares`**: All 32 dark squares $\rightarrow$ `b on dark`

## 🔢 Piece Counts

```text
queens == 0                  # Queenless positions
[Qq] == 0                    # Zero queens on board
rooks >= 3                   # 3 or more rooks
white_pawns <= 4             # 4 or fewer White pawns
white_light_bishops == 1 and black_dark_bishops == 1
```

## 🏁 Board States & Turns

* **`wtm`** / **`turn white`**: White to move
* **`btm`** / **`turn black`**: Black to move
* **`check`**: King in check
* **`checkmate`**: Position is checkmate
* **`stalemate`**: Position is stalemate
* **`legal`** / **`legal count`**: Number of legal moves $\rightarrow$ `legal count >= 30`

---

# 🏹 Chapter 3: Move & Path Patterns

This chapter documents move matching, path sequences, the quiet move separator (`--`), capture syntax (`x` and `[x]`), promotions, and path gap quantifiers.

## 📌 Move Grammar & Separators

* **`--`**: Separator for quiet moves (source to destination).
* **`x`** / **`[x]`**: Separator for captures (attacker to target/destination).
* **`=`**: Promotion specifier.

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

## 🛣️ Path Sequences & Gaps (`path [...]`)

* **`...`** / **`--*`** / **`*`**: 0 or more intermediate moves $\rightarrow$ `path [e4 ... d5 ... Pe7xd8=Q]`
* **`--+`** / **`+`**: 1 or more intermediate moves $\rightarrow$ `path [e4 --+ d5]`
* **`--{min, max}`**: Explicit ply gap range $\rightarrow$ `path [e4 --{0, 2} e5]`
* **`--{n}`**: Exact ply gap $\rightarrow$ `path [e4 --{1} d6]`

---

# 🧱 Chapter 4: Bitboard Pawn Structure Analyzer

The pawn structure evaluator uses bitboard acceleration to analyze passed, isolated, doubled, backward pawns, and pawn islands.

## 📌 Keywords & Predicates

| Keyword | Description | Syntax |
| :--- | :--- | :--- |
| **`passed_pawns`** / **`passed`** | Passed pawns count for a color | `passed_pawns [white/black] [op] [count]` |
| **`isolated_pawns`** / **`isolated`** | Isolated pawns count for a color | `isolated_pawns [white/black] [op] [count]` |
| **`doubled_pawns`** / **`doubled`** | Doubled pawns count (pawns beyond 1st on a file) | `doubled_pawns [white/black] [op] [count]` |
| **`backward_pawns`** / **`backward`** | Backward pawns count | `backward_pawns [white/black] [op] [count]` |
| **`pawn_islands`** / **`islands`** | Number of pawn islands (contiguous file groups) | `pawn_islands [white/black] [op] [count]` |

## 🔍 Examples

```text
passed_pawns white >= 2 and passed_pawns black == 0
isolated_pawns white == 1 and [Qq] == 0
doubled_pawns black >= 1 and isolated black >= 1
pawn_islands white <= 2 and pawn_islands black >= 3
```

---

# ⚔️ Chapter 5: Tactical & Geometric Motifs

This chapter covers geometric tactical predicates: Pins, Forks, Skewers, Trapped pieces, Outposts, Attacks, and Square Distances.

## 📌 Tactical Keywords

| Keyword | Description | Syntax / Example |
| :--- | :--- | :--- |
| **`pin`** | Absolute or relative pin along a ray | `pin(rook, knight, king)` or `pin(bishop, black_knight, black_king)` |
| **`fork`** | Piece simultaneously attacking 2+ targets | `fork(knight, queen, rook)` or `fork(pawn, bishop, knight)` |
| **`skewer`** | Skewer along an attack ray | `skewer(bishop, king, queen)` |
| **`trapped`** | Piece has 0 legal/safe departure moves | `trapped black_bishop` or `trapped black_queen` |
| **`outpost`** | Advanced protected square | `outpost knight on d5` |
| **`attacks(attacker, target)`** | Attack relation between squares/pieces | `attacks(g5, f6)` or `attacks(B, k)` |
| **`distance(sq1, sq2)`** | Chebyshev square distance | `distance(K, k) <= 2` |
| **`is_attacked`** | Square attacked by color | `is_attacked e4 by black` |

---

# ⚖️ Chapter 6: Material Balance & Piece Power

This chapter documents material composition filters, piece point differences, bishop color complexes, and total piece power balance.

## 📌 Material & Power Keywords

| Keyword | Description | Syntax / Example |
| :--- | :--- | :--- |
| **`material_diff`** | Material point difference `(White - Black)` based on P=1, N=3, B=3, R=5, Q=9 | `material_diff >= 3` |
| **`opposite_bishops`** | Opposite-colored bishops on board | `opposite_bishops and [Qq] == 0` |
| **`same_colored_bishops`** | Same-colored bishops | `same_colored_bishops` |
| **`white_power`** | Total piece power points for White | `white_power >= 30` |
| **`black_power`** | Total piece power points for Black | `black_power <= 15` |
| **`total_power`** | Combined piece power on the board | `total_power <= 20` |
| **`white_vs_black_power`** | Relative power comparison | `white_power > black_power` |
| **`power_diff`** | Absolute power difference `\|White - Black\|` | `power_diff >= 5` |

---

# 🔄 Chapter 7: Board Transformations & Symmetries

Transformation blocks automatically expand a search query across geometric board symmetries (reflections, rotations) or color inversions.

## 📌 Transformation Keywords

* **`flipcolor`** / **`invertcolor`**: Color Inversion (White $\leftrightarrow$ Black).
* **`flipvertical`** / **`flip_v`**: Vertical reflection across ranks 4 and 5 ($1 \leftrightarrow 8, 2 \leftrightarrow 7$).
* **`fliphorizontal`** / **`flip_h`**: Horizontal reflection across files d and e ($a \leftrightarrow h, b \leftrightarrow g$).
* **`rotate90`**, **`rotate180`**, **`rotate270`**: Board rotations.
* **`flipmaindiagonal`** / **`flip_diag`**: Main diagonal reflection ($a1-h8$).
* **`flipantidiagonal`** / **`flip_antidiag`**: Anti-diagonal reflection ($a8-h1$).
* **`flip:all`** / **`symm:all`**: Expands into all 8 geometric board symmetries.

```text
flipcolor {
    white "Carlsen" and fork(knight, queen, rook)
}

fliphorizontal {
    Bc4 and Qh5 and attacks(h7, f7)
}
```

## 🎲 Variable Bindings (`piece $var in [...] { ... }`)

```text
piece $minor in [N, B] {
    $minor on d5 and fork($minor, queen, rook)
}
```

---

# ⏱️ Chapter 8: Boolean Logic, Timeline & Annotations

This chapter covers logical combinators (`and`, `or`, `not`), timeline scopes (`ply in ...`, `move_number`, `occurrences`), and move annotation filters (comments and NAGs).

## 📌 Boolean Combinators

* **`and`**: Logical Conjunction $\rightarrow$ `player "Kasparov" and result "1-0"`
* **`or`**: Logical Disjunction $\rightarrow$ `eco "B90" or eco "B92"`
* **`not`**: Logical Negation $\rightarrow$ `not check and legal == 0`
* **`( ... )`**: Grouping Parentheses $\rightarrow$ `(white "Karpov" or white "Kasparov") and date >= "1985"`

## ⏳ Timeline Scopes

* **`ply in min..max { ... }`**: Restrict evaluation to ply range $\rightarrow$ `ply in 1..20 { fork(knight, queen, rook) }`
* **`move_number [op] [num]`**: Match at specific full move numbers $\rightarrow$ `move_number <= 10 and queens == 0`
* **`occurrences min..max { ... }`**: Require $N$ occurrences throughout the game $\rightarrow$ `occurrences >= 3 { check }`

## 💬 Comment & NAG Annotation Filters

* **`comment contains "blunder"`**
* **`comment contains "??"`**
* **`nag $1`** (`!` good move), **`nag $3`** (`!!` brilliant move), **`nag $4`** (`??` blunder).

---

# 🎯 Complete Example Queries

```text
# 1. Greek Gift Sacrifice with high ratings
avg_elo >= 2600 and path [Bxh7 kxh7 ... Ng5+ kg8 ... Qh5]

# 2. Passed Pawn Endgame Conversion
move_number >= 35 and [Qq] == 0 and passed_pawns white >= 1 and passed_pawns black == 0 and result "1-0"

# 3. Double Symmetrical Minor Piece Outpost
flipcolor {
    outpost knight on d5 and rooks == 2
}
```
