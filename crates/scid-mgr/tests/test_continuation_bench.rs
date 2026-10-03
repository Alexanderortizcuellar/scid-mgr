use anyhow::Result;
use scid_mgr::continuation_index::ContinuationQuery;
use scid_mgr::search_booster::{
    resolve_companion_booster_path, BoostIndexBuilder, BoostSearchEvaluator, MmapBoostIndex,
};
use std::time::Instant;
use tempfile::tempdir;

#[test]
fn test_benchmark_continuation_scaling() -> Result<()> {
    let dir = tempdir()?;
    let pgn_path = dir.path().join("large_bench.pgn");

    // Generate 200,000 games with realistic opening tree branching
    let move1_w = ["e4", "d4", "c4", "Nf3", "g3", "f4", "b3", "Nc3"];
    let move1_b = ["e5", "c5", "e6", "c6", "d5", "Nf6", "g6", "d6"];
    let move2_w = ["Nf3", "Nc3", "c4", "d4", "Bc4", "g3", "Be2", "d3"];
    let move2_b = ["Nc6", "Nf6", "d6", "e5", "c5", "d5", "a6", "Be7"];
    let move3_w = ["Bb5", "Bc4", "d4", "O-O", "d3", "exd5", "cxd5", "Re1"];
    let move3_b = ["a6", "Nf6", "Bc5", "d6", "cxd4", "Nxd5", "exd5", "O-O"];
    let move4_w = ["d4", "c3", "O-O", "Nxd4", "Qxd4", "Bxc6", "Re1", "h3"];
    let move4_b = ["exd4", "b5", "Be7", "Nxd4", "Qxd4", "dxc6", "O-O", "Bg4"];

    let mut pgn_data = String::with_capacity(30_000_000);
    let total_bench_games = 200_000;
    for i in 0..total_bench_games {
        let m1w = move1_w[i % move1_w.len()];
        let m1b = move1_b[(i / 2) % move1_b.len()];
        let m2w = move2_w[(i / 3) % move2_w.len()];
        let m2b = move2_b[(i / 5) % move2_b.len()];
        let m3w = move3_w[(i / 7) % move3_w.len()];
        let m3b = move3_b[(i / 11) % move3_b.len()];
        let m4w = move4_w[(i / 13) % move4_w.len()];
        let m4b = move4_b[(i / 17) % move4_b.len()];

        let header = format!(
            "[Event \"Bench {}\"]\n[White \"W {}\"]\n[Black \"B {}\"]\n[Result \"1-0\"]\n\n",
            i, i, i
        );
        pgn_data.push_str(&header);
        let moves_str = format!(
            "1. {} {} 2. {} {} 3. {} {} 4. {} {} 1-0\n\n",
            m1w, m1b, m2w, m2b, m3w, m3b, m4w, m4b
        );
        pgn_data.push_str(&moves_str);
    }

    std::fs::write(&pgn_path, pgn_data)?;
    let booster_path = resolve_companion_booster_path(&pgn_path);

    let t0 = Instant::now();
    BoostIndexBuilder::build_for_pgn(&pgn_path, Some(booster_path.clone()), None)?;
    let build_time = t0.elapsed();
    println!("\n=== BOOSTER BENCHMARK (200,000 games, high branch entropy) ===");
    println!("Index build time: {:?}", build_time);

    let index = MmapBoostIndex::open(&booster_path)?;
    let evaluator = BoostSearchEvaluator::new(&index);

    let start_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    // 1. Move 0 Depth 8 Query
    let query_d8 = ContinuationQuery {
        position: start_fen.to_string(),
        max_depth: 8,
        min_games: 1,
        min_percentage: 0.0,
        max_lines: 50,
        hot_idx: None,
        pos_idx: None,
    };

    let iterations = 10;
    let t_start = Instant::now();
    for _ in 0..iterations {
        let res = evaluator.calculate_continuations(&query_d8, None, None::<fn(usize) -> _>)?;
        assert!(res.is_some());
    }
    let elapsed_d8 = t_start.elapsed() / iterations as u32;
    println!("Avg Move 0 (Depth 8, 50k games): {:?}", elapsed_d8);

    // 2. Move 0 Depth 4 Query
    let query_d4 = ContinuationQuery {
        position: start_fen.to_string(),
        max_depth: 4,
        min_games: 1,
        min_percentage: 0.0,
        max_lines: 50,
        hot_idx: None,
        pos_idx: None,
    };

    let t_start = Instant::now();
    for _ in 0..iterations {
        let res = evaluator.calculate_continuations(&query_d4, None, None::<fn(usize) -> _>)?;
        assert!(res.is_some());
    }
    let elapsed_d4 = t_start.elapsed() / iterations as u32;
    println!("Avg Move 0 (Depth 4, 50k games): {:?}", elapsed_d4);

    // 3. Opening Tree at Move 0 with continuations
    let t_start = Instant::now();
    for _ in 0..iterations {
        let res = evaluator.calculate_opening_tree(
            start_fen,
            None,
            Some(50),
            None::<fn(usize) -> _>,
            Some(&query_d8),
        )?;
        assert!(res.is_some());
    }
    let elapsed_tree = t_start.elapsed() / iterations as u32;
    println!("Avg Opening Tree + Cont (Move 0, 50k games): {:?}", elapsed_tree);

    Ok(())
}
