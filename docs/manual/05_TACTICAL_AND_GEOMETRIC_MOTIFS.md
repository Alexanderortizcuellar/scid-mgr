# ⚔️ Chapter 5: Tactical & Geometric Motifs

This chapter covers geometric tactical predicates: Pins, Forks, Skewers, Trapped pieces, Outposts, Attacks, and Square Distances.

---

## 📌 Tactical Keywords & Functions

| Keyword / Function | Description | Syntax |
| :--- | :--- | :--- |
| **`pin`** | Absolute or relative pin along a ray | `pin([pinner], [pinned], [target])` |
| **`fork`** | Piece simultaneously attacking 2+ targets | `fork([attacker], [target1], [target2])` |
| **`skewer`** | Skewer along an attack ray | `skewer([attacker], [front_target], [rear_target])` |
| **`trapped`** | Piece has 0 legal/safe departure moves | `trapped [piece]` |
| **`outpost`** | Advanced protected square | `outpost [piece] on [square]` |
| **`attacks(attacker, target)`** / **`attacks[...]`** | Target squares attacked by attacker set | `attacks(R, k)` or `attacks[white, [d1..d8]]` |
| **`attackers(attacker, target)`** / **`attackers[...]`** | Attacking piece squares that target squares | `attackers(white, e5)` or `attackers[white, e5]` |
| **`ray(direction, origin)`** | Squares along a directional ray | `ray(up, d4)` or `ray(diagonal, [c1, f1])` |
| **`between(from, to)`** / **`between[...]`** | Squares strictly between two sets | `between(K, R)` or `between[K, R]` |
| **`offset(target, dx, dy)`** / **`offset[...]`** | Cartesian $(\Delta x, \Delta y)$ spatial offset | `offset(e3, 2, 1)` or `offset[N, 2, 1]` |
| **`distance(sq1, sq2)`** | Chebyshev distance between two squares `max(|dx|, |dy|)` | `distance(e1, e8) >= 5` |
| **`is_attacked`** | Square attacked by color | `is_attacked e4 by black` |

