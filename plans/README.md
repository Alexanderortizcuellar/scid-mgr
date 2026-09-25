# 🗺️ SCID-MGR Language & Engine Architecture Roadmaps

> **Engineering Design & Planning Directory**  
> This directory contains formal technical specifications, grammar extensions, evaluation mechanics, and concurrency architectures for upcoming features in the SCID-MGR query language and search engine.

---

## 🚀 Active Feature Plans

| # | Plan Document | Feature | Status | Key Highlights |
| :-: | :--- | :--- | :---: | :--- |
| **02** | [**`02_ENGINE_EVAL_AND_MATE_SEARCH_PLAN.md`**](./02_ENGINE_EVAL_AND_MATE_SEARCH_PLAN.md) | UCI Engine Integration (`eval` / `mate_in`) | 📋 Architecture Draft | 4-tier pipeline, bounded UCI worker pool, FEN cache, `eval` comparisons, native Rust $N \le 2$ mate solver. |
| **03** | [**`03_ECHO_AND_FIND_FILTERS_PLAN.md`**](./03_ECHO_AND_FIND_FILTERS_PLAN.md) | `echo` & `find` Filters | 📋 Architecture Draft | Echo pattern repetition, symmetrical motifs, multi-ply distances, occurrence search. |
| **04** | [**`04_RANK_FILE_AND_TIMECONTROL_UTILITIES_PLAN.md`**](./04_RANK_FILE_AND_TIMECONTROL_UTILITIES_PLAN.md) | `rank`, `file`, `time_control` | 📋 Architecture Draft | Geometric square generators (`rank 7`, `file e`), native PGN time categories (`bullet`, `blitz`, `rapid`, `classical`). |
| **05** | [**`05_PROGRAMMING_LANGUAGE_FEATURES_PLAN.md`**](./05_PROGRAMMING_LANGUAGE_FEATURES_PLAN.md) | Scripting & Programming Features | 📋 Architecture Draft | Control flow (`if`/`else`, `for`, `while`), user functions (`fn`), types & methods (`.lower()`, `.distance_to()`), math stdlib (`max`, `min`, `avg`, `sum`). |
| **06** | [**`06_HIGH_LEVEL_TACTICAL_AND_MOBILITY_FILTERS_PLAN.md`**](./06_HIGH_LEVEL_TACTICAL_AND_MOBILITY_FILTERS_PLAN.md) | Tactical & Mobility Filters | 📋 Architecture Draft | High-level tactical detection: `hanging`, `overloaded` (defending multiple targets/threats), and `static_for` (immobility / idle piece tracking). |
| **07** | [**`07_TEMPORAL_INVARIANTS_AND_SACRIFICES_PLAN.md`**](./07_TEMPORAL_INVARIANTS_AND_SACRIFICES_PLAN.md) | Temporal Invariants & Sacrifices | 📋 Architecture Draft | Motif recurrence (`repeat`), tactical investment & recovery (`sacrifice`), material delta tracking (`material_change`), and game-long persistence (`always`). |
| **08** | [**`08_KING_SAFETY_ITERATORS_AND_GENERALIZED_VARIABLES_PLAN.md`**](./08_KING_SAFETY_ITERATORS_AND_GENERALIZED_VARIABLES_PLAN.md) | King Safety, Iterators & Variables | 📋 Architecture Draft | `king_safety` (shield & attack metrics), quantified board loops (`for_square`, `for_piece`, `any_...`, `all_...`), generalized `let $var = ...` binding. |

---

## 📦 Completed & Implemented Plans (`plans/completed/`)

* [**`01_WHAT_IF_FILTER_PLAN.md`**](./completed/01_WHAT_IF_FILTER_PLAN.md) — Speculative board mutator & isolated sandbox engine: `remove`, `pass`, `swap`, `swap_color`, `move`, `add`, `turn`, and move sequence rollouts `[...]`. *(Completed)*
* [**`03_CQL_LINE_PLAN.md`**](./completed/03_CQL_LINE_PLAN.md) — Forward/backward line transitions, repetition quantifiers (`+`, `*`, `?`, `{m,n}`), lookahead/lookbehind, and nestban deduplication. *(Completed)*
* [**`04_SQUARE_SET_ALGEBRA_PLAN.md`**](./completed/04_SQUARE_SET_ALGEBRA_PLAN.md) — Hardware bitboard square set algebra, operators (`&`, `|`, `-`, `^`), bracket sets (`[K, Q]`), set comparisons. *(Completed)*
