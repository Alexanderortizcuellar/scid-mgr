use anyhow::Result;
use scid_mgr::db::{ScidDatabaseWrapper, ScidFormat};
use scid_mgr::endgame_index::{
    EndgameCatalog, EndgameDetector, EndgameIndexBuilder, EndgamePopularityReport,
    EndgameQueryEngine, FeatureDetector, MmapFeatureIndex,
};
use shakmaty::fen::Fen;
use shakmaty::{CastlingMode, Chess};
use std::io::Write;
use tempfile::tempdir;

const ENDGAME_PGN: &str = r#"[Event "Endgame Study 1 - Lucena"]
[Date "2024.01.01"]
[White "Player A"]
[Black "Player B"]
[Result "1-0"]
[SetUp "1"]
[FEN "1K1R4/8/k7/8/8/8/8/6r1 w - - 0 1"]

1. Rd6+ Ka5 2. Rd7 1-0

[Event "Endgame Study 2 - Opposite Bishops"]
[Date "2024.01.02"]
[White "Player C"]
[Black "Player D"]
[Result "1/2-1/2"]
[SetUp "1"]
[FEN "8/8/8/4k3/4b3/8/5B2/4K3 w - - 0 1"]

1. Bg3+ Kd4 2. Bf2+ Kd5 1/2-1/2

[Event "Endgame Study 3 - King and Pawn"]
[Date "2024.01.03"]
[White "Player E"]
[Black "Player F"]
[Result "1-0"]
[SetUp "1"]
[FEN "8/8/8/8/4k3/8/4P3/4K3 w - - 0 1"]

1. Kd2 Kd4 2. e3+ Ke4 3. Ke2 1-0
"#;

#[test]
fn test_endgame_catalog_and_detector_integration() {
    let catalog = EndgameCatalog::default_catalog();
    assert_eq!(catalog.features.len(), 47);

    let mut detector = EndgameDetector::with_catalog(catalog);

    let fen: Fen = "8/8/8/8/4k3/8/4P3/4K3 w - - 0 1".parse().unwrap();
    let pos: Chess = fen.into_position(CastlingMode::Chess960).unwrap();

    detector.process_position(&pos);
    let record = detector.finish_game();

    assert!(record.has_endgame_bit(0)); // END_PAWN_KP_K
    assert!(!record.has_endgame_bit(5)); // END_ROOK_R_R
}

#[test]
fn test_pgn_endgame_index_build_and_query() -> Result<()> {
    let dir = tempdir()?;
    let pgn_path = dir.path().join("endgames.pgn");
    let mut file = std::fs::File::create(&pgn_path)?;
    file.write_all(ENDGAME_PGN.as_bytes())?;
    file.flush()?;

    let builder = EndgameIndexBuilder::new();
    let (idx_path, total_games, _elapsed) = builder.build_for_pgn(&pgn_path, None, None)?;

    assert_eq!(total_games, 3);
    assert!(idx_path.exists());

    let mmap_idx = MmapFeatureIndex::open(&idx_path)?;
    assert_eq!(mmap_idx.game_count(), 3);

    let catalog = EndgameCatalog::default_catalog();
    let popularity: EndgamePopularityReport = EndgameQueryEngine::calculate_popularity(
        &mmap_idx,
        &catalog,
        pgn_path.to_str().unwrap(),
        None,
        None,
        None,
    )?;

    assert_eq!(popularity.total_db_games, 3);
    assert_eq!(popularity.categories.len(), 8);

    // Query feature 0 (END_PAWN_KP_K)
    let q_rep = EndgameQueryEngine::query_feature(&mmap_idx, &catalog, "END_PAWN_KP_K", 5)?;
    assert_eq!(q_rep.matching_games_count, 1);
    assert_eq!(q_rep.sample_game_ids, vec![2]);

    // Query feature 18 (END_BISHOP_OCB)
    let q_rep_ocb = EndgameQueryEngine::query_feature(&mmap_idx, &catalog, "END_BISHOP_OCB", 5)?;
    assert_eq!(q_rep_ocb.matching_games_count, 1);
    assert_eq!(q_rep_ocb.sample_game_ids, vec![1]);

    Ok(())
}

