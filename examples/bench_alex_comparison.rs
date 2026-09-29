use anyhow::Result;
use pgn_reader::{BufferedReader, SanPlus, Skip, Visitor};
use rayon::prelude::*;
use scid_mgr::search_booster::{
    chess_to_board_array, resolve_companion_booster_path, BoostIndexBuilder, BoostSearchEvaluator,
    MmapBoostIndex,
};
use shakmaty::fen::Fen;
use shakmaty::{CastlingMode, Chess, Position};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::time::Instant;

fn raw_pgn_search_position(
    mmap: &[u8],
    offsets: &[(usize, usize)],
    target_fen: &str,
) -> Result<Vec<(usize, Vec<usize>)>> {
    let fen: Fen = target_fen.parse()?;
    let target_pos: Chess = fen
        .clone()
        .into_position(CastlingMode::Standard)
        .or_else(|_| {
            fen.into_position(CastlingMode::Chess960)
                .map_err(|e| anyhow::anyhow!("{}", e))
        })?;
    let target_board = chess_to_board_array(&target_pos);

    let matches: Vec<(usize, Vec<usize>)> = offsets
        .par_iter()
        .enumerate()
        .filter_map(|(gid, &(start_pos, end_pos))| {
            let chunk = &mmap[start_pos..end_pos];
            let mut reader = BufferedReader::new(chunk);
            let mut pos = Chess::default();
            let mut matching_plies = Vec::new();
            let mut ply = 0;

            if chess_to_board_array(&pos) == target_board {
                matching_plies.push(0);
            }

            struct RawCollector<'a> {
                pos: &'a mut Chess,
                matching_plies: &'a mut Vec<usize>,
                ply: &'a mut usize,
                target_board: &'a [u8; 64],
            }

            impl<'a> Visitor for RawCollector<'a> {
                type Result = ();

                fn header(&mut self, key: &[u8], value: pgn_reader::RawHeader<'_>) {
                    if key == b"FEN" {
                        let fen_str = String::from_utf8_lossy(value.as_bytes());
                        if let Ok(fen) = fen_str.parse::<Fen>() {
                            if let Ok(p) = fen.into_position(CastlingMode::Standard) {
                                *self.pos = p;
                                if chess_to_board_array(self.pos) == *self.target_board {
                                    self.matching_plies.push(0);
                                }
                            }
                        }
                    }
                }

                fn end_headers(&mut self) -> Skip {
                    Skip(false)
                }

                fn san(&mut self, san_plus: SanPlus) {
                    if let Ok(m) = san_plus.san.to_move(self.pos) {
                        self.pos.play_unchecked(&m);
                        *self.ply += 1;
                        if chess_to_board_array(self.pos) == *self.target_board {
                            self.matching_plies.push(*self.ply);
                        }
                    }
                }

                fn begin_variation(&mut self) -> Skip {
                    Skip(true)
                }

                fn end_game(&mut self) -> Self::Result {}
            }

            let mut collector = RawCollector {
                pos: &mut pos,
                matching_plies: &mut matching_plies,
                ply: &mut ply,
                target_board: &target_board,
            };
            let _ = reader.read_all(&mut collector);

            if !matching_plies.is_empty() {
                Some((gid, matching_plies))
            } else {
                None
            }
        })
        .collect();

    Ok(matches)
}

