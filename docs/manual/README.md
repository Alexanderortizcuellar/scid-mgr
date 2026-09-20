# 📖 SCID-MGR Query Language (CQLite) User Manual

Welcome to the **SCID-MGR Search Engine & Query Language Manual**.

`scid-mgr` features a high-performance, modular query engine inspired by CQL (Chess Query Language) that operates directly across both **PGN databases** and **native SCID binary databases (`.si4`/`.si5`)**.

---

## 📚 Manual Table of Contents

| Chapter | Topic | Description & Key Features |
| :--- | :--- | :--- |
| **[01. Header Filters](./01_HEADER_FILTERS.md)** | Metadata & PGN Tags | Player names, ratings, ECO codes, dates, results, custom tags, regex matching. |
| **[02. Piece Identifiers & Placement](./02_PIECE_IDENTIFIERS_AND_PLACEMENTS.md)** | Pieces, Squares & Boards | Piece symbols (`P`, `p`, `A`, `a`), compact placements (`Kd4`), square sets (`[c3, d5]`), ranges (`a1..h8`), FEN wildcards. |
| **[03. Move & Path Patterns](./03_MOVE_AND_PATH_PATTERNS.md)** | Moves, Captures & Lines | Move separator (`--`), captures (`x`, `[x]`), promotions (`Pe7xd8=Q`, `P--=Q`, `A--=Q`), path gaps (`...`, `--*`, `--{min,max}`). |
| **[04. Pawn Structures](./04_PAWN_STRUCTURES.md)** | Bitboard Pawn Analyzer | Passed pawns, isolated pawns, doubled pawns, backward pawns, and pawn islands. |
| **[05. Tactical & Geometric Motifs](./05_TACTICAL_AND_GEOMETRIC_MOTIFS.md)** | Tactics & Board Geometry | Absolute/relative pins, forks, skewers, trapped pieces, outposts, attack rays, and square distance. |
| **[06. Material & Power Balance](./06_MATERIAL_AND_POWER.md)** | Material & Piece Power | Piece counts, material point difference, opposite-colored bishops, and total/color power balances. |
| **[07. Transformations & Symmetries](./07_TRANSFORMATION_AND_SYMMETRY.md)** | Board Transformations | Color flipping (`flipcolor`), vertical/horizontal/diagonal reflections, 90°/180°/270° rotations, all-symmetries (`flip:all`), variable bindings (`$var`). |
| **[08. Boolean Logic & Timeline Filters](./08_BOOLEAN_LOGIC_AND_TIMELINE.md)** | Compound Logic & Timeline | `and`, `or`, `not`, ply ranges (`ply in 10..30`), occurrence counts (`occurrences >= 2`), comments, and NAGs. |
| **[Complete Unified Manual](./COMPLETE_QUERY_MANUAL.md)** | Single Document | The complete reference manual in one document, ready for PDF or HTML export. |

---

## 🚀 Quick Start Cheat Sheet

### 1. Header Search
```text
player "Kasparov" and white_elo >= 2700 and date >= "2000" and result "1-0"
```

### 2. Piece Placement & FEN Wildcards
```text
Kd4 and qd8 and Pa5 and [Qq] == 0
fen "*/*/*/*ppA*/*/*/*/*"
```

### 3. Move & Sequence (Path) Search
```text
path [e4 ... d5 ... Pe7xd8=Q]
path [Ph6--h7]
path [Bxh7 kxh7]
path [A--=Q]
```

### 4. Pawn Structure & Material Search
```text
passed_pawns white >= 1 and isolated black >= 1 and doubled_pawns == 0
opposite_bishops and material_diff == 0
```

### 5. Tactical Motif & Transformation Search
```text
pin(bishop, knight, king)
flipcolor { white "Carlsen" and fork(knight, queen, rook) }
```
