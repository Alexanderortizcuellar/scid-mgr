use anyhow::Result;
use scid_mgr::db::{ScidDatabaseWrapper, ScidFormat};
use scid_mgr::search_booster::{
    resolve_companion_booster_path, BoostIndexBuilder, BoostMove, BoostSearchEvaluator,
    MmapBoostIndex,
};
use shakmaty::fen::Fen;
use shakmaty::{Chess, Move, Position, Role, Square};
use tempfile::tempdir;

const TEST_PGN: &str = r#"[Event "London Immortal Game"]
[Site "London"]
[Date "1851.06.21"]
[Round "1"]
[White "Adolf Anderssen"]
[Black "Lionel Kieseritzky"]
[Result "1-0"]

1. e4 e5 2. f4 exf4 3. Bc4 Qh4+ 4. Kf1 b5 5. Bxb5 Nf6 6. Nf3 Qh6 7. d3 Nh5 8. Nh4 Qg5 9. Nf5 c6 10. g4 Nf6 11. Rg1 cxb5 12. h4 Qg6 13. h5 Qg5 14. Qf3 Ng8 15. Bxf4 Qf6 16. Nc3 Bc5 17. Nd5 Qxb2 18. Bd6 Bxg1 19. e5 Qxa1+ 20. Ke2 Na6 21. Nxg7+ Kd8 22. Qf6+ Nxf6 23. Be7# 1-0

