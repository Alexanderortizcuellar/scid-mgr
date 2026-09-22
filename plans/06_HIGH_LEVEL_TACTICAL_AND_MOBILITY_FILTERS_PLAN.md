# 🪤 High-Level Tactical & Mobility Filters Plan

> **Design & Engineering Specification Document**  
> **Target**: Advanced Tactical & Mobility Engine Extensions  
> **Status**: Concept & Architecture Planning  

---

## 1. Executive Summary

This plan specifies three high-level, human-readable tactical and piece mobility filters:
1. **`hanging`**: Detects undefended, underdefended, or en prise pieces.
2. **`overloaded`**: Detects overburdened defending pieces performing multiple critical defensive duties.
3. **`static_for`**: Measures piece immobility (detecting passive/unactivated pieces or deeply anchored dominating outposts).

---

## 2. The `hanging` Filter

### 2.1. Chess Definition & Evaluation Criteria
A piece on square $S$ of color $C$ is **hanging** if:
1. **Enemy Attack**: The enemy attacks $S$ ($\text{attacks}(\text{enemy}, S) > 0$).
2. **Lack of Defense**:
   - **Strictly Undefended**: Friendly defenders $= 0$ ($\text{attacks}(C, S) == 0$).
   - **Tactically Underdefended**: The lowest-value attacking piece is less valuable than the piece on $S$ (e.g. Queen attacked by Pawn, even if protected by Rook).

### 2.2. Query Syntax & Examples
```text
# Any hanging black piece
hanging black
hanging white

# Specific piece type hanging (Free Queen / Minor piece)
hanging(queen)
hanging(knight)
hanging in [Q, R]

# Quantifier: Multiple hanging pieces (Finding double-attack targets)
hanging count >= 2

# Square-restricted hanging pieces
hanging on [d4, e5]

# Tactical puzzle generator: White to move, Black has hanging material
hanging black and wtm
```

---

## 3. The `overloaded` Filter

### 3.1. Chess Definition & Evaluation Criteria
A defending piece $D$ on square $S_D$ is **overloaded** if:
1. $D$ defends a target piece $T_1$ (if $D$ is removed, $T_1$ becomes hanging).
2. $D$ simultaneously defends a second target $T_2$, or prevents a mating threat / check on square $S_M$.
3. When simulated via internal sandbox (`what_if remove S_D`), multiple enemy tactical threats become winning at once.

### 3.2. Query Syntax & Examples
```text
# Find games where an enemy Queen is overloaded
overloaded(queen)

# Any overloaded piece defending 2 or more critical targets
overloaded(tasks >= 2)
overloaded(piece in [Q, R, B, N])

# Exploit overloaded piece (Player removed or deflected the defender)
overloaded(queen) and move capture
```

---

## 4. The `static_for` Filter

### 4.1. Chess Definition & Evaluation Criteria
`static_for` inspects the game timeline backward from the current position and measures how many consecutive plies/moves a specific piece has remained stationary on the same square.

### 4.2. Key Use Cases
1. **Unactivated / "Sleeping" Passive Pieces**:
   Rook stuck in the corner (`a1` / `h8`) or passive bishop on `c8` that never developed.
2. **Dominant Blockades & "Octopus" Outposts**:
   A centralized monster knight anchored on `d5` for 15 moves.

### 4.3. Query Syntax & Examples
```text
# 1. Unactivated Corner Rook (Stayed on a1 for 25+ full moves):
static_for(rook on a1, moves >= 25)

# 2. Dominant Outpost Knight (Stayed on d5 for 10+ moves):
static_for(knight on d5, moves >= 10)

# 3. Sleeping Bad Bishop:
static_for(bishop on c8, moves >= 20) and move_number >= 30

# 4. General ply immobility:
static_for(queen, plies >= 16)
```

---

## 5. AST Representation

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum SearchQuery {
    // ...
    Hanging {
        color: Option<shakmaty::Color>,
        piece_matcher: Option<PieceMatcher>,
        squares: Option<Vec<shakmaty::Square>>,
        count_op: Option<(ComparisonOp, usize)>,
    },
    Overloaded {
        piece_matcher: Option<PieceMatcher>,
        min_tasks: usize,
    },
    StaticFor {
        piece_matcher: Option<PieceMatcher>,
        square: Option<shakmaty::Square>,
        min_moves: usize,
        is_plies: bool,
    },
}
```