> 💡 **Parentheses `(...)` & Brackets `[...]` Flexibility**: All geometric functions (`attacks`, `attackers`, `between`, `offset`) accept both parentheses `(...)` and brackets `[...]` interchangeably. Square set algebra operators (`&`, `|`, `-`, `\`, `~`) can be freely chained with these functions.

---

## 🎯 Detail & Examples

### 1. Pin Motif (`pin`)
A pinner (Bishop, Rook, Queen) attacks a target through an intermediary pinned piece:
```text
// Pin White Bishop against Black King using White Rook
pin(rook, knight, king)

// Specific pieces and colors
pin(bishop, black_knight, black_king)
pin(white_bishop, black_queen, black_king)
```

### 2. Fork Motif (`fork`)
A piece simultaneously attacks multiple enemy targets:
```text
// Knight forks Queen and Rook
fork(knight, queen, rook)

// Pawn forks Bishop and Knight
fork(pawn, bishop, knight)

// Specific squares
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

### 6. Square Set Geometric Functions & Set Comparisons

#### Attack Target Evaluation (`attacks`)
Returns the subset of `target` squares attacked by any piece in `attacker`:
```text
attacks(R, k) >= 2                                      // King subjected to double rook check/attack
attacks(white_pieces, [d1..d8]) > attacks(black_pieces, [d1..d8]) // Spatial file control dominance
attacks(n, K) & ~occupied                               // Knight attacks on king while targeting escape squares
attacks(A, a) & attacks(a, a)                           // Defended pieces: squares occupied by Black attacked by White AND defended by Black
attacks(K, r) & a1-b3                                   // Squares where King attacks Rook within bounding box a1-b3
attacks[K, r] & a1-b3                                   // Same expression using bracket syntax
attacks(white_pieces, [d1..d8]) & ~occupied             // Unoccupied squares on d-file controlled by White
btm and mate and not attacks(k, [A, _])                 // Canonical Smothered Mate: Black is checkmated and King has zero attacks on White pieces (A) or empty squares (_)
```

> **Smothered Mate (`not attacks(k, [A, _])`) Mechanics**:
> `[A, _]` represents all squares occupied by enemy (White) pieces `A` or empty squares `_`. If `not attacks(k, [A, _])` holds, every square adjacent to the king is occupied by a friendly (Black) piece `a`. In checkmate, this means the king is completely smothered by its own army!

#### Attack Origin Evaluation (`attackers`)
Returns the subset of `attacker` pieces that attack any square in `target`:
```text
attackers(white_pieces, e5) > attackers(black_pieces, e5) // Outnumbering defenders on e5
attackers(N, d5) >= 2                                    // Multiple knights attacking d5
```

#### Directional Rays, Vectors & Lines (`ray`)
Rays and directional filters use full English words for directions (abbreviations like `ne` are avoided to prevent ambiguity with piece/square tokens like `ne5`):

| Direction Keyword | Meaning / Vector | Example |
| :--- | :--- | :--- |
| **`up`** | North (increasing rank $\uparrow$) | `ray(up, e4)` |
| **`down`** | South (decreasing rank $\downarrow$) | `ray(down, d5)` |
| **`left`** | West (decreasing file $\leftarrow$) | `ray(left, e4)` |
| **`right`** | East (increasing file $\rightarrow$) | `ray(right, d4)` |
| **`northeast`** | Up-Right diagonal ($\nearrow$) | `ray(northeast, c1)` |
| **`northwest`** | Up-Left diagonal ($\nwarrow$) | `ray(northwest, f1)` |
| **`southeast`** | Down-Right diagonal ($\searrow$) | `ray(southeast, c8)` |
| **`southwest`** | Down-Left diagonal ($\swarrow$) | `ray(southwest, f8)` |
| **`diagonal`** | All 4 diagonal directions ($\nearrow \nwarrow \searrow \swarrow$) | `ray(diagonal, [c1, f1]) & [d4, e5]` |
| **`orthogonal`** | All 4 orthogonal rank/file directions ($\uparrow \downarrow \leftarrow \rightarrow$) | `ray(orthogonal, d4)` |
| **`vertical`** | Files ($\uparrow \downarrow$) | `ray(vertical, e1)` |
| **`horizontal`** | Ranks ($\leftarrow \rightarrow$) | `ray(horizontal, a4)` |
| **`anydirection`** | All 8 directions | `ray(anydirection, e4)` |

```text
ray(diagonal, [c1, f1]) & [d4, e5]                       // Diagonal ray intersecting central squares
between(k, q) & occupied == 0                            // Clear open line between king and queen
distance(K, k) <= 2                                      // Kings in close proximity
```

---

### 7. Directional Spatial Shift Translation Operators (`<direction> [dist] <SquareSet>`)

In CQL and CQLi, direction keywords can be used anywhere as **prefix translation (shift) operators** on square sets:

$$\text{direction}\quad[\text{distance}]\quad\text{SquareSet}$$

Every square in `SquareSet` is shifted by the specified distance in the given direction.

#### Syntax & Expressions:
* `northwest 2 Q`: Shifts the White Queen's square by 2 squares North-West ($\nwarrow\nwarrow$).
* `up 1 k`: Shifts the Black King's square by 1 square North ($\uparrow$).
* `right 1 k`: Shifts the Black King's square by 1 square East ($\rightarrow$).
* `down 1..3 R`: Shifts White Rook squares down 1 to 3 ranks.
* `orthogonal 1 [d4, e5]`: Shifts central squares 1 step in all 4 orthogonal directions.

#### Geometric Matrix & Mating Nets:
Combining shifts with set intersection (`&`) and symmetries allows searching for precise geometric arrangements:

```cql
mate
flipcolor rotate90 {
    northwest 2 Q & up 1 k & R
    right 1 k & _
}
```

* **`northwest 2 Q & up 1 k & R`**: Asserts that a square exists which is simultaneously 2 squares NW of the Queen, 1 square North of the enemy King, and occupied by a White Rook (contact Rook check supported diagonally by Queen 2 squares behind).
* **`right 1 k & _`**: Asserts that the adjacent flight square 1 step right of the King is empty (`_`).
* **`flipcolor rotate90`**: Evaluates this mating net across all 4 rotational angles (0°, 90°, 180°, 270°) and both White/Black mating perspectives!

---

### 8. Hypothetical Board Mutation Sandbox (`what_if(...) { ... }`)

The `what_if` filter allows you to perform **speculative evaluation** by applying arbitrary hypothetical mutations to a copy of the current board state and evaluating test filters inside a sandbox.

#### Syntax:
```cql
what_if(<mutations>) {
    <test filters>
}
// or bracket syntax
what_if[<mutations>] {
    <test filters>
}
```

#### Supported Mutations:
1. **`pass` / `null_move`**: Switches the side to move without moving any piece (threat / null-move analysis).
2. **`remove <squares>` / `without <piece> on <square>`**: Clears squares (e.g. `remove f6`, `remove [f6, g7]`, `without knight on f6`) to test defender removal or unblocking.
3. **`move <from> to <to>` / `transfer <from> -> <to>`**: Moves a piece from one square to another hypothetically (e.g. `move b1 to d5`).
4. **`add <piece> on <square>` / `place <piece> on <square>`**: Injects a piece onto an empty square (e.g. `add Q on e5`).
5. **`swap <sq1>, <sq2>`**: Swaps the contents of two squares (e.g. `swap g1, f1`).
6. **`swap_color <square>` / `invert_color <square>`**: Inverts the piece color on a square (e.g. `swap_color c4`).
7. **`turn white` / `turn black` / `wtm` / `btm`**: Explicitly sets side to move.
8. **`[<moves>]`**: Plays an arbitrary sequence of moves before evaluating the inner block (e.g. `[e4 e5 Qh5]`).

#### Examples:
```cql
// 1. Threat Detection (If Black passes, does White have immediate mate?):
what_if(pass) { play { mate } }

// 2. Deflection / Removing the Defender (If Nf6 is removed, can White mate?):
what_if(remove f6) { play { mate } }

// 3. Piece Placement Fantasy:
what_if(add Q on e5) { attacks(Q, e8) }

// 4. Multi-mutation rollout:
what_if(remove f6, turn white) { play { mate } }

// 5. Hypothetical Move Sequence:
what_if([e4 e5 Qh5]) { attacks(Q, f7) }
```