[Event "Paris Opera"]
[Site "Paris"]
[Date "1858.11.02"]
[Round "1"]
[White "Paul Morphy"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0

[Event "Sicilian Defense"]
[Site "CyberSpace"]
[Date "2024.01.15"]
[Round "1"]
[White "Player White"]
[Black "Player Black"]
[Result "1/2-1/2"]

1. e4 c5 2. Nf3 d6 3. d4 cxb4 4. Nxd4 Nf6 5. Nc3 a6 6. Be2 e6 7. O-O Be7 8. f4 O-O 1/2-1/2

[Event "Promotion & En Passant Test"]
[Site "TestLab"]
[Date "2026.01.01"]
[Round "1"]
[White "Tester W"]
[Black "Tester B"]
[Result "*"]

1. e4 d5 2. e5 f5 3. exf6 e5 4. f7+ Ke7 5. fxg8=N+ Rxg8 6. d4 e4 7. Bg5+ Ke8 *
"#;

const GAME_1: &str = r#"[Event "London Immortal Game"]
[Site "London"]
[Date "1851.06.21"]
[Round "1"]
[White "Adolf Anderssen"]
[Black "Lionel Kieseritzky"]
[Result "1-0"]

1. e4 e5 2. f4 exf4 3. Bc4 Qh4+ 4. Kf1 b5 5. Bxb5 Nf6 6. Nf3 Qh6 7. d3 Nh5 8. Nh4 Qg5 9. Nf5 c6 10. g4 Nf6 11. Rg1 cxb5 12. h4 Qg6 13. h5 Qg5 14. Qf3 Ng8 15. Bxf4 Qf6 16. Nc3 Bc5 17. Nd5 Qxb2 18. Bd6 Bxg1 19. e5 Qxa1+ 20. Ke2 Na6 21. Nxg7+ Kd8 22. Qf6+ Nxf6 23. Be7# 1-0
"#;

const GAME_2: &str = r#"[Event "Paris Opera"]
[Site "Paris"]
[Date "1858.11.02"]
[Round "1"]
[White "Paul Morphy"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0
"#;

const GAME_3: &str = r#"[Event "Sicilian Defense"]
[Site "CyberSpace"]
[Date "2024.01.15"]
[Round "1"]
[White "Player White"]
[Black "Player Black"]
[Result "1/2-1/2"]

1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6 6. Be2 e6 7. O-O Be7 8. f4 O-O 1/2-1/2
"#;

const GAME_4: &str = r#"[Event "Promotion & En Passant Test"]
[Site "TestLab"]
[Date "2026.01.01"]
[Round "1"]
[White "Tester W"]
[Black "Tester B"]
[Result "*"]

1. e4 d5 2. e5 f5 3. exf6 e5 4. f7+ Ke7 5. fxg8=N+ Rxg8 6. d4 e4 7. Bg5+ Ke8 *
"#;

#[test]
fn test_boost_move_encoding_and_flags() {
    // Normal move: e2 -> e4 (Quiet double pawn push: flag = 1)
    let mv_quiet_push = Move::Normal {
        role: Role::Pawn,
        from: Square::E2,
        to: Square::E4,
        capture: None,
        promotion: None,
    };
    let bm_quiet_push = BoostMove::from_shakmaty(&mv_quiet_push);
    assert_eq!(bm_quiet_push.from(), Square::E2 as usize);
    assert_eq!(bm_quiet_push.to(), Square::E4 as usize);
    assert_eq!(bm_quiet_push.flags(), 1);
    assert!(bm_quiet_push.is_double_pawn_push());
    assert!(!bm_quiet_push.is_capture());
    assert!(!bm_quiet_push.is_promotion());
    assert!(!bm_quiet_push.is_castle());
    assert!(!bm_quiet_push.is_en_passant());

    // Single step quiet: e2 -> e3 (flag = 0)
    let mv_quiet_step = Move::Normal {
        role: Role::Pawn,
        from: Square::E2,
        to: Square::E3,
        capture: None,
        promotion: None,
    };
    let bm_quiet_step = BoostMove::from_shakmaty(&mv_quiet_step);
    assert_eq!(bm_quiet_step.flags(), 0);
    assert!(bm_quiet_step.is_quiet());

    // Capture: exd5 (flag = 4)
    let mv_cap = Move::Normal {
        role: Role::Pawn,
        from: Square::E4,
        to: Square::D5,
        capture: Some(Role::Pawn),
        promotion: None,
    };
    let bm_cap = BoostMove::from_shakmaty(&mv_cap);
    assert_eq!(bm_cap.from(), Square::E4 as usize);
    assert_eq!(bm_cap.to(), Square::D5 as usize);
    assert_eq!(bm_cap.flags(), 4);
    assert!(bm_cap.is_capture());
    assert!(!bm_cap.is_promotion());

    // En Passant (flag = 5)
    let mv_ep = Move::EnPassant {
        from: Square::E5,
        to: Square::F6,
    };
    let bm_ep = BoostMove::from_shakmaty(&mv_ep);
    assert_eq!(bm_ep.from(), Square::E5 as usize);
    assert_eq!(bm_ep.to(), Square::F6 as usize);
    assert_eq!(bm_ep.flags(), 5);
    assert!(bm_ep.is_capture());
    assert!(bm_ep.is_en_passant());

    // Castles King-side (flag = 2)
    let mv_castle = Move::Castle {
        king: Square::E1,
        rook: Square::H1,
    };
    let bm_castle = BoostMove::from_shakmaty(&mv_castle);
    assert_eq!(bm_castle.from(), Square::E1 as usize);
    assert_eq!(bm_castle.to(), Square::G1 as usize);
    assert_eq!(bm_castle.flags(), 2);
    assert!(bm_castle.is_castle());
    assert!(bm_castle.is_castle_kingside());

    // Queen Promotion Quiet (flag = 11)
    let mv_q_prom = Move::Normal {
        role: Role::Pawn,
        from: Square::E7,
        to: Square::E8,
        capture: None,
        promotion: Some(Role::Queen),
    };
    let bm_q_prom = BoostMove::from_shakmaty(&mv_q_prom);
    assert_eq!(bm_q_prom.flags(), 11);
    assert!(bm_q_prom.is_promotion());
    assert!(!bm_q_prom.is_capture());
    assert_eq!(bm_q_prom.promotion_role(), Some(Role::Queen));

    // Knight Promotion Capture (flag = 12: base 8 + 4)
    let mv_n_cap_prom = Move::Normal {
        role: Role::Pawn,
        from: Square::F7,
        to: Square::G8,
        capture: Some(Role::Knight),
        promotion: Some(Role::Knight),
    };
    let bm_n_cap_prom = BoostMove::from_shakmaty(&mv_n_cap_prom);
    assert_eq!(bm_n_cap_prom.flags(), 12);
    assert!(bm_n_cap_prom.is_promotion());
    assert!(bm_n_cap_prom.is_capture());
    assert_eq!(bm_n_cap_prom.promotion_role(), Some(Role::Knight));
}

#[test]
fn test_boost_index_build_and_mmap_pgn() -> Result<()> {
    let dir = tempdir()?;
    let pgn_path = dir.path().join("games.pgn");
    std::fs::write(&pgn_path, TEST_PGN)?;

    let booster_path = resolve_companion_booster_path(&pgn_path);
    assert!(booster_path.to_string_lossy().ends_with("games.boost.idx"));

    // Build booster index
    let (_, game_count, move_count, _) =
        BoostIndexBuilder::build_for_pgn(&pgn_path, Some(booster_path.clone()), None)?;
    assert_eq!(game_count, 4);
    assert!(move_count > 100);

    // Open index via memory mapping
    let index = MmapBoostIndex::open(&booster_path)?;
    assert_eq!(index.num_games(), 4);
    assert_eq!(index.total_moves(), move_count);

    // Verify game entries
    for g in 0..4 {
        let entry = index.game_entry(g).expect("game entry exists");
        let moves = index.game_moves(g).expect("game moves exists");
        assert_eq!(entry.ply_count as usize, moves.len());
        assert!(!moves.is_empty());
    }

    // Verify game 0 (Immortal game: 1. e4 e5 2. f4 exf4 ...)
    let moves_g0 = index.game_moves(0).unwrap();
    assert_eq!(moves_g0[0].from(), Square::E2 as usize);
    assert_eq!(moves_g0[0].to(), Square::E4 as usize);
    assert_eq!(moves_g0[1].from(), Square::E7 as usize);
    assert_eq!(moves_g0[1].to(), Square::E5 as usize);
    assert_eq!(moves_g0[2].from(), Square::F2 as usize);
    assert_eq!(moves_g0[2].to(), Square::F4 as usize);
    assert_eq!(moves_g0[3].from(), Square::E5 as usize);
    assert_eq!(moves_g0[3].to(), Square::F4 as usize);
    assert!(moves_g0[3].is_capture());

    Ok(())
}

#[test]
fn test_boost_search_evaluator_positions() -> Result<()> {
    let dir = tempdir()?;
    let pgn_path = dir.path().join("sample_search.pgn");
    std::fs::write(&pgn_path, TEST_PGN)?;

    let booster_path = resolve_companion_booster_path(&pgn_path);
    BoostIndexBuilder::build_for_pgn(&pgn_path, Some(booster_path.clone()), None)?;

    let index = MmapBoostIndex::open(&booster_path)?;
    let evaluator = BoostSearchEvaluator::new(&index);

    // 1. Search starting position FEN -> all 4 games match at ply 0
    let start_pos_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let matches = evaluator.search_position(start_pos_fen, None)?;
    assert_eq!(matches.len(), 4);
    for m in &matches {
        assert!(m.matching_plies.contains(&0));
    }

    // 2. Search 1. e4 e5 position -> Immortal (0), Opera (1)
    let e4_e5_fen = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2";
    let e4_e5_matches = evaluator.search_position(e4_e5_fen, None)?;
    assert!(e4_e5_matches
        .iter()
        .any(|m| m.game_id == 0 && m.matching_plies.contains(&2)));
    assert!(e4_e5_matches
        .iter()
        .any(|m| m.game_id == 1 && m.matching_plies.contains(&2)));
    assert!(!e4_e5_matches.iter().any(|m| m.game_id == 2)); // Game 2 is Sicilian (1. e4 c5)

    // 3. Search Sicilian 1. e4 c5 position:
    let sicilian_fen = "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq c6 0 2";
    let sicilian_matches = evaluator.search_position(sicilian_fen, None)?;
    assert_eq!(sicilian_matches.len(), 1);
    assert_eq!(sicilian_matches[0].game_id, 2);
    assert_eq!(sicilian_matches[0].matching_plies, vec![2]);

    // 4. Search Opera Game final mate position:
    let mut sim_pos = Chess::default();
    let move_strs = [
        "e4", "e5", "Nf3", "d6", "d4", "Bg4", "dxe5", "Bxf3", "Qxf3", "dxe5", "Bc4", "Nf6", "Qb3",
        "Qe7", "Nc3", "c6", "Bg5", "b5", "Nxb5", "cxb5", "Bxb5+", "Nbd7", "O-O-O", "Rd8", "Rxd7",
        "Rxd7", "Rd1", "Qe6", "Bxd7+", "Nxd7", "Qb8+", "Nxb8", "Rd8#",
    ];
    for san_str in move_strs {
        let san: shakmaty::san::San = san_str.parse().unwrap();
        let m = san.to_move(&sim_pos).unwrap();
        sim_pos.play_unchecked(&m);
    }
    let mate_fen = format!(
        "{}",
        Fen::from_position(sim_pos.clone(), shakmaty::EnPassantMode::Legal)
    );
    let mate_matches = evaluator.search_position(&mate_fen, None)?;
    assert_eq!(mate_matches.len(), 1);
    assert_eq!(mate_matches[0].game_id, 1);
    assert_eq!(mate_matches[0].matching_plies, vec![33]);

    // 5. Search non-existent position -> 0 matches
    let non_existent_fen = "8/8/8/8/8/8/8/4K2k w - - 0 1";
    let zero_matches = evaluator.search_position(non_existent_fen, None)?;
    assert_eq!(zero_matches.len(), 0);

    Ok(())
}

#[test]
fn test_boost_index_build_and_search_scid() -> Result<()> {
    let dir = tempdir()?;
    let db_base = dir.path().join("test_scid.si4");
    let mut scid_db = ScidDatabaseWrapper::create(&db_base, ScidFormat::Si4)?;

    // Add games
    scid_db.add_game(GAME_1)?;
    scid_db.add_game(GAME_2)?;
    scid_db.add_game(GAME_3)?;
    scid_db.add_game(GAME_4)?;
    scid_db.save()?;
    assert_eq!(scid_db.game_count(), 4);

    let booster_path = resolve_companion_booster_path(&db_base);

    let (_, num_games, total_moves, _) =
        BoostIndexBuilder::build_for_scid(&scid_db, Some(booster_path.clone()), None)?;
    assert_eq!(num_games, 4);
    assert!(total_moves > 100);

    let index = MmapBoostIndex::open(&booster_path)?;
    assert_eq!(index.num_games(), 4);

    let evaluator = BoostSearchEvaluator::new(&index);
    let start_pos_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let matches = evaluator.search_position(start_pos_fen, None)?;
    assert_eq!(matches.len(), 4);

    Ok(())
}

#[test]
fn test_boost_evaluator_opening_tree_and_continuations() -> Result<()> {
    let dir = tempdir()?;
    let pgn_path = dir.path().join("games.pgn");
    std::fs::write(&pgn_path, TEST_PGN)?;

    let booster_path = resolve_companion_booster_path(&pgn_path);
    BoostIndexBuilder::build_for_pgn(&pgn_path, Some(booster_path.clone()), None)?;

    let index = MmapBoostIndex::open(&booster_path)?;
    let evaluator = BoostSearchEvaluator::new(&index);

    // Mock metadata provider for the 4 games:
    // Game 0: London Immortal (1-0, 2600 / 2500, 1851)
    // Game 1: Paris Opera (1-0, 2700 / 2400, 1858)
    // Game 2: Sicilian (1/2-1/2, 2300 / 2300, 2024)
    // Game 3: Promo (*)
    let meta_lookup = |gid: usize| -> Option<scid_mgr::search_booster::BoostGameMeta> {
        match gid {
            0 => Some(scid_mgr::search_booster::BoostGameMeta::new(
                1,
                2600,
                2500,
                Some(1851),
            )),
            1 => Some(scid_mgr::search_booster::BoostGameMeta::new(
                1,
                2700,
                2400,
                Some(1858),
            )),
            2 => Some(scid_mgr::search_booster::BoostGameMeta::new(
                3,
                2300,
                2300,
                Some(2024),
            )),
            3 => Some(scid_mgr::search_booster::BoostGameMeta::new(
                0,
                0,
                0,
                Some(2026),
            )),
            _ => None,
        }
    };

    // 1. Opening Tree for starting position (without continuations)
    let tree_rep = evaluator
        .calculate_opening_tree("", None, Some(10), Some(meta_lookup), None)?
        .expect("Tree report should exist for start position");

    assert_eq!(tree_rep.total_games, 4);
    assert_eq!(tree_rep.white_wins, 2);
    assert_eq!(tree_rep.draws, 1);
    assert_eq!(tree_rep.moves.len(), 1); // Only 1. e4 was played in all 4 games
    assert!(tree_rep.continuations.is_none());

    let e4_move = &tree_rep.moves[0];
    assert_eq!(e4_move.san, "e4");
    assert_eq!(e4_move.uci, "e2e4");
    assert_eq!(e4_move.total_games, 4);
    assert_eq!(e4_move.white_wins, 2);
    assert_eq!(e4_move.draws, 1);
    assert_eq!(e4_move.white_pct, 50.0);
    assert_eq!(e4_move.draw_pct, 25.0);
    assert_eq!(e4_move.black_pct, 0.0);
    assert_eq!(e4_move.avg_white_elo, Some(2533)); // (2600 + 2700 + 2300) / 3 = 2533
    assert_eq!(e4_move.first_year, Some(1851));
    assert_eq!(e4_move.last_year, Some(2026));
    assert_eq!(e4_move.last_played, Some("2026".to_string()));

    // 2. Opening Tree after 1. e4 with single-pass continuations
    let after_e4_fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";
    let cont_q = scid_mgr::continuation_index::ContinuationQuery {
        position: after_e4_fen.to_string(),
        max_depth: 4,
        max_lines: 5,
        min_games: 1,
        min_percentage: 0.0,
        hot_idx: None,
        pos_idx: None,
    };

    let e4_tree = evaluator
        .calculate_opening_tree(
            after_e4_fen,
            None,
            Some(10),
            Some(meta_lookup),
            Some(&cont_q),
        )?
        .expect("Tree report should exist after 1. e4");

    assert_eq!(e4_tree.total_games, 4);
    assert_eq!(e4_tree.moves.len(), 3); // e5 (2), c5 (1), d5 (1)
    assert_eq!(e4_tree.moves[0].san, "e5");
    assert_eq!(e4_tree.moves[0].total_games, 2);
    assert_eq!(e4_tree.moves[0].white_wins, 2);
    assert_eq!(e4_tree.moves[0].white_pct, 100.0);
    assert_eq!(e4_tree.moves[0].first_year, Some(1851));
    assert_eq!(e4_tree.moves[0].last_year, Some(1858));
    assert!(e4_tree.continuations.is_some());
    let cont_lines = e4_tree.continuations.as_ref().unwrap();
    assert!(!cont_lines.is_empty());
    assert!(cont_lines[0].last_year.is_some());

    // 3. Dynamic Continuation Queries
    let query = scid_mgr::continuation_index::ContinuationQuery {
        position: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string(),
        max_depth: 4,
        max_lines: 5,
        min_games: 1,
        min_percentage: 0.0,
        hot_idx: None,
        pos_idx: None,
    };

    let cont_res = evaluator
        .calculate_continuations(&query, None, Some(meta_lookup))?
        .expect("Continuation result should exist");

    assert_eq!(cont_res.total_games_processed, 4);
    assert_eq!(cont_res.games_reaching_position, 4);
    assert!(!cont_res.lines.is_empty());

    // Check top continuation line (e4)
    let top_line = &cont_res.lines[0];
    assert!(top_line.moves[0] == "e4");
    assert!(top_line.games >= 1);
    assert_eq!(top_line.last_year, Some(2024));

    Ok(())
}

#[test]
fn test_server_booster_json_rpc() -> Result<()> {
    let dir = tempdir()?;
    let pgn_path = dir.path().join("server_booster_test.pgn");
    std::fs::write(&pgn_path, TEST_PGN)?;

    let mut current_db = None;
    let mut current_pos_index = None;
    let mut current_tree_index = None;
    let mut session_mgr = scid_mgr::server::search_session::SearchSessionManager::new();
    let thread_pool = rayon::ThreadPoolBuilder::new().num_threads(2).build()?;

    // 1. Open database via RPC
    let open_req = scid_mgr::server::RequestMessage {
        id: Some(1),
        command: "open".to_string(),
        params: serde_json::json!({ "path": pgn_path.to_str().unwrap() }),
    };
    let open_resp = scid_mgr::server::handlers::db::handle_open_db(
        &open_req,
        &mut current_db,
        &mut current_pos_index,
        &mut current_tree_index,
    );
    assert_eq!(open_resp.status, "ok");
    let open_data = open_resp.data.unwrap();
    assert_eq!(open_data["booster_index_status"], "missing");

    // 2. Build Booster via RPC
    let build_req = scid_mgr::server::RequestMessage {
        id: Some(2),
        command: "build_booster".to_string(),
        params: serde_json::json!({}),
    };
    let build_resp =
        scid_mgr::server::handlers::index::handle_build_booster(&build_req, &current_db);
    assert_eq!(build_resp.status, "ok");
    let build_data = build_resp.data.unwrap();
    assert_eq!(build_data["status"], "valid");
    assert_eq!(build_data["games_indexed"], 4);

    // 3. Check Booster Status via RPC
    let status_req = scid_mgr::server::RequestMessage {
        id: Some(3),
        command: "booster_status".to_string(),
        params: serde_json::json!({}),
    };
    let status_resp =
        scid_mgr::server::handlers::index::handle_booster_status(&status_req, &current_db);
    assert_eq!(status_resp.status, "ok");
    let status_data = status_resp.data.unwrap();
    assert_eq!(status_data["status"], "valid");

    // 4. Position Search via RPC (should auto-use booster)
    let pos_req = scid_mgr::server::RequestMessage {
        id: Some(4),
        command: "search_position".to_string(),
        params: serde_json::json!({
            "fen": "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        }),
    };
    let pos_resp = scid_mgr::server::handlers::position::handle_search_position(
        &pos_req,
        &current_db,
        &mut current_pos_index,
        &mut session_mgr,
        &thread_pool,
    );
    assert_eq!(pos_resp.status, "ok");
    let pos_data = pos_resp.data.unwrap();
    assert_eq!(pos_data["engine"], "search_booster");
    assert_eq!(pos_data["matched_count"], 4);

    // 5. Opening Tree via RPC (should auto-use booster)
    let tree_req = scid_mgr::server::RequestMessage {
        id: Some(5),
        command: "opening_tree".to_string(),
        params: serde_json::json!({
            "fen": "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        }),
    };
    let cancel_token = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let tree_resp = scid_mgr::server::handlers::tree::handle_opening_tree(
        &tree_req,
        &current_db,
        &mut current_pos_index,
        &mut current_tree_index,
        &cancel_token,
    );
    assert_eq!(tree_resp.status, "ok");
    let tree_data = tree_resp.data.unwrap();
    assert_eq!(tree_data["total_games"], 4);
    assert_eq!(tree_data["moves"][0]["san"], "e4");

    // 6. Continuations via RPC (should auto-use booster)
    let cont_req = scid_mgr::server::RequestMessage {
        id: Some(6),
        command: "continuations".to_string(),
        params: serde_json::json!({
            "fen": "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "max_depth": 4
        }),
    };
    let cont_resp = scid_mgr::server::handlers::continuations::handle_continuations(
        &cont_req,
        &current_db,
        &mut current_pos_index,
        &cancel_token,
    );
    assert_eq!(cont_resp.status, "ok");
    let cont_data = cont_resp.data.unwrap();
    assert_eq!(cont_data["total_games_processed"], 4);
    assert!(!cont_data["lines"].as_array().unwrap().is_empty());

    Ok(())
}

#[test]
fn test_custom_fen_games_excluded_from_fast_evaluator() -> Result<()> {
    let dir = tempdir()?;
    let pgn_path = dir.path().join("custom_fen_test.pgn");

    let pgn_content = r#"[Event "Normal Standard Game"]
[Site "London"]
[Date "2024.01.01"]
[Round "1"]
[White "Player A"]
[Black "Player B"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 1-0

[Event "Custom FEN Puzzle Game"]
[Site "Online"]
[Date "2024.01.02"]
[Round "2"]
[White "Puzzle W"]
[Black "Puzzle B"]
[Result "1-0"]
[FEN "r1bqk2r/pppp1ppp/2n5/4p3/2B1n3/2P2N2/PPP2PPP/R1BQK2R w KQkq - 0 7"]

7. Bxf7+ Kxf7 8. Qd5+ Ke8 9. Qxe4 1-0
"#;

    std::fs::write(&pgn_path, pgn_content)?;
    let booster_path = resolve_companion_booster_path(&pgn_path);

    BoostIndexBuilder::build_for_pgn(&pgn_path, Some(booster_path.clone()), None)?;
    let index = MmapBoostIndex::open(&booster_path)?;

    assert_eq!(index.num_games(), 2);

    let entry0 = index.game_entry(0).unwrap();
    assert!(!entry0.is_custom_fen());

    let entry1 = index.game_entry(1).unwrap();
    assert!(entry1.is_custom_fen());

    let evaluator = BoostSearchEvaluator::new(&index);

    // Opening tree at starting position should only aggregate the 1 standard game
    let start_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let report = evaluator
        .calculate_opening_tree(start_fen, None, Some(50), None::<fn(usize) -> _>, None)?
        .expect("Tree report exists");
    assert_eq!(report.total_games, 1);
    assert_eq!(report.moves.len(), 1);
    assert_eq!(report.moves[0].san, "e4");

    // Board search for starting position should match exactly 1 game (game 0)
    let matches = evaluator.search_position(start_fen, None)?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].game_id, 0);

    // Continuation query from start position should only process the standard game
    let query = scid_mgr::continuation_index::ContinuationQuery {
        position: start_fen.to_string(),
        max_depth: 4,
        min_games: 1,
        min_percentage: 0.0,
        max_lines: 10,
        hot_idx: None,
        pos_idx: None,
    };
    let cont_res = evaluator
        .calculate_continuations(&query, None, None::<fn(usize) -> _>)?
        .expect("Continuation result exists");
    assert_eq!(cont_res.games_reaching_position, 1);

    Ok(())
}

#[test]
fn test_packed_path_256_encoding_equality_and_hashing() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn compute_hash<T: Hash>(item: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        hasher.finish()
    }

    let m1 = BoostMove(100);
    let m2 = BoostMove(200);
    let m3 = BoostMove(300);
    let m4 = BoostMove(400);

    let path_a = scid_mgr::search_booster::PackedPath256::from_slice(&[m1, m2, m3]);
    let path_b = scid_mgr::search_booster::PackedPath256::from_slice(&[m1, m2, m3]);
    let path_prefix = scid_mgr::search_booster::PackedPath256::from_slice(&[m1, m2]);
    let path_diff = scid_mgr::search_booster::PackedPath256::from_slice(&[m1, m2, m4]);

    assert_eq!(path_a, path_b);
    assert_ne!(path_a, path_prefix);
    assert_ne!(path_a, path_diff);

    assert_eq!(compute_hash(&path_a), compute_hash(&path_b));
    assert_ne!(compute_hash(&path_a), compute_hash(&path_prefix));
    assert_ne!(compute_hash(&path_a), compute_hash(&path_diff));

    assert_eq!(path_a.len(), 3);
    assert_eq!(path_a.to_boost_moves(), vec![m1, m2, m3]);
    assert_eq!(path_prefix.to_boost_moves(), vec![m1, m2]);

    // Test up to 16 moves (across both lo and hi 128-bit words)
    let sixteen_moves: Vec<BoostMove> = (1..=16).map(BoostMove).collect();
    let packed_16 = scid_mgr::search_booster::PackedPath256::from_slice(&sixteen_moves);
    assert_eq!(packed_16.len(), 16);
    assert_eq!(packed_16.to_boost_moves(), sixteen_moves);
}

#[test]
fn test_booster_language_search_adapter_capability_checks() {
    use cql_lang::parser::QueryParser;
    use scid_mgr::search_booster::BoosterLanguageSearchAdapter;

    // Supported queries
    let q_pos = QueryParser::parse_str("piece wp on e4 and piece bp on e5").unwrap();
    assert!(BoosterLanguageSearchAdapter::can_booster_evaluate(&q_pos));

    let q_mat = QueryParser::parse_str("white_queens == 1 and opposite_bishops").unwrap();
    assert!(BoosterLanguageSearchAdapter::can_booster_evaluate(&q_mat));

    let q_sym = QueryParser::parse_str("flip_vertical piece wp on e4").unwrap();
    assert!(BoosterLanguageSearchAdapter::can_booster_evaluate(&q_sym));

    let q_ply = QueryParser::parse_str("ply 1..20 and turn white").unwrap();
    assert!(BoosterLanguageSearchAdapter::can_booster_evaluate(&q_ply));

    let q_header = QueryParser::parse_str(r#"white "Morphy" and result "1-0""#).unwrap();
    assert!(BoosterLanguageSearchAdapter::can_booster_evaluate(
        &q_header
    ));

    // Unsupported queries requiring fallback
    let q_comment = QueryParser::parse_str(r#"comment "blunder""#).unwrap();
    assert!(!BoosterLanguageSearchAdapter::can_booster_evaluate(
        &q_comment
    ));

    let q_what_if = QueryParser::parse_str("what_if [pass] { check }").unwrap();
    assert!(!BoosterLanguageSearchAdapter::can_booster_evaluate(
        &q_what_if
    ));

    let q_mate = QueryParser::parse_str("checkmate").unwrap();
    assert!(!BoosterLanguageSearchAdapter::can_booster_evaluate(&q_mate));
}

#[test]
fn test_booster_language_search_adapter_eval_game() -> Result<()> {
    use cql_lang::parser::QueryParser;
    use scid_mgr::search_booster::BoosterLanguageSearchAdapter;

    let dir = tempdir()?;
    let pgn_file = dir.path().join("test.pgn");
    std::fs::write(&pgn_file, TEST_PGN)?;

    let (boost_path, game_count, _plies, _time) =
        BoostIndexBuilder::build_for_pgn(&pgn_file, None, None)?;
    assert_eq!(game_count, 4);

    let boost_idx = MmapBoostIndex::open(&boost_path)?;

    // 1. Search starting position (should match all games at ply 0)
    let q1 = QueryParser::parse_str("ply == 0").unwrap();
    for gid in 0..4 {
        let moves = boost_idx.get_game_moves(gid).unwrap();
        let entry = boost_idx.get_game_entry(gid).unwrap();
        let res =
            BoosterLanguageSearchAdapter::evaluate_booster_game(&q1, moves, entry.result, None);
        assert!(res.is_match);
        assert_eq!(res.matching_plies, vec![0]);
    }

    // 2. Search for 1. e4 e5 (should match Anderssen and Morphy games)
    let q_e4_e5 = QueryParser::parse_str("piece wp on e4 and piece bp on e5").unwrap();
    let moves_g1 = boost_idx.get_game_moves(0).unwrap();
    let entry_g1 = boost_idx.get_game_entry(0).unwrap();
    let res_g1 = BoosterLanguageSearchAdapter::evaluate_booster_game(
        &q_e4_e5,
        moves_g1,
        entry_g1.result,
        None,
    );
    assert!(res_g1.is_match);

    // 3. Search for material predicate (queen sacrifice: white queen == 0)
    let q_no_wq = QueryParser::parse_str("white_queens == 0").unwrap();
    let res_sac = BoosterLanguageSearchAdapter::evaluate_booster_game(
        &q_no_wq,
        moves_g1,
        entry_g1.result,
        None,
    );
    assert!(res_sac.is_match);

    Ok(())
}

#[test]
fn test_pgn_missing_event_headers_booster_and_integrity_check() -> Result<()> {
    use scid_mgr::cli::commands::check::handle_check;
    use scid_mgr::pgn_db::PgnDatabaseWrapper;

    let dir = tempdir()?;
    let pgn_file = dir.path().join("mixed_headers.pgn");

    // PGN with 3 games: Game 1 starts with [Site], Game 2 with [White], Game 3 with [Event]
    let pgn_content = r#"[Site "Nagykanizsa HUN"]
[Date "2026.08.15"]
[Round "1.1"]
[White "Golis,Wiktor"]
[Black "Bluebaum,Alexander"]
[Result "1-0"]

1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6 1-0

[White "Carlsen, Magnus"]
[Black "Nakamura, Hikaru"]
[Date "2026.08.16"]
[Result "1/2-1/2"]

1. d4 Nf6 2. c4 e6 3. Nf3 d5 1/2-1/2

[Event "World Cup"]
[Site "Sochi"]
[White "Duda, Jan-Krzysztof"]
[Black "Karjakin, Sergey"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 1-0
"#;

    std::fs::write(&pgn_file, pgn_content)?;

    // 1. Open PgnDatabaseWrapper and verify game count
    let pgn_db = PgnDatabaseWrapper::open(&pgn_file)?;
    assert_eq!(pgn_db.game_count(), 3);

    // 2. Build Booster Index
    let (boost_path, indexed_games, _plies, _time) =
        BoostIndexBuilder::build_for_pgn(&pgn_file, None, None)?;
    assert_eq!(indexed_games, 3);

    let boost_idx = MmapBoostIndex::open(&boost_path)?;
    assert_eq!(boost_idx.game_count(), 3);

    // 3. Verify move count & headers for all 3 games in booster
    let g1_moves = boost_idx.get_game_moves(0).unwrap();
    assert_eq!(g1_moves.len(), 10); // 1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6
    assert_eq!(boost_idx.get_game_entry(0).unwrap().result, 1);

    let g2_moves = boost_idx.get_game_moves(1).unwrap();
    assert_eq!(g2_moves.len(), 6); // 1. d4 Nf6 2. c4 e6 3. Nf3 d5
    assert_eq!(boost_idx.get_game_entry(1).unwrap().result, 3);

    let g3_moves = boost_idx.get_game_moves(2).unwrap();
    assert_eq!(g3_moves.len(), 6); // 1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5
    assert_eq!(boost_idx.get_game_entry(2).unwrap().result, 1);

    // 4. Run integrity check
    handle_check(&pgn_file, true, false)?;

    Ok(())
}
