# 📐 `rank`, `file`, & `time_control` Utilities Plan

> **Design & Engineering Specification Document**  
> **Target**: Future Utility Enhancements  
> **Status**: Concept & Architecture Planning  

---

## 1. Executive Summary

This plan outlines two major syntactic and practical enhancements:
1. **Geometric Square Generators (`rank` & `file`)**: Clean abstractions to generate rows ($1 \dots 8$) and columns ($a \dots h$) without manually typing 8-square arrays.
2. **Speed & Time Control Categorization (`time_control` / `speed`)**: Native header categorization for Bullet, Blitz, Rapid, Classical, and Daily games.

---

## 2. Geometric Generators: `rank` & `file`

### 2.1. `rank` (Horizontal Rows 1..8)
* `rank 7` $\rightarrow$ `[a7, b7, c7, d7, e7, f7, g7, h7]`
* `rank [7, 8]` $\rightarrow$ `[a7..h8]`
* `rank in 1..2` $\rightarrow$ `[a1..h2]`

### 2.2. `file` (Vertical Columns a..h)
* `file e` $\rightarrow$ `[e1, e2, e3, e4, e5, e6, e7, e8]`
* `file [d, e]` $\rightarrow$ `[d1..e8]`
* `file in c..f` $\rightarrow$ `[c1..f8]`

### 2.3. Clean Query Expressions
```text
# Queen on the 7th rank
piece Q on rank 7

# Rook on the e-file
piece R on file e

# Intersect rank and file to identify specific squares
rank 8 & file e  # Evaluates to square 'e8'

# Outpost on center files
outpost knight on file [d, e]
```

---

## 3. Speed & Time Control Categorization

### 3.1. Speed Categories
PGN `TimeControl` header strings (e.g. `"180+2"`, `"300"`, `"600+10"`, `"5400+30"`) are automatically parsed into estimated game seconds:
$$\text{Estimated Seconds} = \text{Base Seconds} + 40 \times \text{Increment}$$

| Category | Estimated Game Duration | Typical Controls |
| :--- | :--- | :--- |
| **`ultra_bullet`** / **`hyper`** | $< 60$s | `30+0`, `45+0`, `30+1` |
| **`bullet`** | $60\text{s} \dots < 180\text{s}$ | `60+0` (1+0), `120+1` (2+1) |
| **`blitz`** | $180\text{s} \dots < 600\text{s}$ | `180+0` (3+0), `180+2` (3+2), `300+0` (5+0), `300+3` (5+3) |
| **`rapid`** | $600\text{s} \dots < 1800\text{s}$ | `600+0` (10+0), `900+10` (15+10) |
| **`classical`** / **`standard`** | $\ge 1800\text{s}$ | `1800+0` (30+0), `5400+30` (90+30) |
| **`correspondence`** / **`daily`** | Per-move days | `1/86400`, `1/259200` |

### 3.2. Query Syntax
```text
speed blitz
time_control rapid
time_control in [bullet, blitz]
speed classical and avg_elo >= 2700
```

---

## 4. AST Representation

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeControlCategory {
    UltraBullet,
    Bullet,
    Blitz,
    Rapid,
    Classical,
    Correspondence,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeaderPredicate {
    // ...
    Speed(TimeControlCategory),
}
```
