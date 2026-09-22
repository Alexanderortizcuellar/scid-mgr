# ♾️ Temporal Invariants, Sacrifices & Material Shifts Plan

> **Design & Engineering Specification Document**  
> **Target**: Advanced Timeline & Dynamic Material Engine  
> **Status**: Concept & Architecture Planning  

---

## 1. Executive Summary

This plan specifies four expressive timeline and dynamic material filters:
1. **`repeat`**: Concise streak matching and total event counting (e.g. perpetual check, multi-forks).
2. **`sacrifice`**: Automated piece sacrifice detection and classification.
3. **`material_change`**: Relative material balance shifts over move intervals.
4. **`always`**: Temporal invariant assertions that hold for the remainder of the game.

---

## 2. The `repeat` Filter

### 2.1. Modes & Evaluation
* **Sequential / Streak Mode (Default)**:
  Asserts that a condition occurs $N$ times consecutively without interruption.
* **Total / Game-wide Mode**:
  Asserts that a condition occurs $N$ times anywhere across the game history.

### 2.2. Query Syntax & Examples
```text
# 1. Perpetual Checks & Checking Streaks (Consecutive):
repeat check 3               # 3 checks in a row
repeat check 5               # 5-move checking rampage
repeat consecutive pin 2     # 2 consecutive pins

# 2. Total Count across Game:
repeat total fork 2          # 2 separate forks in the game
repeat total { piece N on d5 } 4 # Knight visited d5 at 4 distinct moments
```

---

## 3. The `sacrifice` Filter

### 3.1. Chess Definition & Evaluation
A move at ply $P$ is a **sacrifice** if:
1. The moving piece $M$ moves to an attacked square (or captures a piece of lower value).
2. The opponent captures $M$ on the immediately subsequent ply ($P+1$).
3. The net material value lost exceeds the material gained ($\Delta \text{Material} < 0$).

### 3.2. Query Syntax & Examples
```text
# Any Queen sacrifice
sacrifice(queen)
sacrifice Q

# Classic Greek Gift (Bishop sacrifice on h7)
sacrifice(bishop) on h7

# Mating sacrifices (Piece sacrificed leading directly to mate)
sacrifice(queen) and child { child { mate } }

# Sound/Brilliant sacrifice
sacrifice and nag $3
```

---

## 4. The `material_change` Filter

### 4.1. Definition & Usage
Measures the change in material count or balance over a phase or following a tactical skirmish:

```text
# Material lost by White across a sequence:
material_change(white) <= -3       # White lost 3+ points of material

# Queen trade / elimination:
material_change(queens) < 0

# Equal trades (Material balance stayed identical):
material_change == 0
```

---

## 5. The `always` Invariant Filter

### 5.1. Definition & Temporal Logic
In temporal logic ($\square \phi$), **`always { <query> }`** asserts that `<query>` holds true **at the current ply and at every single subsequent ply until the game ends**.

### 5.2. Query Syntax & Examples
```text
# 1. Unmoved piece until the end of the game:
# The bishop on c8 remained on c8 for the rest of the game:
always { piece B on c8 }

# 2. Permanent Queen elimination:
# Queens were traded off and no queen ever appeared again:
always { queens == 0 }

# 3. Permanent Pawn blockade:
# Square d5 stayed occupied by knight until game termination:
always { piece N on d5 }

# 4. Peaceful endgame (No checks until conclusion):
always { not check }
```

---

## 6. AST Representation

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum SearchQuery {
    // ...
    Repeat {
        query: Box<SearchQuery>,
        count: usize,
        consecutive: bool,
    },
    Sacrifice {
        piece_matcher: Option<PieceMatcher>,
        square: Option<shakmaty::Square>,
    },
    MaterialChange {
        color: Option<shakmaty::Color>,
        role: Option<shakmaty::Role>,
        op: ComparisonOp,
        diff: i32,
    },
    Always {
        query: Box<SearchQuery>,
    },
}
```
