# 📜 CQLi (Chess Query Language) Integration Guide

This guide details how `scid-mgr` interfaces with `cqli` (Chess Query Language interpreter v1.0.7) to enable grandmaster-level study searches, tactical motifs, and geometric chess patterns.

---

## 1. Overview of CQLi

CQL allows querying chess games using an expressive domain-specific language for concepts such as:
- Smothered mates & king hunts.
- Knight and underpromotion tactics.
- Queen dominations & piece traps.
- Fortress and endgame geometric studies.

---

## 2. Command Execution Pattern

`cqli` operates on PGN streams with real-time match reporting:

```bash
cqli -i <input.pgn> -o <temp_out.pgn> --showmatches --lineincrement 50 --nosort <query.cql>
```

### Key Flags
| Flag | Purpose |
| :--- | :--- |
| `-i <file>` | Path to the source PGN file (either open `.pgn` or temp exported `.si5`). |
| `-o <file>` | Destination PGN file with annotated moves and comments. |
| `--showmatches` | Emits matching 1-based game IDs (`<1>`, `<5>`, `<12>`) to stdout in real-time. |
| `--lineincrement <N>` | Emits progress indicators (`[50]`, `[100]`) to calculate progress percentages. |
| `--nosort` | Bypasses `cqli` internal sorting for maximum throughput. |

---

## 3. High-Value CQL Presets

### 1. Smothered Mate (Philidor's Legacy)
```cql
cql()
mate
piece [nN] attacks king
```

### 2. Knight Underpromotion
```cql
cql()
piece promote in [nN]
```

### 3. Queen Trapped / Dominated
```cql
cql()
piece q on . attacks 0
```

### 4. Castling with Simultaneous Check
```cql
cql()
move from [e1,e8] to [g1,c1,g8,c8] check
```

### 5. Opposite-Colored Bishops with Rook
```cql
cql()
material [R B:1 p*] [r b:1 p*]
oppositecoloredbishops
```

---

## 4. Virtual Table Integration

When `cqli` emits match tokens (`<ID>`), the server collects these matching indices into a `Vec<usize>`. 
The frontend table model instantly filters the table to display only the matched games with **60 FPS** virtual scrolling.
