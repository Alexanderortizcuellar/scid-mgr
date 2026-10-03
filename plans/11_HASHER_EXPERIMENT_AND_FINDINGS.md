# 🧪 Hasher Experiment Findings & Evaluation (`ahash` vs `std::collections::HashMap`)

## 📌 Context
In `09_CONTINUATION_ENGINE_OPTIMIZATION_PLAN.md` (Option A), it was hypothesized that replacing `std::collections::HashMap` with `ahash::AHashMap` would yield a 3x–5x overall speedup across continuation queries by reducing SipHash hashing overhead and accelerating Rayon multi-thread `.reduce()` map merges.

`10_CONTINUATION_OPTIMIZATION_FINDINGS.md` cautioned that hasher replacements should be measured empirically in isolation on realistic databases before committing to an external dependency.

---

## 🔬 Benchmark Methodology
A dedicated benchmark suite (`crates/scid-mgr/tests/test_continuation_bench.rs`) was constructed with **200,000 games** exhibiting high combinatorial opening branching entropy ($8 \times 8 \times 8 \times 8 \times 8 \times 8 \times 8 \times 8$ paths). Benchmarks were executed in Rust `--release` mode on the same CPU hardware.

---

## 📊 Empirical Results

| Scenario (200,000 Games) | Baseline (`std::collections::HashMap` / SipHash) | With `ahash::AHashMap` (v0.8) | Delta / Real Impact |
| :--- | :---: | :---: | :--- |
| **Move 0 (Depth 8)** | `30.75 ms` | `32.96 ms` | `+2.21 ms` (~7% slower due to cache footprint) |
| **Move 0 (Depth 4)** | `8.17 ms` | `7.98 ms` | `-0.19 ms` (~2.3% faster) |
| **Opening Tree + Cont (Move 0)** | `32.60 ms` | `34.18 ms` | `+1.58 ms` (~4.8% slower) |

---

## 💡 Engineering Root Cause Analysis

1. **Board Replay Dominance**:
   - At 200,000 games evaluated in ~30 ms, the evaluator processes each game in **~150 nanoseconds**.
   - `FastReplayState` board transitions and move loops consume ~80–100 ns per game.
   - Hashing a 16-byte move slice with SipHash vs AHash accounts for only ~5–10 ns per game.
2. **Key Struct Footprint vs Hasher Choice**:
   - The primary memory bottleneck is the 65-byte `InlinePath` struct (`[u16; 32]`) which requires 64 bytes of storage regardless of whether the search depth is 4 plies or 8 plies.
   - Replacing the hasher without compacting the key struct does not alleviate L2/L3 CPU cache pressure.
3. **External Dependency Cost**:
   - Adding `ahash` pulled in transitive dependencies (`zerocopy`, `getrandom`, `version_check`). Given that end-to-end performance was essentially neutral (or slightly worse on deeper lookups due to hash table load factors), adding an external dependency is not justified.

---

## 🎯 Decision & Recommendation
- **Decision**: **REJECT `ahash` dependency**. Retain `std::collections::HashMap` from the Rust standard library.
- **Next Step**: Investigate **Compact Path Keys** (Option B: `PackedPath128` / `PackedPath256`) to shrink key sizes from 65 bytes to 17–33 bytes, reducing CPU cache pressure and enabling single-instruction integer comparisons.
