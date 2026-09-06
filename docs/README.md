# 📚 SCID-MGR Documentation Hub

Welcome to the comprehensive technical documentation for **`scid-mgr`** — an ultra-high-performance Rust chess database engine, CLI utility, and JSON-RPC backend server designed to handle chess databases with **over 10 million games** in real time (along with a reference PyQt5 test client for development verification).

---

## 🗂️ Documentation Navigation

| Document | Description | Key Topics Covered |
| :--- | :--- | :--- |
| 📖 [**Architecture & Workflow**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/ARCHITECTURE_AND_WORKFLOW.md) | Step-by-step breakdown of how the engine works from disk to memory & IPC | Binary file formats (`.si5`, `.si4`, `.pgn`), Memory-mapping, Position Index (`.pos.idx`), JSON-RPC IPC |
| ⚡ [**Performance & Optimizations**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/PERFORMANCE_AND_OPTIMIZATIONS.md) | Deep dive into algorithms and architectural speedups | Alphabetical Rank Tables, Rayon Parallel Sorting, Hardware Bitboards, Adaptive Posting Lists |
| 📊 [**Benchmarks & Metrics**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/BENCHMARKS_AND_METRICS.md) | Empirical performance results on databases up to 10.35M games | Memory footprint, Sorting benchmarks, Material search timings, Ingest throughput |
| 📜 [**CQLi Integration Guide**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/CQL_INTEGRATION_GUIDE.md) | Research and blueprint for running Chess Query Language (CQL) | Process piping, real-time match streaming `<ID>`, preset queries |
| 🔌 [**JSON-RPC Server API**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/API_REFERENCE.md) | Complete reference for frontend-backend communication | Command specifications, parameter tables, JSON payload examples |

---

## 🚀 Quick Highlights

- **Scale**: Seamlessly loads, filters, and sorts the **10.35-Million-Game** `LumbrasGigaBase_OTB.si5` database in **< 2 seconds**.
- **Dual Engine**: Full native read/write/compact support for SCID **SI5 & SI4** formats, plus parallel direct **PGN** indexing.
- **Sub-Millisecond Opening Tree**: Inverted position index (`.pos.idx` v3) supporting sub-0.05 ms position lookup with Delta-Varint & Roaring compression.
- **Search Capabilities**:
  - Exact & partial piece-on-square visual board searches.
  - Bitboard material searches (e.g. Opposite-Colored Bishops, piece counts).
  - High-speed multi-attribute metadata searches.
- **IPC Protocol**: Full JSON-RPC NDJSON interface over `stdin`/`stdout` for arbitrary GUI or web frontends.