fn raw_pgn_find_next_moves(
    mmap: &[u8],
    offsets: &[(usize, usize)],
    target_fen: &str,
) -> Result<HashMap<String, u32>> {
    let fen: Fen = target_fen.parse()?;
    let target_pos: Chess = fen
        .clone()
        .into_position(CastlingMode::Standard)
        .or_else(|_| {
            fen.into_position(CastlingMode::Chess960)
                .map_err(|e| anyhow::anyhow!("{}", e))
        })?;
    let target_board = chess_to_board_array(&target_pos);

    let move_counts = offsets
        .par_iter()
        .fold(
            HashMap::<String, u32>::new,
            |mut acc, &(start_pos, end_pos)| {
                let chunk = &mmap[start_pos..end_pos];
                let mut reader = BufferedReader::new(chunk);
                let mut pos = Chess::default();
                let mut prev_matched = chess_to_board_array(&pos) == target_board;

                struct RawTreeCollector<'a> {
                    pos: &'a mut Chess,
                    prev_matched: &'a mut bool,
                    target_board: &'a [u8; 64],
                    acc: &'a mut HashMap<String, u32>,
                }

                impl<'a> Visitor for RawTreeCollector<'a> {
                    type Result = ();

                    fn end_headers(&mut self) -> Skip {
                        Skip(false)
                    }

                    fn san(&mut self, san_plus: SanPlus) {
                        if *self.prev_matched {
                            *self.acc.entry(san_plus.san.to_string()).or_insert(0) += 1;
                        }
                        if let Ok(m) = san_plus.san.to_move(self.pos) {
                            self.pos.play_unchecked(&m);
                            *self.prev_matched =
                                chess_to_board_array(self.pos) == *self.target_board;
                        }
                    }

                    fn begin_variation(&mut self) -> Skip {
                        Skip(true)
                    }

                    fn end_game(&mut self) -> Self::Result {}
                }

                let mut collector = RawTreeCollector {
                    pos: &mut pos,
                    prev_matched: &mut prev_matched,
                    target_board: &target_board,
                    acc: &mut acc,
                };
                let _ = reader.read_all(&mut collector);
                acc
            },
        )
        .reduce(HashMap::new, |mut map1, map2| {
            for (k, v) in map2 {
                *map1.entry(k).or_insert(0) += v;
            }
            map1
        });

    Ok(move_counts)
}

