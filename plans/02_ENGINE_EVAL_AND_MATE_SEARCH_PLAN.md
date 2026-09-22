# ♟️ UCI Engine Integration & `eval` / `mate_in` Architectural Plan

> **Design & Engineering Specification Document**  
> **Status**: Concept & Architecture Planning  
> **Target**: Future Engine Integration Milestone  

---

## 1. Executive Summary

This document addresses the design and architectural challenges of integrating external UCI chess engines (such as Stockfish) into `scid-mgr`'s multi-threaded search engine. It defines the syntax and semantics for `eval` (evaluation comparisons and blunder detection) and `mate_in` (forced checkmate distance search), while resolving multi-process concurrency and memory consumption bottlenecks.

---

## 2. Core Architectural Challenges & Solutions

### 2.1. The Concurrency & Memory Dilemma
`scid-mgr` evaluates thousands of games in parallel across CPU cores using Rayon. If engine evaluation is introduced naively:
* **Anti-Pattern A (Process per Thread)**: 16 threads $\times$ 16 Stockfish instances $\rightarrow$ severe CPU context switching and massive memory footprint ($16 \times 64\text{MB} = 1\text{GB}+$ RAM).
* **Anti-Pattern B (Single Shared Process)**: 16 threads competing for a single stdin/stdout pipe $\rightarrow$ lock contention bottlenecks the entire scan speed down to single-threaded speed.

### 2.2. The 4-Tier Architecture Solution

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           SEARCH ENGINE QUERY                           │
│             e.g., material_diff <= -3 and mate_in 2 and eval >= +2.0    │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                 ┌───────────────────▼───────────────────┐
                 │ Stage 1: Fast Pipeline Short-Circuit  │
                 │ Evaluates cheap filters (Rust) first. │
                 │ 99.5% of non-matching plies discarded │
                 └───────────────────┬───────────────────┘
                                     │ (Surviving ~0.5% positions)
                 ┌───────────────────▼───────────────────┐
                 │ Stage 2: In-Memory FEN Cache (DashMap)│
                 │ Hits instant cached eval for seen FEN │
                 └───────────────────┬───────────────────┘
                                     │ (Cache Misses)
                 ┌───────────────────┴───────────────────┐
                 ▼                                       ▼
    [For mate_in <= 2]                     [For deep eval / mate >= 3]
  Stage 3: Built-in Rust Solver          Stage 4: Bounded UCI Worker Pool
  Hardware bitboard alpha-beta solver    Bounded pool (e.g. 2-4 workers)
  Zero external processes (microseconds) Fixed Hash (16MB) / Threads (1)
```

---

## 3. Two-Tier `eval` Syntax & Behaviors

### 3.1. Syntax Overview
```text
# General evaluation comparisons (pawns)
eval >= +2.0
eval in -0.5..+0.5
eval <= -1.5

# Forced mate evaluation
eval == #3
eval <= #-2

# Parameterized options
eval(depth=12) >= +3.0
eval(cached_only=true) >= +1.5
eval(engine="stockfish", depth=15) >= +2.0
```

### 3.2. Cached Annotations vs. Live UCI
1. **Tier 1 (Cached Mode)**:
   - Scans PGN comments for standard evaluation tags (`[%eval +1.45]`, `eval: -0.8`, `+2.10`).
   - If present, resolves in **~100 nanoseconds**.
2. **Tier 2 (Live UCI Mode)**:
   - If annotations are absent and live mode is enabled, sends position FEN to the UCI worker pool.
   - If live mode is disabled, returns `false` (or `None`).

---

## 4. Dedicated `mate_in` Function

### 4.1. Why `mate_in` is Fast
* **UCI `go mate N` Support**: UCI engines prune non-forcing quiet moves aggressively when searching exclusively for mates.
* **Rust Native Bitboard Solver**: For `mate_in 1` and `mate_in 2`, `scid-mgr` solves the mate directly in Rust in **microseconds** with zero UCI process overhead.

### 4.2. Syntax Overview
```text
# Forced mate in exactly 2 full moves
mate_in == 2
mate_in 2

# Forced mate within 3 moves
mate_in <= 3

# With search depth/engine bounds
mate_in(depth=14) <= 3
mate_in(engine="stockfish") 4

# Specific player mates
mate_in(white) 2
mate_in(black) 3
```

---

## 5. Blunder & Tactical Swing Analysis

Combining `eval` with `parent { ... }` or delta modifiers allows discovering turning points and mistakes:

```text
# Finding dramatic blunders: Position was winning (+2.5), but next move drops to losing (<= 0.0)
parent { eval >= +2.5 } and eval <= 0.0

# Missed winning tactic: Legal move is +4.0, but played move was 0.0
play { eval >= +4.0 } and eval <= 0.0

# Critical defender: Removing Black's knight causes White's eval to jump from 0.0 to +6.0
what_if remove f6 { eval >= +6.0 } and eval <= +1.0
```

---

## 6. UCI Process Pool Architecture (Rust)

### 6.1. Worker Lifecycle
```rust
pub struct UciWorker {
    process: std::process::Child,
    stdin: std::io::BufWriter<std::process::ChildStdin>,
    stdout: std::io::BufReader<std::process::ChildStdout>,
}

pub struct UciPool {
    workers: crossbeam_channel::Sender<UciWorker>,
    receiver: crossbeam_channel::Receiver<UciWorker>,
    cache: dashmap::DashMap<shakmaty::zobrist::ZobristHash, i32>,
    max_workers: usize,
}
```

### 6.2. Resource Safeguards
* **Engine Hash Constraint**: Default `setoption name Hash value 16` (16MB per worker).
* **Engine Thread Constraint**: Default `setoption name Threads value 1` (1 thread per worker).
* **Timeout & Early Abort**: Queries time out after a configurable limit (e.g. 100ms per position) to prevent infinite hangs.
* **Process Auto-Restart**: If an engine crashes or exits, the pool automatically spawns a healthy replacement worker.

---

## 7. Next Steps & Roadmap

1. Implement `plans/01_WHAT_IF_FILTER_PLAN.md` (pure Rust board sandbox).
2. Build native Rust `mate_in 1` and `mate_in 2` bitboard proof solver.
3. Implement `UciPool` process manager and `eval` / `mate_in` AST nodes.
4. Integrate cache-first PGN comment parser (`[%eval ...]`).
5. Expose engine configurations in the GUI Workbench settings.
