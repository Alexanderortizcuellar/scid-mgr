# 📊 Benchmarks & Performance Metrics

This document contains empirical performance measurements across different database sizes, including the **10.35-Million-Game** `LumbrasGigaBase_OTB.si5` database.

---

## 1. System Benchmark Environment

- **OS**: Windows 11 x64
- **Processor**: Multi-core x86_64
- **Storage**: Fast NVMe SSD
- **Binary**: `scid-mgr.exe` (Rust release build, with LTO and Rayon parallelization)

---

## 2. Benchmark Results on LumbrasGigaBase (10,355,488 Games)

**Database**: `LumbrasGigaBase_OTB.si5` (2.60 GB on disk, 544,871 players, 234,432 events, 82,528 sites)

```
==========================================================================================
                      DATABASE PERFORMANCE BENCHMARK REPORT                              
==========================================================================================
Database:     LumbrasGigaBase_OTB.si5
Format:       si5
Total Games:  10,355,488
Entities:     544,871 players, 234,432 events, 82,528 sites
Disk Size:    2,603.18 MB
------------------------------------------------------------------------------------------
Category               | Benchmark Operation                        |  Time (ms) | Details / Speed     
-----------------------+--------------------------------------------+------------+---------------------
Database Open          | Read & Parse si5 Database                  |     682.64 | 2,603.18 MB total (.si + .sn + .sg)
Indexing               | Alphabetical Name Ranking (544k players)   |     114.59 | Parallel radix/quicksort lookup array
Sorting (Parallel)     | Sort by ID / Game Index                    |      26.39 | 392,351,421 games/s
Sorting (Parallel)     | Sort by Date                               |   1,442.30 | 7,179,826 games/s
Sorting (Parallel)     | Sort by White Player (Ranked)              |   2,093.97 | 4,945,393 games/s (down from >10.5s)
Sorting (Parallel)     | Sort by Black Player (Ranked)              |   2,125.09 | 4,872,959 games/s
Sorting (Parallel)     | Sort by White ELO                          |   1,223.77 | 8,461,928 games/s
Sorting (Parallel)     | Sort by Black ELO                          |   1,299.36 | 7,969,680 games/s
Sorting (Parallel)     | Sort by ECO Code                           |     654.84 | 15,813,887 games/s
Sorting (Parallel)     | Sort by Event (Ranked)                     |   1,983.25 | 5,221,472 games/s
Sorting (Parallel)     | Sort by Site (Ranked)                      |   1,513.80 | 6,840,736 games/s
Sorting (Parallel)     | Sort by Result                             |     289.65 | 35,751,419 games/s
Filtering (Parallel)   | Player Name (Kasparov)                     |      98.68 | Found 5,309 matching games
Filtering (Parallel)   | Exact ECO (B90 Sicilian Najdorf)           |     464.44 | Found 124,893 matching games
Filtering (Parallel)   | Year Filter (2024)                         |   1,661.73 | Found 212,377 matching games
Filtering (Parallel)   | Result Filter (1-0 White Win)              |      76.58 | Found 3,966,202 matching games
Material Search        | Rook Endgame Search (WR=1, BR=1, WQ=0, BQ=0) | 2,459.00 | Found 12 games in 2.45s via bitboards
PGN Reconstruction     | Full Binary Decode & PGN Generation        |     329.90 | 606 games/sec decode speed
==========================================================================================
Overall Benchmark Duration: 18.57 seconds
```

---

## 3. Benchmark Results on Lichess Elite PGN (280,246 Games)

**Database**: `lichess_elite_2025-11.pgn` (255 MB raw text)

| Operation | Time | Notes |
| :--- | :---: | :--- |
| **Initial 1-Pass Indexing** | ~920 ms | Created companion `.pgn.idx` |
| **Subsequent Open via Cache**| **< 10 ms** | Instant loading |
| **Sort by White Player** | **34 ms** | Parallel sort on 280k games |
| **Player Search (`Carlsen`)** | **12 ms** | Matched 501 games |
| **Random PGN Slice Read** | **0.01 ms** | Direct slice via `Mmap` |

---

## 4. How to Run Benchmarks

### Via Command-Line (CLI)
```powershell
# Standard Benchmark
.\target\release\scid-mgr.exe bench "path\to\database.si5"

# Deep Benchmark (includes full opening ply position searches)
.\target\release\scid-mgr.exe bench "path\to\database.si5" --heavy
```

### Via Graphical Interface (GUI)
1. Open any database in the GUI.
2. Click the **`📊 Metrics / Benchmark...`** button in the connection toolbar.
3. Check **"Deep Position Search"** if desired, then click **`▶ Run Full Benchmark`**.
4. View color-coded execution times or click **`📋 Copy Report`** to copy the formatted markdown table.
