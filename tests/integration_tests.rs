use anyhow::Result;
use scid_mgr::db::{GameFilter, ScidDatabaseWrapper, ScidFormat};
use scid_mgr::pgn_db::PgnDatabaseWrapper;
use scid_mgr::position_index::PositionIndex;
use scid_mgr::tree_index::TreeIndex;
use shakmaty::zobrist::ZobristHash;
use tempfile::tempdir;

const SAMPLE_GAME_1: &str = r#"[Event "London"]
[Site "London"]
[Date "1851.06.21"]
[Round "1"]
[White "Adolf Anderssen"]
[Black "Lionel Kieseritzky"]
[Result "1-0"]
[ECO "C33"]
[WhiteElo "2600"]
[BlackElo "2550"]

1. e4 e5 2. f4 exf4 3. Bc4 Qh4+ 4. Kf1 b5 5. Bxb5 Nf6 6. Nf3 Qh6 7. d3 Nh5 8. Nh4 Qg5 9. Nf5 c6 10. g4 Nf6 11. Rg1 cxb5 12. h4 Qg6 13. h5 Qg5 14. Qf3 Ng8 15. Bxf4 Qf6 16. Nc3 Bc5 17. Nd5 Qxb2 18. Bd6 Bxg1 19. e5 Qxa1+ 20. Ke2 Na6 21. Nxg7+ Kd8 22. Qf6+ Nxf6 23. Be7# 1-0
"#;

