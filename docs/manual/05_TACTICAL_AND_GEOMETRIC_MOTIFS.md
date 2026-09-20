# ⚔️ Chapter 5: Tactical & Geometric Motifs

This chapter covers geometric tactical predicates: Pins, Forks, Skewers, Trapped pieces, Outposts, Attacks, and Square Distances.

---

## 📌 Tactical Keywords & Functions

| Keyword | Description | Syntax |
| :--- | :--- | :--- |
| **`pin`** | Absolute or relative pin along a ray | `pin([pinner], [pinned], [target])` |
| **`fork`** | Piece simultaneously attacking 2+ targets | `fork([attacker], [target1], [target2])` |
| **`skewer`** | Skewer along an attack ray | `skewer([attacker], [front_target], [rear_target])` |
| **`trapped`** | Piece has 0 legal/safe departure moves | `trapped [piece]` |
| **`outpost`** | Advanced protected square | `outpost [piece] on [square]` |
| **`attacks(attacker, target)`** | Square/Piece attacks another square/piece | `attacks(e4, d5)` or `attacks(B, k)` |
| **`distance(sq1, sq2)`** | Chebyshev distance between two squares `max(\|dx\|, \|dy\|)` | `distance(e1, e8) >= 5` |
| **`is_attacked`** | Square attacked by color | `is_attacked e4 by black` |

---

## 🎯 Detail & Examples

### 1. Pin Motif (`pin`)
A pinner (Bishop, Rook, Queen) attacks a target through an intermediary pinned piece:
```text
# Pin White Bishop against Black King using White Rook
pin(rook, knight, king)

# Specific pieces and colors
pin(bishop, black_knight, black_king)
pin(white_bishop, black_queen, black_king)
```

### 2. Fork Motif (`fork`)
A piece simultaneously attacks multiple enemy targets:
```text
# Knight forks Queen and Rook
fork(knight, queen, rook)

# Pawn forks Bishop and Knight
fork(pawn, bishop, knight)

# Specific squares
fork(N on c7, e8, a8)
```

### 3. Skewer Motif (`skewer`)
A line piece attacks a more valuable front target (e.g. King/Queen), which when moved exposes a rear target:
```text
skewer(bishop, king, queen)
skewer(rook, king, rook)
```

### 4. Trapped Piece (`trapped`)
Identifies pieces with no safe retreat squares:
```text
trapped black_bishop
trapped black_queen
```

### 5. Outpost (`outpost`)
Identifies pieces stationed on advanced protected squares:
```text
outpost knight on d5
outpost white_knight on e5
```

### 6. Square Distance & Geometric Attacks
```text
# King distance in the endgame
distance(K, k) <= 2

# Direct attack predicate
attacks(g5, f6)
```