#[test]
fn test_scid_endgame_index_build_and_query() -> Result<()> {
    let dir = tempdir()?;
    let scid_base = dir.path().join("test_db.si5");
    let mut db = ScidDatabaseWrapper::create(&scid_base, ScidFormat::Si5)?;

    let g1 = r#"[Event "Endgame Study 1 - Lucena"]
[Date "2024.01.01"]
[White "Player A"]
[Black "Player B"]
[Result "1-0"]
[SetUp "1"]
[FEN "1K1R4/8/k7/8/8/8/8/6r1 w - - 0 1"]

1. Rd6+ Ka5 2. Rd7 1-0"#;

    let g2 = r#"[Event "Endgame Study 2 - Opposite Bishops"]
[Date "2024.01.02"]
[White "Player C"]
[Black "Player D"]
[Result "1/2-1/2"]
[SetUp "1"]
[FEN "8/8/8/4k3/4b3/8/5B2/4K3 w - - 0 1"]

1. Bg3+ Kd4 2. Bf2+ Kd5 1/2-1/2"#;

    let g3 = r#"[Event "Endgame Study 3 - King and Pawn"]
[Date "2024.01.03"]
[White "Player E"]
[Black "Player F"]
[Result "1-0"]
[SetUp "1"]
[FEN "8/8/8/8/4k3/8/4P3/4K3 w - - 0 1"]

1. Kd2 Kd4 2. e3+ Ke4 3. Ke2 1-0"#;

    db.add_game(g1)?;
    db.add_game(g2)?;
    db.add_game(g3)?;
    db.save()?;
    assert_eq!(db.game_count(), 3);

    let builder = EndgameIndexBuilder::new();
    let (idx_path, total_games, _elapsed) = builder.build_for_scid(&db, None, None)?;

    assert_eq!(total_games, 3);
    assert!(idx_path.exists());

    let mmap_idx = MmapFeatureIndex::open(&idx_path)?;
    assert_eq!(mmap_idx.game_count(), 3);

    let catalog = EndgameCatalog::default_catalog();
    let popularity = EndgameQueryEngine::calculate_popularity(
        &mmap_idx,
        &catalog,
        scid_base.to_str().unwrap(),
        None,
        None,
        None,
    )?;

    assert_eq!(popularity.total_db_games, 3);
    Ok(())
}

#[test]
fn test_cli_endgames_handlers() -> Result<()> {
    let dir = tempdir()?;
    let pgn_path = dir.path().join("endgames_cli.pgn");
    let mut file = std::fs::File::create(&pgn_path)?;
    file.write_all(ENDGAME_PGN.as_bytes())?;
    file.flush()?;

    // 1. Build index via CLI handler
    scid_mgr::cli::commands::endgames::handle_build_endgames(&pgn_path, None)?;

    // 2. Query general popularity via CLI handler
    scid_mgr::cli::commands::endgames::handle_endgames(
        &pgn_path, None, None, None, true, // JSON output
        10,
    )?;

    // 3. Query category filter
    scid_mgr::cli::commands::endgames::handle_endgames(
        &pgn_path,
        None,
        Some("ROOK".to_string()),
        None,
        false,
        10,
    )?;

    // 4. Query specific feature ID
    scid_mgr::cli::commands::endgames::handle_endgames(
        &pgn_path,
        None,
        None,
        Some("END_PAWN_KP_K".to_string()),
        false,
        10,
    )?;

    Ok(())
}

#[test]
fn test_json_rpc_endgames_handlers() -> Result<()> {
    let dir = tempdir()?;
    let pgn_path = dir.path().join("endgames_rpc.pgn");
    let mut file = std::fs::File::create(&pgn_path)?;
    file.write_all(ENDGAME_PGN.as_bytes())?;
    file.flush()?;

    let pgn_db = scid_mgr::pgn_db::PgnDatabaseWrapper::open(&pgn_path)?;
    let backend = Some(scid_mgr::server::DatabaseBackend::Pgn(pgn_db));
    let mut pos_idx = None;

    // 1. Build index via RPC
    let build_req = scid_mgr::server::RequestMessage {
        id: Some(1),
        command: "build_endgames".to_string(),
        params: serde_json::json!({}),
    };
    let build_resp =
        scid_mgr::server::handlers::endgames::handle_build_endgames(&build_req, &backend);
    assert_eq!(build_resp.status, "ok");

    // 2. Query general endgames via RPC
    let query_req = scid_mgr::server::RequestMessage {
        id: Some(2),
        command: "endgames".to_string(),
        params: serde_json::json!({}),
    };
    let query_resp =
        scid_mgr::server::handlers::endgames::handle_endgames(&query_req, &backend, &mut pos_idx);
    assert_eq!(query_resp.status, "ok");
    assert!(query_resp.data.is_some());

    // 3. Query specific feature via RPC
    let feat_req = scid_mgr::server::RequestMessage {
        id: Some(3),
        command: "endgames".to_string(),
        params: serde_json::json!({
            "feature_id": "END_PAWN_KP_K",
            "max_samples": 5
        }),
    };
    let feat_resp =
        scid_mgr::server::handlers::endgames::handle_endgames(&feat_req, &backend, &mut pos_idx);
    assert_eq!(feat_resp.status, "ok");
    assert!(feat_resp.data.is_some());

    Ok(())
}