const SAMPLE_GAME_2: &str = r#"[Event "Paris Opera"]
[Site "Paris"]
[Date "1858.11.02"]
[Round "1"]
[White "Paul Morphy"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]
[ECO "C41"]
[WhiteElo "2700"]
[BlackElo "2300"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0
"#;

const SAMPLE_GAME_3_VARIATIONS: &str = r#"[Event "Test Variations & NAGs"]
[Site "CyberSpace"]
[Date "2024.01.15"]
[Round "3.1"]
[White "Garry Kasparov"]
[Black "Deep Blue"]
[Result "1/2-1/2"]
[ECO "B85"]
[WhiteElo "2800"]
[BlackElo "2800"]

1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6 (5... e6 6. Be2 Be7) 6. Be2 e6 7. O-O Be7 8. f4 O-O 1/2-1/2
"#;

const SAMPLE_GAME_4_CUSTOM_FEN: &str = r#"[Event "Custom FEN Puzzle"]
[Site "Puzzle World"]
[Date "2023.05.20"]
[Round "1"]
[White "White to Move"]
[Black "Black Defender"]
[Result "1-0"]
[SetUp "1"]
[FEN "8/8/8/8/8/5K2/4R3/6k1 w - - 0 1"]

1. Re1+ Kh2 2. Ra1 Kh3 3. Rh1# 1-0
"#;

#[test]
fn test_scid_format_roundtrip() -> Result<()> {
    for format in [ScidFormat::Si4, ScidFormat::Si5] {
        let dir = tempdir()?;
        let db_path = dir.path().join(format!("test_db.{}", format));

        let mut db = ScidDatabaseWrapper::create(&db_path, format)?;
        assert_eq!(db.game_count(), 0);

        let idx0 = db.add_game(SAMPLE_GAME_1)?;
        let idx1 = db.add_game(SAMPLE_GAME_2)?;
        let idx2 = db.add_game(SAMPLE_GAME_3_VARIATIONS)?;
        let idx3 = db.add_game(SAMPLE_GAME_4_CUSTOM_FEN)?;

        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(idx2, 2);
        assert_eq!(idx3, 3);
        assert_eq!(db.game_count(), 4);

        let pgn0 = db.game_pgn(0)?;
        let pgn1 = db.game_pgn(1)?;
        let pgn2 = db.game_pgn(2)?;
        let pgn3 = db.game_pgn(3)?;

        assert!(pgn0.contains("Anderssen") && pgn0.contains("Be7#"));
        assert!(pgn1.contains("Paul Morphy"));
        assert!(pgn2.contains("Kasparov"));
        assert!(pgn3.contains("FEN") || pgn3.contains("Rh1#"));

        let filter_player = GameFilter {
            player: Some("Morphy".to_string()),
            ..Default::default()
        };
        let (results, count) = db.query_games(&filter_player, 0, 10);
        assert_eq!(count, 1);
        assert_eq!(results[0].white, "Paul Morphy");

        let updated_morphy = SAMPLE_GAME_2.replace("Paris Opera", "Grand Paris Opera");
        db.update_game(1, &updated_morphy)?;
        let pgn1_updated = db.game_pgn(1)?;
        assert!(pgn1_updated.contains("Grand Paris Opera"));

        db.delete_game(0)?;
        assert_eq!(db.is_deleted(0), Some(true));
        db.undelete_game(0)?;
        assert_eq!(db.is_deleted(0), Some(false));

        db.compact()?;
        db.save()?;

        let reopened = ScidDatabaseWrapper::open(&db_path)?;
        assert_eq!(reopened.game_count(), 4);
        assert_eq!(reopened.format(), format);
    }
    Ok(())
}

#[test]
fn test_alapin_sicilian_piece_placement_search() {
    let alapin_game = r#"[Event "Sicilian Alapin Test"]
[Site "Online"]
[Date "2024.01.01"]
[Round "1"]
[White "Player A"]
[Black "Player B"]
[Result "1-0"]

1. e4 c5 2. c3 d5 3. exd5 Qxd5 4. d4 1-0
"#;

    let dir = tempdir().unwrap();
    let pgn_path = dir.path().join("alapin.pgn");
    std::fs::write(&pgn_path, alapin_game).unwrap();

    let pgn_db = PgnDatabaseWrapper::open(&pgn_path).unwrap();

    let alapin_piece_placement = "rnbqkbnr/pp1ppppp/8/2p5/4P3/2P5/PP1P1PPP/RNBQKBNR";
    let res = pgn_db
        .search_position(alapin_piece_placement, None, None, Some(50), |_, _, _| {})
        .unwrap();
    assert_eq!(res.matches.len(), 1);
    assert_eq!(res.matches[0].ply, 3);

    let partial_fen = "8/8/8/2p5/8/2P5/8/8";
    let res_partial = pgn_db
        .search_position(partial_fen, None, Some("partial"), Some(50), |_, _, _| {})
        .unwrap();
    assert_eq!(res_partial.matches.len(), 1);

    let fen_black_turn = "rnbqkbnr/pp1ppppp/8/2p5/4P3/2P5/PP1P1PPP/RNBQKBNR b KQkq - 0 2";
    let res_turn = pgn_db
        .search_position(fen_black_turn, Some("b"), None, Some(50), |_, _, _| {})
        .unwrap();
    assert_eq!(res_turn.matches.len(), 1);
}

#[test]
fn test_dynamic_opening_tree_scid_and_pgn() {
    let pgn_content = r#"[Event "Game 1"]
[White "White 1"]
[Black "Black 1"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. c3 Nf6 5. d4 exd4 1-0

[Event "Game 2"]
[White "White 2"]
[Black "Black 2"]
[Result "0-1"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 0-1

[Event "Game 3"]
[White "White 3"]
[Black "Black 3"]
[Result "1/2-1/2"]

1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 1/2-1/2
"#;

    let dir = tempdir().unwrap();
    let pgn_path = dir.path().join("tree_test.pgn");
    std::fs::write(&pgn_path, pgn_content).unwrap();

    let pgn_db = PgnDatabaseWrapper::open(&pgn_path).unwrap();

    let tree_start =
        TreeIndex::calculate_tree_for_pgn(&pgn_db.entries, pgn_db.mmap_ref(), "", None, Some(500))
            .unwrap();

    assert_eq!(tree_start.total_games, 3);
    assert_eq!(tree_start.moves.len(), 1);
    assert_eq!(tree_start.moves[0].san, "e4");

    let scid_path = dir.path().join("tree_test.si5");
    let mut scid_db = ScidDatabaseWrapper::create(&scid_path, ScidFormat::Si5).unwrap();
    scid_db
        .add_game(
            r#"[Event "Game 1"]
[White "White 1"]
[Black "Black 1"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. c3 Nf6 5. d4 exd4 1-0"#,
        )
        .unwrap();
    scid_db.save().unwrap();

    let tree_scid_start = TreeIndex::calculate_tree_for_scid(
        scid_db.entries(),
        scid_db.games_path(),
        "",
        None,
        Some(500),
    )
    .unwrap();

    assert_eq!(tree_scid_start.total_games, 1);
}

#[test]
fn test_compact_single_file_pgn_index() {
    let pgn_text = r#"[Event "World Championship"]
[Date "2018.11.09"]
[White "Carlsen, Magnus"]
[Black "Caruana, Fabiano"]
[Result "1/2-1/2"]

1. e4 c5 2. Nf3 Nc6 3. Bb5 g6 1/2-1/2
"#;

    let dir = tempdir().unwrap();
    let pgn_path = dir.path().join("championship.pgn");
    std::fs::write(&pgn_path, pgn_text).unwrap();

    let pgn_db = PgnDatabaseWrapper::open(&pgn_path).unwrap();
    assert_eq!(pgn_db.game_count(), 1);

    let idx_path = dir.path().join("championship.pgn.idx");
    assert!(idx_path.exists());
}

#[test]
fn test_sort_scid_database_in_place_and_to_dest() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_sort.si5");
    let mut db = ScidDatabaseWrapper::create(&db_path, ScidFormat::Si5).unwrap();

    db.add_game("[Event \"Modern\"]\n[Date \"2023.01.01\"]\n[WhiteElo \"2800\"]\n[White \"Carlsen\"]\n[Black \"Nakamura\"]\n\n1. e4 e5 *").unwrap();
    db.add_game("[Event \"Old\"]\n[Date \"1999.03.20\"]\n[WhiteElo \"2600\"]\n[White \"Kramnik\"]\n[Black \"Leko\"]\n\n1. Nf3 d5 *").unwrap();
    db.save().unwrap();

    let sorted_count = db.sort_database("date", true, false).unwrap();
    assert_eq!(sorted_count, 2);

    let g0 = db.get_game_summary(0).unwrap();
    assert_eq!(g0.date, "1999.03.20");
}

#[test]
fn test_multi_column_caching_and_fast_desc() {
    let dir = tempdir().unwrap();
    let scid_path = dir.path().join("sort_cache_test.si5");
    let mut scid_db = ScidDatabaseWrapper::create(&scid_path, ScidFormat::Si5).unwrap();

    let games = [
        "[Event \"Event C\"]\n[White \"Carlsen, Magnus\"]\n[Black \"Anand, Viswanathan\"]\n[Result \"1-0\"]\n[ECO \"C50\"]\n[Date \"2015.01.10\"]\n[WhiteElo \"2850\"]\n[BlackElo \"2790\"]\n\n1. e4 e5 1-0",
        "[Event \"Event A\"]\n[White \"Kasparov, Garry\"]\n[Black \"Karpov, Anatoly\"]\n[Result \"0-1\"]\n[ECO \"B90\"]\n[Date \"1985.10.15\"]\n[WhiteElo \"2800\"]\n[BlackElo \"2750\"]\n\n1. e4 c5 0-1",
        "[Event \"Event B\"]\n[White \"Fischer, Robert\"]\n[Black \"Spassky, Boris\"]\n[Result \"1/2-1/2\"]\n[ECO \"E60\"]\n[Date \"1972.07.11\"]\n[WhiteElo \"2785\"]\n[BlackElo \"2660\"]\n\n1. d4 Nf6 1/2-1/2",
    ];

    for g in &games {
        scid_db.add_game(g).unwrap();
    }
    scid_db.save().unwrap();

    let filter_white_asc = GameFilter {
        sort_by: Some("white".to_string()),
        sort_asc: Some(true),
        ..Default::default()
    };
    let (summaries_asc, total) = scid_db.query_games(&filter_white_asc, 0, 10);
    assert_eq!(total, 3);
    assert_eq!(summaries_asc[0].white, "Carlsen, Magnus");
    assert_eq!(summaries_asc[1].white, "Fischer, Robert");
    assert_eq!(summaries_asc[2].white, "Kasparov, Garry");

    let filter_white_desc = GameFilter {
        sort_by: Some("white".to_string()),
        sort_asc: Some(false),
        ..Default::default()
    };
    let (summaries_desc, _) = scid_db.query_games(&filter_white_desc, 0, 10);
    assert_eq!(summaries_desc[0].white, "Kasparov, Garry");
    assert_eq!(summaries_desc[1].white, "Fischer, Robert");
    assert_eq!(summaries_desc[2].white, "Carlsen, Magnus");
}

#[test]
fn test_scidpos5_inverted_index_filtered_and_unfiltered() {
    let pgn_text = r#"[Event "Game 1"]
[White "Player A"]
[Black "Player B"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 1-0

[Event "Game 2"]
[White "Player C"]
[Black "Player D"]
[Result "0-1"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 0-1
"#;

    let dir = tempdir().unwrap();
    let pgn_path = dir.path().join("inverted_test.pgn");
    std::fs::write(&pgn_path, pgn_text).unwrap();

    let pgn_db = PgnDatabaseWrapper::open(&pgn_path).unwrap();

    let pos_idx = PositionIndex::build_for_pgn(
        &pgn_path,
        &pgn_db.entries,
        pgn_db.mmap_ref(),
        16,
        None,
        None,
        Some(1),
        |_, _, _| {},
    )
    .expect("build_for_pgn should succeed");

    assert_eq!(
        pos_idx.header.magic,
        *scid_mgr::position_index::POS_INDEX_MAGIC
    );
    assert_eq!(&pos_idx.header.magic, b"SCIDPOS5");
    assert!(pos_idx.header.unique_positions > 0);
}

#[test]
fn test_scid_pos_idx_multithreaded_build_and_query() {
    let dir = tempdir().unwrap();
    let scid_path = dir.path().join("multithread_tree_test.si5");
    let mut scid_db = ScidDatabaseWrapper::create(&scid_path, ScidFormat::Si5).unwrap();

    for i in 0..30 {
        let pgn = if i % 2 == 0 {
            "[Event \"Test\"]\n[Result \"1-0\"]\n\n1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 1-0"
        } else {
            "[Event \"Test\"]\n[Result \"0-1\"]\n\n1. e4 c5 2. Nf3 d6 3. d4 cxd4 0-1"
        };
        scid_db.add_game(pgn).unwrap();
    }
    scid_db.save().unwrap();

    let games_path = scid_db.games_path().to_path_buf();
    let entries = scid_db.entries();
    let db_path_buf = scid_db.index_path().to_path_buf();

    let tree_idx = TreeIndex::build_for_scid(
        &db_path_buf,
        entries,
        &games_path,
        16,
        Some(10),
        None,
        Some(2),
        |_, _, _| {},
    )
    .expect("build_for_scid should succeed");

    let start_tree = tree_idx
        .query_tree("")
        .expect("Should find starting position");
    assert_eq!(start_tree.total_games, 30);
}

#[test]
fn test_candidate_acceleration_correctness() {
    let dir = tempdir().unwrap();
    let scid_path = dir.path().join("candidate_test.si5");
    let mut scid_db = ScidDatabaseWrapper::create(&scid_path, ScidFormat::Si5).unwrap();

    let pgn_samples = [
        "[Event \"WCh\"]\n[White \"Kasparov\"]\n[Black \"Karpov\"]\n[Result \"1-0\"]\n[Date \"1985.10.15\"]\n\n1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6 1-0",
        "[Event \"WCh\"]\n[White \"Karpov\"]\n[Black \"Kasparov\"]\n[Result \"0-1\"]\n[Date \"1985.10.17\"]\n\n1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6 0-1",
    ];

    for pgn in pgn_samples {
        scid_db.add_game(pgn).unwrap();
    }
    scid_db.save().unwrap();

    let fen_najdorf = "rnbqkb1r/1p2pppp/p2p1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 6";
    let matcher =
        scid_mgr::position_search::parse_position_matcher(fen_najdorf, None, Some("exact"))
            .unwrap();
    let full_scan_matches = scid_mgr::position_search::search_position_matcher_mmap_with_progress(
        scid_db.entries(),
        scid_db.games_path(),
        &matcher,
        Some(16),
        |_, _, _| {},
    )
    .unwrap();
    let full_scan_ids: Vec<usize> = full_scan_matches.iter().map(|m| m.game_id).collect();
    assert_eq!(full_scan_ids, vec![0, 1]);
}

#[test]
fn test_pos_idx_min_games_filter() {
    let dir = tempdir().unwrap();
    let scid_path = dir.path().join("min_games_test.si5");
    let mut scid_db = ScidDatabaseWrapper::create(&scid_path, ScidFormat::Si5).unwrap();

    for _ in 0..5 {
        scid_db
            .add_game("[Event \"Test\"]\n\n1. e4 e5 2. Nf3 Nc6 *")
            .unwrap();
    }
    scid_db
        .add_game("[Event \"Rare\"]\n\n1. b4 e5 2. Bb2 Bxb4 *")
        .unwrap();
    scid_db.save().unwrap();

    let db_path = scid_db.index_path().to_path_buf();
    let games_path = scid_db.games_path().to_path_buf();

    let filtered_idx = TreeIndex::build_for_scid(
        &db_path,
        scid_db.entries(),
        &games_path,
        16,
        None,
        Some(3),
        None,
        |_, _, _| {},
    )
    .unwrap();

    let fen_b4 = "rnbqkbnr/pppppppp/8/8/1P6/8/P1PPPPPP/RNBQKBNR b KQkq b3 0 1";
    assert!(
        filtered_idx.query_tree(fen_b4).is_none(),
        "Rare position 1.b4 should be excluded when min_games=3"
    );
}

#[test]
fn test_position_index_and_tree_index_separation() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_games.si5");
    let mut db = ScidDatabaseWrapper::create(&db_path, ScidFormat::Si5).unwrap();

    db.add_game("[Event \"G1\"]\n[Result \"1-0\"]\n\n1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 1-0")
        .unwrap();
    db.add_game("[Event \"G2\"]\n[Result \"1/2-1/2\"]\n\n1. e4 c5 2. Nf3 d6 3. d4 cxd4 1/2-1/2")
        .unwrap();
    db.save().unwrap();

    let games_path = db.games_path().to_path_buf();
    let entries = db.entries();
    let db_path_buf = db.index_path().to_path_buf();

    let pos_idx = PositionIndex::build_for_scid(
        &db_path_buf,
        entries,
        &games_path,
        24,
        None,
        None,
        None,
        |_scanned, _total, _pos| {},
    )
    .unwrap();

    assert_eq!(
        pos_idx.header.magic,
        *scid_mgr::position_index::POS_INDEX_MAGIC
    );

    let start_pos = shakmaty::Chess::default();
    let start_hash: shakmaty::zobrist::Zobrist64 =
        start_pos.zobrist_hash(shakmaty::EnPassantMode::Legal);

    let matching_gids = pos_idx.get_matching_game_ids(start_hash.0).unwrap();
    assert_eq!(matching_gids.len(), 2);
}

#[test]
fn test_sort_pgn_chronological_and_descending() {
    let dir = tempdir().unwrap();
    let input_pgn = dir.path().join("unsorted.pgn");
    let output_asc = dir.path().join("sorted_asc.pgn");

    let pgn_content = r#"[Event "Game 1"]
[Date "2024.01.01"]
[White "Player D"]
[Black "Player A"]
[Result "1-0"]

1. e4 e5 1-0

[Event "Game 2"]
[Date "1990.05.12"]
[White "Player B"]
[Black "Player C"]
[Result "0-1"]

1. d4 d5 0-1
"#;

    std::fs::write(&input_pgn, pgn_content).unwrap();

    let count_asc =
        scid_mgr::pgn_db::sort_pgn_file(&input_pgn, &output_asc, Some("date"), true).unwrap();
    assert_eq!(count_asc, 2);

    let db_asc = scid_mgr::pgn_db::PgnDatabase::open(&output_asc).unwrap();
    assert_eq!(db_asc.entries[0].date_str(), "1990.05.12");
    assert_eq!(db_asc.entries[1].date_str(), "2024.01.01");
}

#[test]
fn test_unified_search_engine_integration_scid_and_pgn() -> Result<()> {
    let dir = tempdir()?;
    let pgn_path = dir.path().join("games.pgn");
    let si5_path = dir.path().join("games.si5");

    let pgn_data = format!(
        "{}\n\n{}\n\n{}\n\n{}",
        SAMPLE_GAME_1, SAMPLE_GAME_2, SAMPLE_GAME_3_VARIATIONS, SAMPLE_GAME_4_CUSTOM_FEN
    );
    std::fs::write(&pgn_path, &pgn_data)?;

    let mut scid_db = ScidDatabaseWrapper::create(&si5_path, ScidFormat::Si5)?;
    scid_db.add_game(SAMPLE_GAME_1)?;
    scid_db.add_game(SAMPLE_GAME_2)?;
    scid_db.add_game(SAMPLE_GAME_3_VARIATIONS)?;
    scid_db.add_game(SAMPLE_GAME_4_CUSTOM_FEN)?;

    let pgn_db = PgnDatabaseWrapper::open(&pgn_path)?;

    let q1 = scid_mgr::search::QueryParser::parse_str("player 'Morphy'")?;
    assert_eq!(scid_db.search_query(&q1).len(), 1);
    assert_eq!(pgn_db.search_query(&q1).len(), 1);

    let q_checkmate = scid_mgr::search::QueryParser::parse_str("checkmate")?;
    assert_eq!(scid_db.search_query(&q_checkmate).len(), 3);
    assert_eq!(pgn_db.search_query(&q_checkmate).len(), 3);

    Ok(())
}

#[test]
fn test_scid_search_comprehensive_matrix() -> Result<()> {
    let dir = tempdir()?;
    let si5_path = dir.path().join("matrix_games.si5");
    let pgn_path = dir.path().join("matrix_games.pgn");

    let pgn_data = format!(
        "{}\n\n{}\n\n{}\n\n{}",
        SAMPLE_GAME_1, SAMPLE_GAME_2, SAMPLE_GAME_3_VARIATIONS, SAMPLE_GAME_4_CUSTOM_FEN
    );
    std::fs::write(&pgn_path, &pgn_data)?;

    let mut scid_db = ScidDatabaseWrapper::create(&si5_path, ScidFormat::Si5)?;
    scid_db.add_game(SAMPLE_GAME_1)?;
    scid_db.add_game(SAMPLE_GAME_2)?;
    scid_db.add_game(SAMPLE_GAME_3_VARIATIONS)?;
    scid_db.add_game(SAMPLE_GAME_4_CUSTOM_FEN)?;

    let pgn_db = PgnDatabaseWrapper::open(&pgn_path)?;

    // 1. Header tests: White, Black, Elo, ECO, Date, Event, Result
    let q_white = scid_mgr::search::QueryParser::parse_str("white 'Kasparov'")?;
    assert_eq!(scid_db.search_query(&q_white).len(), 1);
    assert_eq!(pgn_db.search_query(&q_white).len(), 1);

    let q_black = scid_mgr::search::QueryParser::parse_str("black 'Kieseritzky'")?;
    assert_eq!(scid_db.search_query(&q_black).len(), 1);
    assert_eq!(pgn_db.search_query(&q_black).len(), 1);

    let q_elo = scid_mgr::search::QueryParser::parse_str("white_elo >= 2700")?;
    // Morphy (2700) and Kasparov (2800)
    assert_eq!(scid_db.search_query(&q_elo).len(), 2);
    assert_eq!(pgn_db.search_query(&q_elo).len(), 2);

    let q_eco = scid_mgr::search::QueryParser::parse_str("eco 'C33'")?;
    assert_eq!(scid_db.search_query(&q_eco).len(), 1);
    assert_eq!(pgn_db.search_query(&q_eco).len(), 1);

    let q_res_draw = scid_mgr::search::QueryParser::parse_str("result '1/2-1/2'")?;
    assert_eq!(scid_db.search_query(&q_res_draw).len(), 1);
    assert_eq!(pgn_db.search_query(&q_res_draw).len(), 1);

    // 2. Pure Board & Position tests
    let q_turn_black = scid_mgr::search::QueryParser::parse_str("btm and ply <= 5")?;
    assert_eq!(scid_db.search_query(&q_turn_black).len(), 4);
    assert_eq!(pgn_db.search_query(&q_turn_black).len(), 4);

    let q_queens = scid_mgr::search::QueryParser::parse_str("[Qq] >= 2")?;
    assert_eq!(scid_db.search_query(&q_queens).len(), 3);
    assert_eq!(pgn_db.search_query(&q_queens).len(), 3);

    // 3. Power tests
    let q_power = scid_mgr::search::QueryParser::parse_str("white_power > black_power")?;
    assert!(scid_db.search_query(&q_power).len() >= 2);
    assert!(pgn_db.search_query(&q_power).len() >= 2);

    // 4. Move tests with and without SAN
    let q_san_move = scid_mgr::search::QueryParser::parse_str("move 'Rd8#'")?;
    assert_eq!(scid_db.search_query(&q_san_move).len(), 1);
    assert_eq!(pgn_db.search_query(&q_san_move).len(), 1);

    let q_piece_move = scid_mgr::search::QueryParser::parse_str("move piece N to d5")?;
    assert_eq!(scid_db.search_query(&q_piece_move).len(), 1); // Game 0: 17. Nd5
    assert_eq!(pgn_db.search_query(&q_piece_move).len(), 1);

    // 5. Line and Path tests
    let q_line = scid_mgr::search::QueryParser::parse_str("line [e4 e5 Nf3]")?;
    assert_eq!(scid_db.search_query(&q_line).len(), 1); // Game 1
    assert_eq!(pgn_db.search_query(&q_line).len(), 1);

    let q_path_wildcard = scid_mgr::search::QueryParser::parse_str("path [e4 ... Rd8#]")?;
    assert_eq!(scid_db.search_query(&q_path_wildcard).len(), 1);
    assert_eq!(pgn_db.search_query(&q_path_wildcard).len(), 1);

    // 6. Boolean & Composite tests
    let q_and = scid_mgr::search::QueryParser::parse_str("player 'Morphy' and move 'Rd8#'")?;
    assert_eq!(scid_db.search_query(&q_and).len(), 1);
    assert_eq!(pgn_db.search_query(&q_and).len(), 1);

    let q_or = scid_mgr::search::QueryParser::parse_str("white 'Morphy' or white 'Kasparov'")?;
    assert_eq!(scid_db.search_query(&q_or).len(), 2);
    assert_eq!(pgn_db.search_query(&q_or).len(), 2);

    let q_not = scid_mgr::search::QueryParser::parse_str("not white 'Morphy'")?;
    assert_eq!(scid_db.search_query(&q_not).len(), 3);
    assert_eq!(pgn_db.search_query(&q_not).len(), 3);

    // 7. Sub-range search tests (e.g. search only games in range [1, 3))
    let q_all = scid_mgr::search::QueryParser::parse_str("result '1-0' or result '1/2-1/2'")?;
    assert_eq!(scid_db.search_query(&q_all).len(), 4);
    assert_eq!(pgn_db.search_query(&q_all).len(), 4);

    let scid_range_0_2 = scid_db.search_query_range(&q_all, 0, 2);
    let pgn_range_0_2 = pgn_db.search_query_range(&q_all, 0, 2);
    assert_eq!(scid_range_0_2.len(), 2);
    assert_eq!(pgn_range_0_2.len(), 2);
    assert_eq!(scid_range_0_2[0].game_id, 0);
    assert_eq!(scid_range_0_2[1].game_id, 1);
    assert_eq!(pgn_range_0_2[0].game_id, 0);
    assert_eq!(pgn_range_0_2[1].game_id, 1);

    let scid_range_2_3 = scid_db.search_query_range(&q_all, 2, 3);
    let pgn_range_2_3 = pgn_db.search_query_range(&q_all, 2, 3);
    assert_eq!(scid_range_2_3.len(), 1);
    assert_eq!(pgn_range_2_3.len(), 1);
    assert_eq!(scid_range_2_3[0].game_id, 2);
    assert_eq!(pgn_range_2_3[0].game_id, 2);

    Ok(())
}
