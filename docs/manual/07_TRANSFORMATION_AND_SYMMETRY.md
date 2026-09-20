# 🔄 Chapter 7: Board Transformations & Symmetries

Transformation blocks automatically expand a search query across geometric board symmetries (reflections, rotations) or color inversions.

---

## 📌 Transformation Keywords

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

---

## 💡 Syntax & Usage

Wrap any search expression inside `{ ... }` following the transformation keyword:

```text
flipcolor {
    white "Carlsen" and fork(knight, queen, rook)
}
```
* **Branch 1 (Original)**: White "Carlsen" and White Knight forks Black Queen & Rook.
* **Branch 2 (Color Inverted)**: Black "Carlsen" and Black Knight forks White Queen & Rook.

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
