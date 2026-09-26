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
