# ⚖️ Chapter 6: Material Balance & Piece Power

This chapter documents material composition filters, piece point differences, bishop color complexes, and total piece power balance.

---

## 📌 Material & Power Keywords

| Keyword | Description | Syntax / Example |
| :--- | :--- | :--- |
| **`material_diff`** / **`materialdiff`** | Material point difference `(White - Black)` based on standard values (P=1, N=3, B=3, R=5, Q=9) | `material_diff >= 3` |
| **`opposite_bishops`** | Opposite-colored bishops (White has 1 light/dark bishop and Black has 1 opposite bishop) | `opposite_bishops` |
| **`same_colored_bishops`** | Same-colored bishops | `same_colored_bishops` |
| **`white_power`** / **`whitepower`** | Total piece power points for White | `white_power >= 30` |
| **`black_power`** / **`blackpower`** | Total piece power points for Black | `black_power <= 15` |
| **`total_power`** / **`power`** | Combined piece power on the board | `total_power <= 20` |
| **`white_vs_black_power`** | Relative power comparison | `white_power > black_power` |
| **`power_diff`** | Absolute power difference `\|White - Black\|` | `power_diff >= 5` |

---

## 🎯 Verified Examples

### 1. Opposite-Colored Bishop Endgames
```text
opposite_bishops and [Qq] == 0 and [Rr] == 0 and [Nn] == 0
```

### 2. Minor Piece Endgames (Knights vs Bishops)
```text
knights == 2 and bishops == 0 and [Qq] == 0 and [Rr] == 0
```

### 3. Material Sacrifices & Imbalances
```text
# Exchange sacrifice (Rook for minor piece)
rooks == 3 and knights == 3 and bishops == 2

# Significant material advantage
material_diff >= 5
```

### 4. Endgame Power Threshold
```text
# Low total piece power on the board
total_power <= 16 and [Qq] == 0
```
