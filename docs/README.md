# 📚 SCID-MGR Documentation Hub

Welcome to the comprehensive technical documentation for **`scid-mgr`** — an ultra-high-performance Rust chess database engine, CLI utility, and JSON-RPC backend server designed to handle chess databases with **over 10 million games** in real time (along with a reference PyQt5 test client for development verification).

---

## 🗂️ Documentation Navigation

| Document | Description | Key Topics Covered |
| :--- | :--- | :--- |
| 📖 [**Architecture & Workflow**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/ARCHITECTURE_AND_WORKFLOW.md) | Step-by-step breakdown of how the engine works from disk to memory & IPC | Binary file formats (`.si5`, `.si4`, `.pgn`), Memory-mapping, Companion Indexes (`.pos.idx`, `.tree.idx`, `.hot.idx`, `.feat.idx`), JSON-RPC IPC |
| 📖 [**CQLite Search Engine Manual**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/manual/README.md) | Comprehensive 8-chapter user manual for the search engine | Header filters, piece identifiers, move & path patterns, pawn structures, tactical motifs, material & power, transformations, boolean logic |
| 🔍 [**Search Engine Reference**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/SEARCH_ENGINE.md) | Specification & query examples for the modular search engine | AST keywords, Headers, Positions, Move paths, Bitboard material, Native SCID/PGN |
| 📈 [**Common Continuations Engine**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/CONTINUATIONS_ENGINE.md) | Sequence analysis & common continuations graph engine | `.hot.idx` binary DAG format (`CHSHOTG1`), dynamic candidate search, CLI, REPL, JSON-RPC |
| ♟️ [**Endgame Taxonomy & Feature Index**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/ENDGAME_INDEX.md) | 47-feature catalog & ultra-compact binary index | `.feat.idx` 8-byte format (`CHSFEAT1`), bitwise filtering, win/draw/loss popularity analytics, CLI, JSON-RPC |
| 🗺️ [**Search Engine Roadmap**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/SEARCH_ENGINE_ROADMAP.md) | Phased development plan & progress tracking | Tactical motifs, Pawn structures, Native SCID adapter, Query DSL, Planner |
| ⚡ [**Performance & Optimizations**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/PERFORMANCE_AND_OPTIMIZATIONS.md) | Deep dive into algorithms and architectural speedups | Alphabetical Rank Tables, Rayon Parallel Sorting, Hardware Bitboards, Adaptive Posting Lists |
| 📊 [**Benchmarks & Metrics**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/BENCHMARKS_AND_METRICS.md) | Empirical performance results on databases up to 10.35M games | Memory footprint, Sorting benchmarks, Material search timings, Ingest throughput |
| 📜 [**CQLi Integration Guide**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/CQL_INTEGRATION_GUIDE.md) | Research and blueprint for running Chess Query Language (CQL) | Process piping, real-time match streaming `<ID>`, preset queries |
| 🔌 [**JSON-RPC Server API**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/API_REFERENCE.md) | Complete reference for frontend-backend communication | Command specifications, parameter tables, JSON payload examples |

---

## 🚀 Quick Highlights

- **Scale**: Seamlessly loads, filters, and sorts the **10.35-Million-Game** `LumbrasGigaBase_OTB.si5` database in **< 2 seconds**.
- **Dual Engine**: Full native read/write/compact support for SCID **SI5 & SI4** formats, plus parallel direct **PGN** indexing.
- **Position Search Accelerator (`.pos.idx`)**: Inverted position index with Delta-Varint posting lists accelerating candidate board searches in microseconds.
- **Sub-Millisecond Opening Tree (`.tree.idx`)**: Binary precomputed opening tree index supporting < 0.05 ms position statistics and move distributions.
- **Search Capabilities**:
  - Exact & partial piece-on-square visual board searches.
  - Bitboard material searches (e.g. Opposite-Colored Bishops, piece counts).
  - High-speed multi-attribute metadata searches.
- **IPC Protocol**: Full JSON-RPC NDJSON interface over `stdin`/`stdout` for arbitrary GUI or web frontends.

