use anyhow::Result;
use scid_mgr::search_booster::{
    resolve_companion_booster_path, BoostIndexBuilder, BoostSearchEvaluator, MmapBoostIndex,
};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

fn main() -> Result<()> {
    let pgn_path =
        Path::new(r#"C:\Users\ASUS\programming\qt_programs\chess\twchess\data\master.pgn"#);
    if !pgn_path.exists() {
        eprintln!("PGN database not found at {:?}", pgn_path);
        return Ok(());
    }

    let booster_path = resolve_companion_booster_path(pgn_path);
    println!("================================================================================");
    println!("                 SEARCH BOOSTER COMPREHENSIVE BENCHMARK                         ");
    println!("================================================================================");
    println!("Database: {:?}", pgn_path);
    println!("Booster:  {:?}", booster_path);

    if !booster_path.exists() {
        println!("\n[1] Building Search Booster (.boost.idx)...");
        let _start_build = Instant::now();
        let progress_cb = Arc::new(|scanned: usize, total: usize, _| {
            if scanned.is_multiple_of(50000) || scanned == total {
                print!(
                    "\r  Building 16-bit Search Booster: {} / {} games ({:.1}%)",
                    scanned,
                    total,
                    (scanned as f64 / total.max(1) as f64) * 100.0
                );
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
        });

        let (out_file, total_games, total_plies, elapsed_ms) = BoostIndexBuilder::build_for_pgn(
            pgn_path,
            Some(booster_path.clone()),
            Some(progress_cb),
        )?;
        println!();
        println!(
            "  -> Built in {:.2} s ({:.1} games/s, {:.1} plies/s)",
            elapsed_ms as f64 / 1000.0,
            (total_games as f64 / (elapsed_ms as f64 / 1000.0)),
            (total_plies as f64 / (elapsed_ms as f64 / 1000.0))
        );
        println!("  -> Saved to {:?}", out_file);
    } else {
        println!("\n[1] Existing Search Booster found at {:?}", booster_path);
    }

    let mmap_index = MmapBoostIndex::open(&booster_path)?;
    let evaluator = BoostSearchEvaluator::new(&mmap_index);

    let num_games = mmap_index.game_count();
    let total_moves = mmap_index.total_moves();
    let file_meta = std::fs::metadata(&booster_path)?;
    let file_size_mb = file_meta.len() as f64 / (1024.0 * 1024.0);
    let bytes_per_game = file_meta.len() as f64 / num_games.max(1) as f64;

    println!("\n============================ BOOSTER METRICS =================================");
    println!("  Total Games Indexed:       {:>12}", num_games);
    println!("  Total Plies Encoded:       {:>12}", total_moves);
    println!(
        "  Average Moves per Game:    {:>12.1}",
        total_moves as f64 / num_games.max(1) as f64
    );
    println!("  Booster Index File Size:   {:>12.2} MB", file_size_mb);
    println!(
        "  Index Storage per Game:    {:>12.1} bytes/game",
        bytes_per_game
    );
    println!("================================================================================\n");

    // Benchmark test cases
    let test_fens = [
        (
            "Starting Position (ply 0)",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            Some(1),
        ),
        (
            "Open Game (1. e4 e5)",
            "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2",
            Some(6),
        ),
        (
            "Sicilian Defense (1. e4 c5)",
            "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq c6 0 2",
            Some(6),
        ),
        (
            "Italian Game (1. e4 e5 2. Nf3 Nc6 3. Bc4)",
            "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3",
            Some(12),
        ),
        (
            "Sicilian Najdorf (1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6)",
            "rnbqkb1r/1p2pppp/p2p1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 6",
            Some(20),
        ),
        (
            "French Winawer (1. e4 e6 2. d4 d5 3. Nc3 Bb4)",
            "rnbqk1nr/ppp2ppp/4p3/3p4/1b1PP3/2N5/PPP2PPP/R1BQKBNR w KQkq - 2 4",
            Some(16),
        ),
    ];

    println!("--------------------------------------------------------------------------------");
    println!("                  BENCHMARK 1: EXACT POSITION SEARCH                            ");
    println!("--------------------------------------------------------------------------------");
    println!(
        "{:<38} | {:<10} | {:<12} | {:<14}",
        "Test Query / Opening", "Matches", "Time (ms)", "Throughput"
    );
    println!(
        "---------------------------------------+------------+--------------+----------------"
    );

    for &(label, fen, max_ply) in &test_fens {
        // Warmup
        let _ = evaluator.search_position(fen, max_ply)?;

        let mut iters = 10;
        if num_games > 2_000_000 {
            iters = 5;
        }

        let start = Instant::now();
        let mut match_count = 0;
        for _ in 0..iters {
            let res = evaluator.search_position(fen, max_ply)?;
            match_count = res.len();
        }
        let elapsed = start.elapsed();
        let avg_time_ms = (elapsed.as_secs_f64() * 1000.0) / iters as f64;
        let throughput_gps = (num_games as f64) / (avg_time_ms / 1000.0);

        println!(
            "{:<38} | {:>10} | {:>9.2} ms | {:>11.1} M g/s",
            label,
            match_count,
            avg_time_ms,
            throughput_gps / 1_000_000.0
        );
    }

    println!("\n--------------------------------------------------------------------------------");
    println!("         BENCHMARK 2: NEXT MOVES / OPENING TREE (WITHOUT STATS)                 ");
    println!("--------------------------------------------------------------------------------");
    println!(
        "{:<38} | {:<10} | {:<12} | {:<14}",
        "Base Position", "Next Moves", "Time (ms)", "Throughput"
    );
    println!(
        "---------------------------------------+------------+--------------+----------------"
    );

    for &(label, fen, max_ply) in &test_fens[0..4] {
        // Warmup
        let _ = evaluator.find_next_moves(fen, max_ply)?;

        let iters = 5;
        let start = Instant::now();
        let mut distinct_moves = 0;
        let mut top_moves = Vec::new();
        for _ in 0..iters {
            let res = evaluator.find_next_moves(fen, max_ply)?;
            distinct_moves = res.len();
            top_moves = res;
        }
        let elapsed = start.elapsed();
        let avg_time_ms = (elapsed.as_secs_f64() * 1000.0) / iters as f64;
        let throughput_gps = (num_games as f64) / (avg_time_ms / 1000.0);

        println!(
            "{:<38} | {:>10} | {:>9.2} ms | {:>11.1} M g/s",
            label,
            distinct_moves,
            avg_time_ms,
            throughput_gps / 1_000_000.0
        );

        let top_3: Vec<String> = top_moves
            .iter()
            .take(5)
            .map(|(m, c)| format!("{}: {}", m.to_uci_string(), c))
            .collect();
        println!("   └─ Top moves: [{}]", top_3.join(", "));
    }

    println!("\n--------------------------------------------------------------------------------");
    println!("       BENCHMARK 3: COMMON LINE CONTINUATIONS (WITHOUT STATS)                   ");
    println!("--------------------------------------------------------------------------------");
    println!(
        "{:<38} | {:<10} | {:<12} | {:<14}",
        "Base Position & Depth", "Lines", "Time (ms)", "Throughput"
    );
    println!(
        "---------------------------------------+------------+--------------+----------------"
    );

    let cont_fens = [
        (
            "Start Pos (Depth 2 plies)",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            2,
            Some(2),
        ),
        (
            "Start Pos (Depth 4 plies)",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            4,
            Some(4),
        ),
        (
            "Open Game (Depth 4 plies)",
            "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2",
            4,
            Some(8),
        ),
        (
            "Sicilian (Depth 4 plies)",
            "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq c6 0 2",
            4,
            Some(8),
        ),
    ];

    for &(label, fen, depth, max_ply) in &cont_fens {
        // Warmup
        let _ = evaluator.find_continuations(fen, depth, max_ply)?;

        let iters = 5;
        let start = Instant::now();
        let mut distinct_lines = 0;
        let mut top_lines = Vec::new();
        for _ in 0..iters {
            let res = evaluator.find_continuations(fen, depth, max_ply)?;
            distinct_lines = res.len();
            top_lines = res;
        }
        let elapsed = start.elapsed();
        let avg_time_ms = (elapsed.as_secs_f64() * 1000.0) / iters as f64;
        let throughput_gps = (num_games as f64) / (avg_time_ms / 1000.0);

        println!(
            "{:<38} | {:>10} | {:>9.2} ms | {:>11.1} M g/s",
            label,
            distinct_lines,
            avg_time_ms,
            throughput_gps / 1_000_000.0
        );

        let top_3: Vec<String> = top_lines
            .iter()
            .take(3)
            .map(|(seq, c)| {
                let ucis: Vec<String> = seq.iter().map(|m| m.to_uci_string()).collect();
                format!("{}: {}", ucis.join(" "), c)
            })
            .collect();
        println!("   └─ Top lines: [{}]", top_3.join(" | "));
    }

    println!("\n================================================================================");
    println!("Benchmark completed successfully.");
    println!("================================================================================");

    Ok(())
}
