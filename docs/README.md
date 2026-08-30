# 📚 SCID-MGR Documentation Hub

Welcome to the comprehensive technical documentation for **`scid-mgr`** — an ultra-high-performance chess database engine, JSON-RPC backend server, and PyQt5 graphical interface designed to handle chess databases with **over 10 million games** in real time.

---

## 🗂️ Documentation Navigation

| Document | Description | Key Topics Covered |
| :--- | :--- | :--- |
| 📖 [**Architecture & Workflow**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/ARCHITECTURE_AND_WORKFLOW.md) | Step-by-step breakdown of how the engine works from disk to screen | Binary file formats (`.si5`, `.si4`, `.pgn`), Memory-mapping, JSON-RPC IPC, PyQt5 Virtual Table |
| ⚡ [**Performance & Optimizations**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/PERFORMANCE_AND_OPTIMIZATIONS.md) | Deep dive into algorithms and architectural speedups | Alphabetical Rank Tables, Rayon Parallel Sorting, Hardware Bitboards, Companion `.idx` Cache |
| 📊 [**Benchmarks & Metrics**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/BENCHMARKS_AND_METRICS.md) | Empirical performance results on databases up to 10.35M games | Memory footprint, Sorting benchmarks, Material search timings, Throughput rates |
| 📜 [**CQLi Integration Guide**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/CQL_INTEGRATION_GUIDE.md) | Research and blueprint for running Chess Query Language (CQL) | Process piping, real-time match streaming `<ID>`, preset queries |
| 🔌 [**JSON-RPC Server API**](file:///C:/Users/ASUS/programming/qt_programs/chess/scid-mgr/docs/API_REFERENCE.md) | Complete reference for frontend-backend communication | Command specifications, parameter tables, JSON payload examples |

---

## 🚀 Quick Highlights

- **Scale**: Seamlessly loads, filters, and sorts the **10.35-Million-Game** `LumbrasGigaBase_OTB.si5` database in **< 2 seconds**.
- **Dual Engine**: Full native read/write/compact support for SCID **SI5 & SI4** formats, plus parallel direct **PGN** indexing.
- **Search Capabilities**:
  - Exact & partial piece-on-square visual board searches.
  - Bitboard material searches (e.g. Opposite-Colored Bishops, piece counts).
  - High-speed multi-attribute metadata searches.
- **Ultra-Smooth GUI**: 60 FPS scrolling virtual table with dynamic chunk caching and embedded chessboard editor.
