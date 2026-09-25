# 🔄 Chapter 7: Board Transformations & Symmetries

Transformation blocks automatically expand a search query across geometric board symmetries (reflections, rotations), spatial shifts (horizontal, vertical, 2D translation), or color inversions.

---

## 📌 Transformation & Shift Keywords

### 1. Geometric Symmetries & Reflections

| Keyword | Description | Transformation Applied |
| :--- | :--- | :--- |
| **`flipcolor`** / **`invertcolor`** | Inverts White $\leftrightarrow$ Black (pieces, colors, headers, turns) | Color Inversion |
| **`flipvertical`** / **`flip_v`** | Vertical reflection across ranks 4 and 5 | Ranks: $1 \leftrightarrow 8, 2 \leftrightarrow 7, \dots$ |
| **`fliphorizontal`** / **`flip_h`** | Horizontal reflection across files d and e | Files: $a \leftrightarrow h, b \leftrightarrow g, \dots$ |
| **`rotate90`** | 90° clockwise board rotation | $(f, r) \rightarrow (r, 7-f)$ |
| **`rotate180`** | 180° board rotation | $(f, r) \rightarrow (7-f, 7-r)$ |
| **`rotate270`** | 270° clockwise board rotation | $(f, r) \rightarrow (7-r, f)$ |
| **`flipmaindiagonal`** / **`flip_diag`** | Reflection along the main diagonal $a1-h8$ | $(f, r) \rightarrow (r, f)$ |
| **`flipantidiagonal`** / **`flip_antidiag`** | Reflection along the anti-diagonal $a8-h1$ | $(f, r) \rightarrow (7-r, 7-f)$ |
| **`flip:all`** | All 8 geometric board symmetries + color inversion | Expands into 8 geometric symmetry branches |
| **`symm:all`** | Complete geometric symmetry expansion | Evaluates query under all board reflections and rotations |

### 2. Spatial Shift Transformations

Shift transformations match relative piece formations, pawn chains, battery motifs, or tactical geometries anywhere across files and/or ranks without being tied to fixed board coordinates.

| Keyword / Syntax | Mode | Translation Range | Description |
| :--- | :--- | :--- | :--- |
| **`shifthorizontal`**<br>`shift_horizontal`<br>`shift_h`<br>`shift:horizontal` | Horizontal | $\Delta f \in [-7, 7], \Delta r = 0$ | Shifts the pattern left/right across files along the same rank. |
| **`shiftvertical`**<br>`shift_vertical`<br>`shift_v`<br>`shift:vertical` | Vertical | $\Delta f = 0, \Delta r \in [-7, 7]$ | Shifts the pattern up/down across ranks along the same file. |
| **`shift`**<br>`shiftall`<br>`shift_all`<br>`shift:all` | 2D (All) | $\Delta f \in [-7, 7], \Delta r \in [-7, 7]$ | Shifts the pattern anywhere across both files and ranks. |

---

## ⚙️ How Shift Transformations Work Internally

1. **Translation Vector Exploration**: The engine generates offset candidate pairs $(\Delta f, \Delta r)$ according to the chosen mode:
   - **Horizontal**: $( \Delta f, 0 )$ for $\Delta f \in -7..=7$ (15 offsets).
   - **Vertical**: $( 0, \Delta r )$ for $\Delta r \in -7..=7$ (15 offsets).
   - **All / 2D**: $( \Delta f, \Delta r )$ for $\Delta f, \Delta r \in -7..=7$ (225 offsets).
2. **Recursive Query Translation**: For each $(\Delta f, \Delta r)$ offset, all inner query components (piece placements, moves, square sets, tactical predicates, FEN placements) are shifted.
3. **Boundary Protection**: If any required square or piece falls outside the $8 \times 8$ board ($0 \le file < 8$ and $0 \le rank < 8$), that offset is automatically deemed invalid and skipped.
4. **Existential Match**: If the shifted query matches the position/game for **any** valid offset, the shift block evaluates to `true` (an existential OR over all valid board shifts).

