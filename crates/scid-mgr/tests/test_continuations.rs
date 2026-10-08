use scid_mgr::continuation_index::{
    build_for_pgn, build_for_scid, calculate_continuations_for_pgn, ContinuationQuery,
    HotGraphBuildConfig, HotGraphQueryable, MmapHotGraph,
};
use scid_mgr::db::ScidDatabaseWrapper;
use std::io::Write;
use tempfile::NamedTempFile;

const SAMPLE_PGN: &str = r#"
[Event "Game 1"]
[Date "2024.01.10"]
[White "Player 1"]
[Black "Player 2"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7 1-0

[Event "Game 2"]
[Date "2024.02.15"]
[White "Player 3"]
[Black "Player 4"]
[Result "0-1"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 Nf6 4. O-O Be7 5. Re1 d6 0-1

[Event "Game 3"]
[Date "2025.03.01"]
[White "Player 5"]
[Black "Player 6"]
[Result "1/2-1/2"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7 1/2-1/2

[Event "Game 4"]
[Date "2025.04.12"]
[White "Player 1"]
[Black "Player 4"]
[Result "1-0"]

1. d4 d5 2. c4 e6 3. Nc3 Nf6 1-0

[Event "Game 5"]
[Date "2026.05.20"]
[White "Player 2"]
[Black "Player 3"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Bxc6 dxc6 1-0
"#;

#[test]
fn test_dynamic_pgn_starting_position_continuations() {
    let mut tmp = NamedTempFile::new().unwrap();
    tmp.write_all(SAMPLE_PGN.as_bytes()).unwrap();
    tmp.flush().unwrap();

    let query = ContinuationQuery {
        position: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string(),
        max_depth: 2,
        max_lines: 5,
        min_games: 1,
        min_percentage: 0.0,
        ..Default::default()
    };

    let result = calculate_continuations_for_pgn(tmp.path(), &query, None).unwrap();
    assert_eq!(result.total_games_processed, 5);
    assert_eq!(result.games_reaching_position, 5);
    assert!(!result.lines.is_empty());

    let e4_line = result
        .lines
        .iter()
        .find(|l| l.moves.first().map(|m| m.as_str()) == Some("e4"));
    assert!(e4_line.is_some());
    assert_eq!(e4_line.unwrap().games, 4);

    let d4_line = result
        .lines
        .iter()
        .find(|l| l.moves.first().map(|m| m.as_str()) == Some("d4"));
    assert!(d4_line.is_some());
    assert_eq!(d4_line.unwrap().games, 1);
}

#[test]
fn test_dynamic_pgn_ruy_lopez_branching() {
    let mut tmp = NamedTempFile::new().unwrap();
    tmp.write_all(SAMPLE_PGN.as_bytes()).unwrap();
    tmp.flush().unwrap();

    let query = ContinuationQuery {
        position: "r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3".to_string(),
        max_depth: 6,
        max_lines: 10,
        min_games: 1,
        min_percentage: 0.0,
        ..Default::default()
    };

    let result = calculate_continuations_for_pgn(tmp.path(), &query, None).unwrap();
    assert_eq!(result.total_games_processed, 5);
    assert_eq!(result.games_reaching_position, 4);

    let a6_lines: Vec<_> = result
        .lines
        .iter()
        .filter(|l| l.moves.first().map(|m| m.as_str()) == Some("a6"))
        .collect();
    assert!(!a6_lines.is_empty());
}

#[test]
fn test_pgn_hot_graph_index_build_and_query() {
    let mut tmp_pgn = tempfile::Builder::new().suffix(".pgn").tempfile().unwrap();
    tmp_pgn.write_all(SAMPLE_PGN.as_bytes()).unwrap();
    tmp_pgn.flush().unwrap();

    let tmp_hot = tempfile::Builder::new()
        .suffix(".hot.idx")
        .tempfile()
        .unwrap();

    let config = HotGraphBuildConfig {
        max_ply: 16,
        min_games: 1,
    };

    let meta = build_for_pgn(tmp_pgn.path(), tmp_hot.path(), config).unwrap();
    assert_eq!(meta.db_game_count, 5);
    assert!(meta.node_count > 0);
    assert!(meta.edge_count > 0);

    let mmap_hot = MmapHotGraph::open(tmp_hot.path()).unwrap();
    assert_eq!(mmap_hot.total_database_games(), 5);

    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let target_pos = shakmaty::fen::Fen::from_ascii(fen.as_bytes())
        .unwrap()
        .into_position(shakmaty::CastlingMode::Standard)
        .unwrap();

    let res = mmap_hot.query_continuations(&target_pos, fen, 2, 10, 1, 0.0);
    assert_eq!(res.total_games_processed, 5);
    assert_eq!(res.games_reaching_position, 5);
    assert!(!res.lines.is_empty());

    let e4_line = res
        .lines
        .iter()
        .find(|l| l.moves.first().map(|m| m.as_str()) == Some("e4"));
    assert!(e4_line.is_some());
    assert_eq!(e4_line.unwrap().games, 4);
}

#[test]
fn test_scid_hot_graph_index_build_and_query() {
    let scid_sample = std::path::Path::new("games/sample_games.pgn");
    if !scid_sample.exists() {
        return;
    }

    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test_scid.si5");

    let mut db = ScidDatabaseWrapper::create(&db_path, scid_mgr::db::ScidFormat::Si5).unwrap();
    db.add_game(SAMPLE_PGN).unwrap();
    db.save().unwrap();

    let hot_path = tmp_dir.path().join("test_scid.hot.idx");
    let config = HotGraphBuildConfig {
        max_ply: 20,
        min_games: 1,
    };

    let meta = build_for_scid(&db_path, &hot_path, config).unwrap();
    assert!(meta.node_count > 0);
    assert!(meta.edge_count > 0);

    let mmap_hot = MmapHotGraph::open(&hot_path).unwrap();
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let target_pos = shakmaty::fen::Fen::from_ascii(fen.as_bytes())
        .unwrap()
        .into_position(shakmaty::CastlingMode::Standard)
        .unwrap();

    let res = mmap_hot.query_continuations(&target_pos, fen, 6, 10, 1, 0.0);
    assert!(res.total_games_processed > 0);
    assert!(res.games_reaching_position > 0);
    assert!(!res.lines.is_empty());

    // Test opening tree calculation directly from .hot.idx nodes graph
    let tree_rep = mmap_hot.query_opening_tree(&target_pos, fen, None);
    assert!(tree_rep.is_some());
    let rep = tree_rep.unwrap();
    assert_eq!(rep.total_games, 5);
    assert!(!rep.moves.is_empty());
    let e4_move = rep.moves.iter().find(|m| m.san == "e4");
    assert!(e4_move.is_some());
    assert_eq!(e4_move.unwrap().total_games, 4);
    let d4_move = rep.moves.iter().find(|m| m.san == "d4");
    assert!(d4_move.is_some());
    assert_eq!(d4_move.unwrap().total_games, 1);
}

#[test]
fn test_booster_accelerated_hot_graph_build() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test_boost_scid.si5");

    let mut db = ScidDatabaseWrapper::create(&db_path, scid_mgr::db::ScidFormat::Si5).unwrap();
    for game_str in SAMPLE_PGN.split("[Event ").filter(|s| !s.trim().is_empty()) {
        let full_pgn = format!("[Event {}", game_str);
        db.add_game(&full_pgn).unwrap();
    }
    db.save().unwrap();
    assert_eq!(db.game_count(), 5);

    // 1. Build booster companion index
    let booster_path = tmp_dir.path().join("test_boost_scid.boost.idx");
    let (_, game_count, _, _) = scid_mgr::search_booster::BoostIndexBuilder::build_for_scid(
        &db,
        Some(booster_path.clone()),
        None,
    )
    .unwrap();
    assert_eq!(game_count, 5);

    // 2. Build hot graph - should automatically detect and use booster for fast ingestion
    let hot_path = tmp_dir.path().join("test_boost_scid.hot.idx");
    let config = HotGraphBuildConfig {
        max_ply: 20,
        min_games: 1,
    };
    let meta = build_for_scid(&db_path, &hot_path, config).unwrap();
    assert_eq!(meta.db_game_count, 5);
    assert!(meta.node_count > 0);

    let mmap_hot = MmapHotGraph::open(&hot_path).unwrap();
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let target_pos = shakmaty::fen::Fen::from_ascii(fen.as_bytes())
        .unwrap()
        .into_position(shakmaty::CastlingMode::Standard)
        .unwrap();

    let tree_rep = mmap_hot.query_opening_tree(&target_pos, fen, None).unwrap();
    assert_eq!(tree_rep.total_games, 5);
    assert_eq!(tree_rep.moves.len(), 2);
}

#[test]
fn test_booster_and_hot_graph_opening_tree_exact_match() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let pgn_path = tmp_dir.path().join("match_test.pgn");

    let pgn_content = r#"
[Event "Game 1"]
[Date "2020.01.15"]
[White "W1"]
[Black "B1"]
[Result "1-0"]

1. e4 c5 2. c3 Nf6 3. e5 Nd5 4. d4 cxd4 5. cxd4 d6 6. Nf3 Nc6 1-0

[Event "Game 2"]
[Date "2021.05.20"]
[White "W2"]
[Black "B2"]
[Result "0-1"]

1. e4 c5 2. c3 Nf6 3. e5 Nd5 4. d4 cxd4 5. cxd4 d6 6. Nf3 e6 0-1

[Event "Game 3"]
[Date "2022.08.10"]
[White "W3"]
[Black "B3"]
[Result "1/2-1/2"]

1. e4 c5 2. c3 Nf6 3. e5 Nd5 4. Nf3 Nc6 5. Bc4 Nb6 6. Bb3 c4 1/2-1/2

[Event "Game 4"]
[Date "2023.11.05"]
[White "W4"]
[Black "B4"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7 1-0

[Event "Game 5"]
[Date "2024.03.12"]
[White "W5"]
[Black "B5"]
[Result "0-1"]

1. d4 d5 2. c4 e6 3. Nc3 Nf6 0-1
"#;
    std::fs::write(&pgn_path, pgn_content).unwrap();

    // 1. Build booster index
    let booster_path = tmp_dir.path().join("match_test.boost.idx");
    let (_, game_count, _, _) = scid_mgr::search_booster::BoostIndexBuilder::build_for_pgn(
        &pgn_path,
        Some(booster_path.clone()),
        None,
    )
    .unwrap();
    assert_eq!(game_count, 5);

    // 2. Build hot graph index
    let hot_path = tmp_dir.path().join("match_test.hot.idx");
    let config = HotGraphBuildConfig {
        max_ply: 20,
        min_games: 1,
    };
    let meta = build_for_pgn(&pgn_path, &hot_path, config).unwrap();
    assert_eq!(meta.db_game_count, 5);

    let mmap_hot = MmapHotGraph::open(&hot_path).unwrap();
    let boost_idx = scid_mgr::search_booster::MmapBoostIndex::open(&booster_path).unwrap();
    let evaluator = scid_mgr::search_booster::BoostSearchEvaluator::new(&boost_idx);

    let pgn_db = scid_mgr::pgn_db::PgnDatabaseWrapper::open(&pgn_path).unwrap();
    let meta_lookup = |gid: usize| -> Option<scid_mgr::search_booster::BoostGameMeta> {
        pgn_db.entries.get(gid).map(|e| {
            let res = match e.result {
                1 => 1,
                2 => 2,
                3 => 3,
                _ => 0,
            };
            let year = {
                let y = (e.date >> 9) as u16;
                if y > 0 { Some(y) } else { None }
            };
            let month = {
                let m = ((e.date >> 5) & 0x0F) as u8;
                if (1..=12).contains(&m) { Some(m) } else { None }
            };
            scid_mgr::search_booster::BoostGameMeta::new(res, e.white_elo, e.black_elo, year, month)
        })
    };

    // Test positions to compare
    let test_positions = [
        // Starting position
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        // After 1. e4
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
        // Alapin Sicilian after 1. e4 c5 2. c3 Nf6
        "rnbqkb1r/pp1ppppp/5n2/2p5/4P3/2P5/PP1P1PPP/RNBQKBNR w KQkq - 1 3",
        // After 3. e5
        "rnbqkb1r/pp1ppppp/5n2/2p1P3/8/2P5/PP1P1PPP/RNBQKBNR b KQkq - 0 3",
        // After 3... Nd5
        "rnbqkb1r/pp1ppppp/8/2pnP3/8/2P5/PP1P1PPP/RNBQKBNR w KQkq - 1 4",
    ];

    for fen in test_positions {
        let target_pos = shakmaty::fen::Fen::from_ascii(fen.as_bytes())
            .unwrap()
            .into_position(shakmaty::CastlingMode::Standard)
            .unwrap();

        let hot_tree = mmap_hot.query_opening_tree(&target_pos, fen, None);
        let boost_tree = evaluator.calculate_opening_tree(fen, None, Some(10), Some(meta_lookup), None).unwrap();

        match (hot_tree, boost_tree) {
            (Some(ht), Some(bt)) => {
                assert_eq!(ht.total_games, bt.total_games, "total_games mismatch at FEN: {}", fen);
                assert_eq!(ht.white_wins, bt.white_wins, "white_wins mismatch at FEN: {}", fen);
                assert_eq!(ht.draws, bt.draws, "draws mismatch at FEN: {}", fen);
                assert_eq!(ht.black_wins, bt.black_wins, "black_wins mismatch at FEN: {}", fen);
                assert_eq!(ht.moves.len(), bt.moves.len(), "moves count mismatch at FEN: {}", fen);

                for (hm, bm) in ht.moves.iter().zip(bt.moves.iter()) {
                    assert_eq!(hm.san, bm.san, "Move san mismatch");
                    assert_eq!(hm.uci, bm.uci, "Move uci mismatch");
                    assert_eq!(hm.total_games, bm.total_games, "Move total_games mismatch for {}", hm.san);
                    assert_eq!(hm.white_wins, bm.white_wins, "Move white_wins mismatch for {}", hm.san);
                    assert_eq!(hm.draws, bm.draws, "Move draws mismatch for {}", hm.san);
                    assert_eq!(hm.black_wins, bm.black_wins, "Move black_wins mismatch for {}", hm.san);
                    assert_eq!(hm.first_year, bm.first_year, "Move first_year mismatch for {}", hm.san);
                    assert_eq!(hm.first_month, bm.first_month, "Move first_month mismatch for {}", hm.san);
                    assert_eq!(hm.last_year, bm.last_year, "Move last_year mismatch for {}", hm.san);
                    assert_eq!(hm.last_month, bm.last_month, "Move last_month mismatch for {}", hm.san);
                    assert_eq!(hm.last_played, bm.last_played, "Move last_played mismatch for {}", hm.san);
                }
            }
            (None, None) => {}
            _ => panic!("One index found position while other did not for FEN: {}", fen),
        }
    }
}

#[test]
fn test_master_pgn_booster_and_hot_graph_comparison() {
    let master_path = std::path::Path::new(r#"C:\Users\ASUS\programming\qt_programs\chess\twchess\data\master.pgn"#);
    if !master_path.exists() {
        return;
    }

    let booster_path = scid_mgr::search_booster::resolve_companion_booster_path(master_path);
    if !booster_path.exists() {
        return;
    }

    let hot_path = scid_mgr::continuation_index::resolve_companion_hot_path(master_path);
    let config = HotGraphBuildConfig {
        max_ply: 24,
        min_games: 1,
    };
    let meta = build_for_pgn(master_path, &hot_path, config).expect("Should build hot graph for master.pgn");
    assert!(meta.db_game_count > 0);

    let mmap_hot = MmapHotGraph::open(&hot_path).expect("Should open hot graph v2");
    let boost_idx = scid_mgr::search_booster::MmapBoostIndex::open(&booster_path).expect("Should open booster index");
    let evaluator = scid_mgr::search_booster::BoostSearchEvaluator::new(&boost_idx);

    let pgn_db = scid_mgr::pgn_db::PgnDatabaseWrapper::open(master_path).expect("Should open pgn db");
    let meta_lookup = |gid: usize| -> Option<scid_mgr::search_booster::BoostGameMeta> {
        pgn_db.entries.get(gid).map(|e| {
            let res = match e.result {
                1 => 1,
                2 => 2,
                3 => 3,
                _ => 0,
            };
            let year = {
                let y = (e.date >> 9) as u16;
                if y > 0 { Some(y) } else { None }
            };
            let month = {
                let m = ((e.date >> 5) & 0x0F) as u8;
                if (1..=12).contains(&m) { Some(m) } else { None }
            };
            scid_mgr::search_booster::BoostGameMeta::new(res, e.white_elo, e.black_elo, year, month)
        })
    };

    let target_fen = "rnbqkb1r/pp1ppppp/5n2/2p5/4P3/2P5/PP1P1PPP/RNBQKBNR w KQkq - 1 3";
    let target_pos = shakmaty::fen::Fen::from_ascii(target_fen.as_bytes())
        .unwrap()
        .into_position(shakmaty::CastlingMode::Standard)
        .unwrap();

    let hot_tree = mmap_hot.query_opening_tree(&target_pos, target_fen, None).expect("Hot graph tree should exist");
    let boost_tree = evaluator
        .calculate_opening_tree(target_fen, None, Some(10), Some(meta_lookup), None)
        .expect("Booster tree evaluation ok")
        .expect("Booster tree should exist");

    println!("HOT GRAPH TOTAL GAMES: {}", hot_tree.total_games);
    println!("BOOSTER TOTAL GAMES: {}", boost_tree.total_games);
    println!("HOT GRAPH MOVES ({} moves):", hot_tree.moves.len());
    for (i, m) in hot_tree.moves.iter().enumerate() {
        println!("  #{}: {} (games: {}, w:{}, d:{}, b:{}, first:{:?}, last:{:?})", i+1, m.san, m.total_games, m.white_wins, m.draws, m.black_wins, m.first_played, m.last_played);
    }
    println!("BOOSTER MOVES ({} moves):", boost_tree.moves.len());
    for (i, m) in boost_tree.moves.iter().enumerate() {
        println!("  #{}: {} (games: {}, w:{}, d:{}, b:{}, first:{:?}, last:{:?})", i+1, m.san, m.total_games, m.white_wins, m.draws, m.black_wins, m.first_played, m.last_played);
    }

    // 1. Hot Graph Node Index
    let hot_cont = mmap_hot.query_continuations(&target_pos, target_fen, 8, 10, 1, 0.0);
    let boost_query = ContinuationQuery {
        position: target_fen.to_string(),
        max_depth: 8,
        max_lines: 10,
        min_games: 1,
        min_percentage: 0.0,
        hot_idx: None,
        pos_idx: None,
    };
    
    // 2. Booster Index
    let boost_cont = evaluator
        .calculate_continuations(&boost_query, None, Some(meta_lookup))
        .expect("Booster continuations evaluation ok")
        .expect("Booster continuations should exist");

    println!("HOT GRAPH LINES ({} lines):", hot_cont.lines.len());
    for (i, l) in hot_cont.lines.iter().enumerate() {
        println!("  #{}: {} (games: {}, w:{}, d:{}, b:{})", i+1, l.formatted, l.games, l.white_wins, l.draws, l.black_wins);
    }
    println!("BOOSTER LINES ({} lines):", boost_cont.lines.len());
    for (i, l) in boost_cont.lines.iter().enumerate() {
        println!("  #{}: {} (games: {}, w:{}, d:{}, b:{})", i+1, l.formatted, l.games, l.white_wins, l.draws, l.black_wins);
    }

    // 3. Ground Truth Dynamic Engine (uses full Shakmaty Position + Zobrist hash with all castling/en-passant flags)
    let dyn_res = scid_mgr::continuation_index::calculate_continuations_for_pgn(
        master_path,
        &boost_query,
        None,
    ).expect("Dynamic continuations should succeed");

    println!("DYNAMIC ENGINE (GROUND TRUTH ZOBRIST) LINES ({} lines):", dyn_res.lines.len());
    for (i, l) in dyn_res.lines.iter().enumerate() {
        println!("  #{}: {} (games: {}, w:{}, d:{}, b:{})", i+1, l.formatted, l.games, l.white_wins, l.draws, l.black_wins);
    }


    // Verify that Search Booster and Ground Truth Dynamic Engine match 100%
    assert_eq!(boost_cont.lines.len(), dyn_res.lines.len(), "Line count mismatch between Booster and Dynamic engine");
    for (i, (bl, dl)) in boost_cont.lines.iter().zip(dyn_res.lines.iter()).enumerate() {
        assert_eq!(bl.formatted, dl.formatted, "Line #{} formatted mismatch between Booster and Ground truth", i+1);
        assert_eq!(bl.games, dl.games, "Line #{} games mismatch between Booster and Ground truth", i+1);
        assert_eq!(bl.white_wins, dl.white_wins, "Line #{} white_wins mismatch", i+1);
        assert_eq!(bl.draws, dl.draws, "Line #{} draws mismatch", i+1);
        assert_eq!(bl.black_wins, dl.black_wins, "Line #{} black_wins mismatch", i+1);
    }
}