#[test]
fn test_handle_check_all_four_indexes() -> Result<()> {
    let dir = tempdir()?;
    let pgn_path = dir.path().join("check_test.pgn");
    let mut file = std::fs::File::create(&pgn_path)?;
    file.write_all(ENDGAME_PGN.as_bytes())?;
    file.flush()?;

    // 1. Initial check before any companion indexes are built
    scid_mgr::cli::commands::check::handle_check(&pgn_path, true, false)?;
    scid_mgr::cli::commands::check::handle_check(&pgn_path, true, true)?;

    // 2. Build endgame index (.feat.idx)
    scid_mgr::cli::commands::endgames::handle_build_endgames(&pgn_path, None)?;

    // 3. Build position index (.pos.idx)
    scid_mgr::cli::commands::index::handle_build_pos_idx(&pgn_path, 24, 0, 1, None)?;

    // 4. Build tree index (.tree.idx)
    scid_mgr::cli::commands::tree::handle_build_tree(&pgn_path, 24, 0, None)?;

    // 5. Build continuations index (.hot.idx)
    scid_mgr::cli::commands::continuations::handle_build_continuations(&pgn_path, 16, 1)?;

    // 6. Check again with all 4 indexes generated (text and json output)
    scid_mgr::cli::commands::check::handle_check(&pgn_path, true, false)?;
    scid_mgr::cli::commands::check::handle_check(&pgn_path, true, true)?;

    Ok(())
}

#[test]
fn test_booster_accelerated_endgame_build_and_equivalence() -> Result<()> {
    let dir = tempdir()?;
    let pgn_path = dir.path().join("booster_endgames.pgn");
    let mut file = std::fs::File::create(&pgn_path)?;
    file.write_all(ENDGAME_PGN.as_bytes())?;
    file.flush()?;

    let builder = EndgameIndexBuilder::new();

    // 1. Build normal endgame index without booster
    let no_boost_feat = dir.path().join("no_boost.feat.idx");
    let (p1, total1, _t1) = builder.build_for_pgn(&pgn_path, Some(no_boost_feat.clone()), None)?;
    assert_eq!(total1, 3);
    let mmap1 = MmapFeatureIndex::open(&p1)?;

    // 2. Build booster index for the PGN
    let (boost_path, boost_games, boost_plies, _tb) =
        scid_mgr::search_booster::BoostIndexBuilder::build_for_pgn(&pgn_path, None, None)?;
    assert_eq!(boost_games, 3);
    assert!(boost_plies > 0);
    assert!(boost_path.exists());

    // 3. Build endgame index WITH booster (automatically detected)
    let with_boost_feat = dir.path().join("with_boost.feat.idx");
    let (p2, total2, _t2) =
        builder.build_for_pgn(&pgn_path, Some(with_boost_feat.clone()), None)?;
    assert_eq!(total2, 3);
    let mmap2 = MmapFeatureIndex::open(&p2)?;

    // 4. Verify exact bit-for-bit equivalence of feature records
    assert_eq!(mmap1.game_count(), mmap2.game_count());
    for gid in 0..mmap1.game_count() {
        let r1 = mmap1.get_record(gid).unwrap();
        let r2 = mmap2.get_record(gid).unwrap();
        assert_eq!(
            r1.endgame_bits, r2.endgame_bits,
            "Game {} endgame bits mismatch between standard build and booster-accelerated build",
            gid
        );
    }

    // 5. Test with SCID format (.si5)
    let scid_base = dir.path().join("booster_scid.si5");
    let mut db = ScidDatabaseWrapper::create(&scid_base, ScidFormat::Si5)?;
    let g1 = r#"[Event "Endgame Study 1 - Lucena"]
[Date "2024.01.01"]
[White "Player A"]
[Black "Player B"]
[Result "1-0"]
[SetUp "1"]
[FEN "1K1R4/8/k7/8/8/8/8/6r1 w - - 0 1"]

1. Rd6+ Ka5 2. Rd7 1-0"#;
    let g2 = r#"[Event "Endgame Study 2 - Opposite Bishops"]
[Date "2024.01.02"]
[White "Player C"]
[Black "Player D"]
[Result "1/2-1/2"]
[SetUp "1"]
[FEN "8/8/8/4k3/4b3/8/5B2/4K3 w - - 0 1"]

1. Bg3+ Kd4 2. Bf2+ Kd5 1/2-1/2"#;
    db.add_game(g1)?;
    db.add_game(g2)?;
    db.save()?;

    // Build SCID booster
    let (scid_boost_path, _, _, _) =
        scid_mgr::search_booster::BoostIndexBuilder::build_for_scid(&db, None, None)?;
    assert!(scid_boost_path.exists());

    // Build SCID feat index with booster fast-path
    let scid_feat_path = dir.path().join("booster_scid.feat.idx");
    let (scid_feat, total_scid, _) =
        builder.build_for_scid(&db, Some(scid_feat_path.clone()), None)?;
    assert_eq!(total_scid, 2);
    let scid_mmap = MmapFeatureIndex::open(&scid_feat)?;
    assert_eq!(scid_mmap.game_count(), 2);

    let catalog = EndgameCatalog::default_catalog();
    let q_rep = EndgameQueryEngine::query_feature(&scid_mmap, &catalog, "END_BISHOP_OCB", 5)?;
    assert_eq!(q_rep.matching_games_count, 1);
    assert_eq!(q_rep.sample_game_ids, vec![1]);

    Ok(())
}