---

## 💡 Syntax & Usage

Wrap any search expression inside `{ ... }` (or `( ... )`) following the transformation or shift keyword:

```text
flipcolor {
    white "Carlsen" and fork(knight, queen, rook)
}
```
* **Branch 1 (Original)**: White "Carlsen" and White Knight forks Black Queen & Rook.
* **Branch 2 (Color Inverted)**: Black "Carlsen" and Black Knight forks White Queen & Rook.

```text
shifthorizontal {
    piece P on d4 and piece P on e4
}
```
* Matches adjacent connected White pawns on the 4th rank anywhere on the board (`a4+b4`, `b4+c4`, `c4+d4`, `d4+e4`, `e4+f4`, `f4+g4`, `g4+h4`).

---

## 🎲 Variable Bindings (`piece $var in [...] { ... }`)

Bind variables to iterate over piece types or square domains dynamically:

```text
# Search for positions where any minor piece ($minor) is on d5 and forks two pieces
piece $minor in [N, B] {
    $minor on d5 and fork($minor, queen, rook)
}
```

---

## 🎯 Verified Examples

### 1. Symmetrical Kingside / Queenside Attack
```text
fliphorizontal {
    Bc4 and Qh5 and attacks(h7, f7)
}
```

### 2. Universal Endgame Theme (Either Color)
```text
flipcolor {
    passed_pawns white >= 1 and [Qq] == 0 and [Rr] == 0 and result "1-0"
}
```

### 3. All Board Reflections for a Tactical Pattern
```text
flip:all {
    fen "8/8/8/8/8/8/4P3/8 w - - 0 1"
}
```

### 4. Horizontal Shift: Connected Center/Flank Pawns
```text
shifthorizontal {
    piece P on d4 and piece P on e4
}
```
* Matches a duo of adjacent White pawns on the 4th rank on any adjacent files.

### 5. Vertical Shift: File Alignment / Battery
```text
shiftvertical {
    piece R on f1 and piece K on f2
}
```
* Matches a King directly in front of a friendly Rook along any rank segment of the f-file.

### 6. 2D All-Board Shift: Queen & Bishop Battery
```text
shift {
    piece Q on f7 and piece B on c4
}
```
* Matches a Queen placed 3 files to the right and 3 ranks above a Bishop anywhere on the board (e.g. `Qf7` + `Bc4`, `Qe6` + `Bb3`, `Qd5` + `Ba2`).

### 7. Shift Combined with Tactical Predicates
```text
shift:horizontal {
    pin(bishop, knight, king)
}
```
* Detects absolute or relative bishop-on-knight pins across all files.

### 8. Direct Coordinate Offset Function: `offset(target, dx, dy)`
As an intuitive alternative to chaining directions (e.g. `up 1 right 2 ...`), you can use the Cartesian `offset(target, dx, dy)` function:
* **`dx`** = File offset ($\Delta x$): positive is Right / East ($\rightarrow$), negative is Left / West ($\leftarrow$).
* **`dy`** = Rank offset ($\Delta y$): positive is Up / North ($\uparrow$), negative is Down / South ($\downarrow$).

```cql
offset(e3, 2, 1)                      # Resolves statically to g4 (File E + 2 = G, Rank 3 + 1 = 4)
offset(N, 2, 1) & e4                  # Knight move offset: square 2 right and 1 up from any White Knight
offset(k, -1, 0) & _                  # Asserts that 1 square left of the Black King is empty
offset([d4, e5], 0, -2)               # Shifts squares down 2 ranks (to d2 and e3)
```

### 9. Chained Symmetries & Mating Matrix (`flipcolor rotate90 { ... }`)
```cql
mate
flipcolor rotate90 {
    northwest 2 Q & up 1 k & R
    right 1 k & _
}
```
* Chains `flipcolor` (both player perspectives) with `rotate90` (all 4 board rotations: 0°, 90°, 180°, 270°).
* Accurately tests all 8 orientations of this Queen + Rook mating net.