fn main() -> Result<()> {
    let pgn_path = Path::new(r#"C:\Users\ASUS\chess\database\games\chesscom\alex.pgn"#);
    if !pgn_path.exists() {
        eprintln!("Database not found at {:?}", pgn_path);
        return Ok(());
    }

    let booster_path = resolve_companion_booster_path(pgn_path);

    println!("================================================================================");
    println!("             BENCHMARK: SEARCH BOOSTER VS RAW PGN ON ALEX.PGN                   ");
    println!("================================================================================");
    println!("Database: {:?}", pgn_path);

    // 1. Build booster if missing
    if !booster_path.exists() {
        println!("\n[1] Building Search Booster (.boost.idx)...");
        let _start = Instant::now();
        let (_out_file, total_games, total_plies, elapsed_ms) =
            BoostIndexBuilder::build_for_pgn(pgn_path, Some(booster_path.clone()), None)?;
        println!(
            "  -> Built in {:.2} ms ({} games, {} plies) at {:.1} games/sec",
            elapsed_ms as f64,
            total_games,
            total_plies,
            (total_games as f64 / (elapsed_ms as f64 / 1000.0))
        );
    } else {
        println!("\n[1] Existing Search Booster found at {:?}", booster_path);
    }

    let raw_file = File::open(pgn_path)?;
    let raw_mmap = unsafe { memmap2::Mmap::map(&raw_file)? };
    let raw_offsets = scid_mgr::pgn::scan_pgn_game_offsets(&raw_mmap);

    let mmap_index = MmapBoostIndex::open(&booster_path)?;
    let evaluator = BoostSearchEvaluator::new(&mmap_index);

    let num_games = mmap_index.game_count();
    let total_moves = mmap_index.total_moves();
    let pgn_size_kb = raw_mmap.len() as f64 / 1024.0;
    let boost_size_kb = std::fs::metadata(&booster_path)?.len() as f64 / 1024.0;

    // Check size of existing companion indexes in the same folder if present
    let dir = pgn_path.parent().unwrap();
    let pos_idx_kb = std::fs::metadata(dir.join("alex.pgn.pos.idx"))
        .map(|m| m.len() as f64 / 1024.0)
        .unwrap_or(0.0);
    let tree_idx_kb = std::fs::metadata(dir.join("alex.pgn.tree.idx"))
        .map(|m| m.len() as f64 / 1024.0)
        .unwrap_or(0.0);
    let hot_idx_kb = std::fs::metadata(dir.join("alex.pgn.hot.idx"))
        .map(|m| m.len() as f64 / 1024.0)
        .unwrap_or(0.0);
    let feat_idx_kb = std::fs::metadata(dir.join("alex.pgn.feat.idx"))
        .map(|m| m.len() as f64 / 1024.0)
        .unwrap_or(0.0);
    let total_old_idx_kb = pos_idx_kb + tree_idx_kb + hot_idx_kb + feat_idx_kb;

    println!("\n============================ STORAGE COMPARISON ===============================");
    println!(
        "  Raw PGN File Size:              {:>10.2} KB ({:.2} MB)",
        pgn_size_kb,
        pgn_size_kb / 1024.0
    );
    println!(
        "  Search Booster (.boost.idx):    {:>10.2} KB ({:.2} MB)",
        boost_size_kb,
        boost_size_kb / 1024.0
    );
    println!(
        "  Booster vs Raw PGN:             {:>10.1}% size reduction",
        (1.0 - boost_size_kb / pgn_size_kb) * 100.0
    );
    println!("  -------------------------------------------------------------");
    println!(
        "  Old Companion Indexes Total:    {:>10.2} KB ({:.2} MB)",
        total_old_idx_kb,
        total_old_idx_kb / 1024.0
    );
    println!("    - Position Index (.pos.idx):  {:>10.2} KB", pos_idx_kb);
    println!("    - Opening Tree (.tree.idx):   {:>10.2} KB", tree_idx_kb);
    println!("    - Continuations (.hot.idx):   {:>10.2} KB", hot_idx_kb);
    println!("    - Endgame Index (.feat.idx):  {:>10.2} KB", feat_idx_kb);
    println!(
        "  Booster vs All 4 Old Indexes:   {:>10.1}% smaller footprint!",
        (1.0 - boost_size_kb / total_old_idx_kb) * 100.0
    );
    println!(
        "  Total Games: {:>6} | Total Moves: {:>8}",
        num_games, total_moves
    );
    println!("================================================================================\n");

    let openings = [
        (
            "Alapin Sicilian (1. e4 c5 2. c3)",
            "rnbqkbnr/pp1ppppp/8/2p5/4P3/2P5/PP1P1PPP/RNBQKBNR b KQkq - 0 2",
            Some(6),
        ),
        (
            "Italian Game (1. e4 e5 2. Nf3 Nc6 3. Bc4)",
            "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3",
            Some(8),
        ),
        (
            "Caro-Kann Advance (1. e4 c6 2. d4 d5 3. e5)",
            "rnbqkbnr/pp2pppp/2p5/3pP3/3P4/8/PPP2PPP/RNBQKBNR b KQkq - 0 3",
            Some(8),
        ),
        (
            "Caro-Kann Exchange (1. e4 c6 2. d4 d5 3. exd5 cxd5)",
            "rnbqkbnr/pp2pppp/8/3p4/3P4/8/PPP2PPP/RNBQKBNR w KQkq - 0 4",
            Some(8),
        ),
    ];

    println!("--------------------------------------------------------------------------------");
    println!("           BENCHMARK 1: EXACT POSITION SEARCH (RAW PGN VS BOOSTER)              ");
    println!("--------------------------------------------------------------------------------");
    println!(
        "{:<36} | {:<7} | {:<12} | {:<12} | {:<8}",
        "Opening Query", "Matches", "Raw PGN", "Booster", "Speedup"
    );
    println!(
        "-------------------------------------+---------+--------------+--------------+----------"
    );

    for &(name, fen, max_ply) in &openings {
        // Raw PGN timing (50 iterations)
        let iters = 50;
        let start_raw = Instant::now();
        for _ in 0..iters {
            let _ = raw_pgn_search_position(&raw_mmap, &raw_offsets, fen)?;
        }
        let elapsed_raw_us = start_raw.elapsed().as_micros() as f64 / iters as f64;

        // Booster timing (200 iterations)
        let iters_b = 200;
        let start_b = Instant::now();
        let mut matches_b = 0;
        for _ in 0..iters_b {
            let res = evaluator.search_position(fen, max_ply)?;
            matches_b = res.len();
        }
        let elapsed_b_us = start_b.elapsed().as_micros() as f64 / iters_b as f64;
        let speedup = elapsed_raw_us / elapsed_b_us.max(0.001);

        println!(
            "{:<36} | {:>7} | {:>9.2} ms | {:>9.2} µs | {:>7.1}x",
            name,
            matches_b,
            elapsed_raw_us / 1000.0,
            elapsed_b_us,
            speedup
        );
    }

    println!("\n--------------------------------------------------------------------------------");
    println!("       BENCHMARK 2: NEXT MOVES / OPENING TREE (RAW PGN VS BOOSTER)              ");
    println!("--------------------------------------------------------------------------------");
    println!(
        "{:<36} | {:<7} | {:<12} | {:<12} | {:<8}",
        "Opening Position", "Moves", "Raw PGN", "Booster", "Speedup"
    );
    println!(
        "-------------------------------------+---------+--------------+--------------+----------"
    );

    for &(name, fen, max_ply) in &openings {
        let iters = 50;
        let start_raw = Instant::now();
        for _ in 0..iters {
            let _ = raw_pgn_find_next_moves(&raw_mmap, &raw_offsets, fen)?;
        }
        let elapsed_raw_us = start_raw.elapsed().as_micros() as f64 / iters as f64;

        let iters_b = 200;
        let start_b = Instant::now();
        let mut moves_b_count = 0;
        let mut top_moves = Vec::new();
        for _ in 0..iters_b {
            let res = evaluator.find_next_moves(fen, max_ply)?;
            moves_b_count = res.len();
            top_moves = res;
        }
        let elapsed_b_us = start_b.elapsed().as_micros() as f64 / iters_b as f64;
        let speedup = elapsed_raw_us / elapsed_b_us.max(0.001);

        println!(
            "{:<36} | {:>7} | {:>9.2} ms | {:>9.2} µs | {:>7.1}x",
            name,
            moves_b_count,
            elapsed_raw_us / 1000.0,
            elapsed_b_us,
            speedup
        );

        let top_3: Vec<String> = top_moves
            .iter()
            .take(4)
            .map(|(m, c)| format!("{}: {}", m.to_uci_string(), c))
            .collect();
        println!("   └─ Next moves in alex.pgn: [{}]", top_3.join(", "));
    }

    println!("\n--------------------------------------------------------------------------------");
    println!("             BENCHMARK 3: COMMON LINE CONTINUATIONS                             ");
    println!("--------------------------------------------------------------------------------");
    println!(
        "{:<36} | {:<7} | {:<12} | {:<14}",
        "Opening Base", "Lines", "Booster Time", "Throughput"
    );
    println!("-------------------------------------+---------+--------------+----------------");

    for &(name, fen, max_ply) in &openings {
        let iters_b = 200;
        let start_b = Instant::now();
        let mut lines_count = 0;
        let mut top_lines = Vec::new();
        for _ in 0..iters_b {
            let res = evaluator.find_continuations(fen, 4, max_ply)?;
            lines_count = res.len();
            top_lines = res;
        }
        let elapsed_b_us = start_b.elapsed().as_micros() as f64 / iters_b as f64;
        let throughput = (num_games as f64) / (elapsed_b_us / 1_000_000.0);

        println!(
            "{:<36} | {:>7} | {:>9.2} µs | {:>11.1} M g/s",
            name,
            lines_count,
            elapsed_b_us,
            throughput / 1_000_000.0
        );

        let top_2: Vec<String> = top_lines
            .iter()
            .take(3)
            .map(|(seq, c)| {
                let ucis: Vec<String> = seq.iter().map(|m| m.to_uci_string()).collect();
                format!("{}: {}", ucis.join(" "), c)
            })
            .collect();
        println!("   └─ Top lines: [{}]", top_2.join(" | "));
    }

    println!("\n================================================================================");
    println!("Benchmark completed successfully.");
    println!("================================================================================");

    Ok(())
}