#[test]
fn test_filtered_endgame_queries() -> Result<()> {
    let dir = tempdir()?;
    let pgn_path = dir.path().join("endgames_filter.pgn");
    let mut file = std::fs::File::create(&pgn_path)?;
    file.write_all(ENDGAME_PGN.as_bytes())?;
    file.flush()?;

    let builder = EndgameIndexBuilder::new();
    let (idx_path, total_games, _elapsed) = builder.build_for_pgn(&pgn_path, None, None)?;
    assert_eq!(total_games, 3);

    let mmap_idx = MmapFeatureIndex::open(&idx_path)?;
    let catalog = EndgameCatalog::default_catalog();

    // 1. Filtered popularity calculation (games 0 and 1 only; excluding game 2 which has KP_K)
    let pop_filtered = EndgameQueryEngine::calculate_popularity(
        &mmap_idx,
        &catalog,
        pgn_path.to_str().unwrap(),
        Some(&[0, 1]),
        None,
        None,
    )?;

    assert_eq!(pop_filtered.total_db_games, 3);
    assert_eq!(pop_filtered.games_reaching_position, 2);
    assert!(pop_filtered.position_filtered);

    // Feature 0 (END_PAWN_KP_K) should have 0 games in games [0, 1]
    let feat_kp = pop_filtered
        .features
        .iter()
        .find(|f| f.id == "END_PAWN_KP_K")
        .unwrap();
    assert_eq!(feat_kp.game_count, 0);

    // Feature 17 (END_BISHOP_OCB) should have 1 game in games [0, 1] (50%)
    let feat_ocb = pop_filtered
        .features
        .iter()
        .find(|f| f.id == "END_BISHOP_OCB")
        .unwrap();
    assert_eq!(feat_ocb.game_count, 1);
    assert_eq!(feat_ocb.percentage, 50.0);

    // 2. Filtered feature query
    let q_excluded = EndgameQueryEngine::query_feature_filtered(
        &mmap_idx,
        &catalog,
        "END_PAWN_KP_K",
        5,
        Some(&[0, 1]),
    )?;
    assert_eq!(q_excluded.matching_games_count, 0);
    assert_eq!(q_excluded.total_db_games, 2);
    assert!(q_excluded.sample_game_ids.is_empty());

    let q_included = EndgameQueryEngine::query_feature_filtered(
        &mmap_idx,
        &catalog,
        "END_PAWN_KP_K",
        5,
        Some(&[1, 2]),
    )?;
    assert_eq!(q_included.matching_games_count, 1);
    assert_eq!(q_included.total_db_games, 2);
    assert_eq!(q_included.sample_game_ids, vec![2]);

    Ok(())
}
