# 🔁 `echo` & `find` Filter Specification Plan

> **Design & Architecture Specification Document**  
> **Target**: Future Language Milestone  
> **Status**: Concept & Architecture Planning  

---

## 1. Executive Summary

This plan specifies the implementation of two prominent CQL features:
1. **`echo`**: Detects recurring tactical themes, motif repetitions, and symmetrical echo patterns within the same game.
2. **`find`**: Scoped pattern locator, occurrence matcher, and sub-sequence exporter.

---

## 2. The `echo` Filter (Theme Repetition & Symmetries)

### 2.1. Definition
An **Echo** occurs when a specific motif (e.g. pin, fork, sacrifice, checkmate pattern) appears at least twice in the same game across different positions $ply_a < ply_b$.

### 2.2. Types of Echoes
1. **Direct Echo (Same Motif Repeated)**:
   The identical pattern occurs at two different points in the game.
   ```text
   # Two separate knight forks in the same game
   echo { fork(knight, queen, rook) }

   # Two separate rook sacrifices
   echo { move capture R and parent { attacks(R, k) } }
   ```

2. **Symmetrical / Color-Inverted Echo**:
   White executes a theme on one wing, and later Black or White executes the same theme symmetrically (or on the other color).
   ```text
   # Symmetrical Greek Gift or Bishop sacrifice
   echo flipcolor { attacks(B, [h7, h2]) }

   # Symmetrical king attacks
   echo fliphorizontal { piece Q on [h5, a5] }
   ```

3. **Multi-Ply Distance Constraints**:
   ```text
   # The echo must occur at least 10 plies apart:
   echo(min_distance=10) { checkmate }
   ```

---

## 3. The `find` Filter

### 3.1. Definition & Usage
`find` evaluates a sub-query across games with explicit occurrence bounds or position export semantics.

```text
# Require finding between 2 and 5 occurrences of a motif:
find count in 2..5 { fork(knight, queen, rook) }

# Find games with at least 3 checks:
find count >= 3 { check }
```

---

## 4. AST Representation

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum SearchQuery {
    // ...
    Echo {
        query: Box<SearchQuery>,
        symmetry: Option<crate::search::transform::BoardSymmetry>,
        min_ply_diff: Option<usize>,
        max_ply_diff: Option<usize>,
    },
    Find {
        count_op: Option<(ComparisonOp, usize)>,
        query: Box<SearchQuery>,
    },
}
```
