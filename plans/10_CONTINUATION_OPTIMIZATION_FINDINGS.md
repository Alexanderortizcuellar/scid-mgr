# Continuation Engine Optimization: Code Review and Recommended Experiments

This note reviews `09_CONTINUATION_ENGINE_OPTIMIZATION_PLAN.md` against the current booster continuation implementation. It is a set of findings and experiment recommendations, not an implementation specification. The plan's timing and speedup estimates have not been independently verified here.

## Context: what the booster already does

The `.boost.idx` stores the mainline moves in compact 16-bit `BoostMove` values. Continuation queries scan those packed moves directly; they do not need to parse PGN or decode comments and variations.

The expensive continuation step is aggregating each matching game's next `max_depth` moves into distinct line counts. In `search_booster/evaluator.rs`, both continuation accumulation paths use `HashMap<InlinePath, PathStats>`. `InlinePath` is currently a fixed-size value with a `u8` length and `[u16; 32]` move storage. It avoids allocating a `Vec` for every key, but it reserves 32 plies even when the requested depth is shorter. Rayon worker accumulators later merge their path maps.

Relevant implementation locations:

- `crates/scid-mgr/src/search_booster/evaluator.rs`: `InlinePath`, continuation accumulation, map merging, filtering, sorting, and `max_lines` truncation.
- `crates/scid-mgr/src/search_booster/types.rs`: the 16-bit `BoostMove` representation.

## Findings on the three proposed optimizations

### 1. Faster hash map hasher: worthwhile as an isolated benchmark

A faster non-cryptographic hasher could reduce the cost of hashing path keys during accumulation and map merging. This is a reasonable, reversible experiment for a local database tool. The current continuation maps are not exposed to arbitrary network-supplied keys in the way a public HTTP endpoint might be, but confirm the threat model before changing a security-sensitive service path.

Do not assume the plan's claimed 3–5x hashing gain or that using a deterministic seed makes Rayon merges instant. Each hasher choice, key size, map load factor, and merge pattern affects results. Benchmark the actual continuation call on representative databases and query shapes. Keep this change only if end-to-end latency improves and output remains identical.

### 2. Compact path keys: the most promising structural experiment

The move representation is already 16 bits, so a query path of up to eight plies can be represented in 128 bits plus a length. This could reduce key bytes processed and map memory compared with the current 32-ply `[u16; 32]` key. It is a plausible improvement because the current key reserves 64 bytes of move storage regardless of the query's `max_depth`.

Important qualifications:

- This is integer packing, not automatically SIMD. Equality/hash performance depends on generated code and the selected hasher.
- The plan mixes plies and full moves: eight plies are four full moves. `max_depth` in the implementation is the number of packed moves/plies, so document and test the intended cap in those units.
- The proposed 256-bit form supports 16 plies, not the current `InlinePath` capacity of 32 plies. Do not silently truncate a query beyond the packed capacity. Either validate the limit or preserve a fallback representation.
- Compare complete map memory and runtime; key size alone does not account for map buckets, `PathStats`, allocator overhead, or worker-local maps.
- Include unequal path lengths and paths sharing prefixes in equality/hash correctness checks.

A useful first prototype is a compact key for the common supported depth (for example, up to 16 plies), with an explicit fallback or validation beyond that limit. Compare it to `InlinePath` before changing the production representation.

### 3. Two-pass prefix pruning: only with a correctness proof

The current evaluator applies `min_games`, `min_percentage`, sorting, and `max_lines` after path counts have been collected. `max_lines` alone does not justify dropping prefixes during the scan: a seemingly minor prefix may contain a continuation that belongs in the final top N. Therefore, pruning based only on “Top N” or assumed frequency distribution can change exact results.

A safe basic bound is available for count thresholds: the number of games containing any particular longer continuation cannot exceed the number containing its prefix. A prefix can therefore be excluded when its aggregate count is already below `min_games`. For a percentage threshold, use the same denominator and carefully preserve the existing inclusion semantics. Any Top-N branch-and-bound needs a valid upper bound on every line below each prefix and must prove it cannot exclude a line that would qualify or enter the final ranking. When `min_games` permits one-game lines, rare branches must remain discoverable.

The plan's “98% one-off” distribution and projected 10–20x speedup should be measured on the target datasets and settings before they motivate pruning.

## Recommended experiment order

1. Capture baseline continuation latency and exact outputs for representative cases: starting position and established openings; several depths; small and large databases; varied `min_games` and `max_lines` values. Record database size, build/revision, CPU, and query configuration.
2. Prototype a compact path key while keeping the evaluator's behavior and result filtering unchanged. Compare result sets and counts exactly against the existing implementation, then benchmark.
3. Separately swap in a faster hasher and rerun the same correctness comparison and benchmarks. Keeping the experiments separate makes their contributions measurable.
4. Consider prefix pruning only after defining and testing a mathematically safe pruning bound for the actual filtering and ranking semantics.

Adopt only changes that improve representative end-to-end latency, preserve exact results for supported queries, and do not introduce an unacceptable memory or security trade-off. The numerical performance estimates in the original plan should be treated as targets to test, not expected outcomes.
