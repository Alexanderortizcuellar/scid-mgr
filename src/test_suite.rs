use crate::db::{GameFilter, ScidDatabaseWrapper, ScidFormat};
use anyhow::Result;
use std::time::Instant;
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

pub fn run_full_test_suite() -> Result<()> {
    println!("============================================================");
    println!("        CHESS-SCID-RW INTEGRATION & TEST SUITE             ");
    println!("============================================================");

    let start_time = Instant::now();

    for format in [ScidFormat::Si4, ScidFormat::Si5] {
        println!("\n--- Testing format: {} ---", format);
        test_format_roundtrip(format)?;
    }

    println!("\n============================================================");
    println!(" [SUCCESS] All tests passed in {:?}", start_time.elapsed());
    println!("============================================================");
    Ok(())
}

fn test_format_roundtrip(format: ScidFormat) -> Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join(format!("test_db.{}", format));

    println!("  1. Creating new in-memory database at {:?}...", db_path);
    let mut db = ScidDatabaseWrapper::create(&db_path, format)?;
    assert_eq!(db.game_count(), 0);

    println!("  2. Adding sample games (standard, variations, custom FEN)...");
    let idx0 = db.add_game(SAMPLE_GAME_1)?;
    let idx1 = db.add_game(SAMPLE_GAME_2)?;
    let idx2 = db.add_game(SAMPLE_GAME_3_VARIATIONS)?;
    let idx3 = db.add_game(SAMPLE_GAME_4_CUSTOM_FEN)?;

    assert_eq!(idx0, 0);
    assert_eq!(idx1, 1);
    assert_eq!(idx2, 2);
    assert_eq!(idx3, 3);
    assert_eq!(db.game_count(), 4);
    println!("     Added 4 games successfully.");

    println!("  3. Verifying game PGN reconstruction...");
    let pgn0 = db.game_pgn(0)?;
    let pgn1 = db.game_pgn(1)?;
    let pgn2 = db.game_pgn(2)?;
    let pgn3 = db.game_pgn(3)?;

    assert!(pgn0.contains("Anderssen"), "Game 0 should contain Anderssen");
    assert!(pgn0.contains("Be7#"), "Game 0 should contain checkmate move");
    assert!(pgn1.contains("Paul Morphy"), "Game 1 should contain Morphy");
    assert!(pgn2.contains("Kasparov"), "Game 2 should contain Kasparov");
    assert!(pgn3.contains("FEN") || pgn3.contains("Rh1#"), "Game 3 should decode custom FEN/moves");
    println!("     PGN reconstruction verified for all games.");

    println!("  4. Testing filtering & queries...");
    let filter_player = GameFilter {
        player: Some("Morphy".to_string()),
        ..Default::default()
    };
    let (results, count) = db.query_games(&filter_player, 0, 10);
    assert_eq!(count, 1);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].white, "Paul Morphy");

    let filter_eco = GameFilter {
        eco: Some("B".to_string()),
        ..Default::default()
    };
    let (results, count) = db.query_games(&filter_eco, 0, 10);
    assert_eq!(count, 1);
    assert_eq!(results[0].eco, "B85");
    println!("     Queries and filters returned expected results.");

    println!("  5. Testing update_game...");
    let updated_morphy = SAMPLE_GAME_2.replace("Paris Opera", "Grand Paris Opera");
    db.update_game(1, &updated_morphy)?;
    let pgn1_updated = db.game_pgn(1)?;
    assert!(pgn1_updated.contains("Grand Paris Opera"));
    println!("     Update game verified.");

    println!("  6. Testing delete_game and undelete_game...");
    assert_eq!(db.is_deleted(0), Some(false));
    db.delete_game(0)?;
    assert_eq!(db.is_deleted(0), Some(true));
    let stats = db.stats();
    assert_eq!(stats.deleted_games, 1);
    assert_eq!(stats.active_games, 3);

    db.undelete_game(0)?;
    assert_eq!(db.is_deleted(0), Some(false));
    println!("     Delete / Undelete verified.");

    println!("  7. Testing compaction...");
    let reclaimed = db.compact()?;
    println!("     Compacted database, reclaimed {} bytes dead space.", reclaimed);

    println!("  8. Saving database to disk...");
    db.save()?;
    println!("     Saved companion files (.index, .namebase, .games).");

    println!("  9. Reopening database from disk & verifying integrity...");
    let reopened = ScidDatabaseWrapper::open(&db_path)?;
    assert_eq!(reopened.game_count(), 4);
    assert_eq!(reopened.format(), format);

    let reopened_pgn0 = reopened.game_pgn(0)?;
    let reopened_pgn1 = reopened.game_pgn(1)?;
    assert!(reopened_pgn0.contains("Anderssen"));
    assert!(reopened_pgn1.contains("Grand Paris Opera"));

    let reopened_stats = reopened.stats();
    println!(
        "     Reopened database stats: {} games, {} players, {} events, files: (idx: {} bytes, names: {} bytes, games: {} bytes)",
        reopened_stats.total_games,
        reopened_stats.players_count,
        reopened_stats.events_count,
        reopened_stats.index_file_size,
        reopened_stats.namebase_file_size,
        reopened_stats.games_file_size
    );

    println!("  [OK] Format {} passed all test stages.", format);
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

    let pgn_db = crate::pgn_db::PgnDatabaseWrapper::open(&pgn_path).unwrap();

    // 1. Piece placement string only (no turn / no castling / no move numbers)
    let alapin_piece_placement = "rnbqkbnr/pp1ppppp/8/2p5/4P3/2P5/PP1P1PPP/RNBQKBNR";
    let res = pgn_db.search_position(alapin_piece_placement, None, None, Some(50), |_, _, _| {}).unwrap();
    assert_eq!(res.matches.len(), 1, "Board-only search should match Alapin Sicilian at move 2!");
    assert_eq!(res.matches[0].ply, 3); // after 1.e4 c5 2.c3 (ply 3)

    // 2. Partial piece placement (only pawn on c3 and pawn on c5)
    let partial_fen = "8/8/8/2p5/8/2P5/8/8";
    let res_partial = pgn_db.search_position(partial_fen, None, Some("partial"), Some(50), |_, _, _| {}).unwrap();
    assert_eq!(res_partial.matches.len(), 1, "Partial piece placement search should match!");

    // 3. FEN with explicit black turn
    let fen_black_turn = "rnbqkbnr/pp1ppppp/8/2p5/4P3/2P5/PP1P1PPP/RNBQKBNR b KQkq - 0 2";
    let res_turn = pgn_db.search_position(fen_black_turn, Some("b"), None, Some(50), |_, _, _| {}).unwrap();
    assert_eq!(res_turn.matches.len(), 1, "Explicit black turn should match!");
}

#[test]
fn test_dynamic_opening_tree_scid_and_pgn() {
    let pgn_content = r#"[Event "Game 1"]
[White "White 1"]
[Black "Black 1"]
[Result "1-0"]
[WhiteElo "2400"]
[BlackElo "2300"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. c3 Nf6 5. d4 exd4 1-0

[Event "Game 2"]
[White "White 2"]
[Black "Black 2"]
[Result "0-1"]
[WhiteElo "2500"]
[BlackElo "2600"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 0-1

[Event "Game 3"]
[White "White 3"]
[Black "Black 3"]
[Result "1/2-1/2"]
[WhiteElo "2700"]
[BlackElo "2700"]

1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 1/2-1/2
"#;

    let dir = tempdir().unwrap();
    let pgn_path = dir.path().join("tree_test.pgn");
    std::fs::write(&pgn_path, pgn_content).unwrap();

    // 1. Test Dynamic PGN Tree
    let pgn_db = crate::pgn_db::PgnDatabaseWrapper::open(&pgn_path).unwrap();

    // Query starting position (unfiltered: all 3 games)
    let tree_start = crate::position_index::PositionIndex::calculate_tree_for_pgn(
        &pgn_db.entries,
        pgn_db.mmap_ref(),
        "",
        None,
        Some(500),
    ).expect("calculate_tree_for_pgn should succeed for starting position");

    assert_eq!(tree_start.total_games, 3);
    assert_eq!(tree_start.moves.len(), 1);
    assert_eq!(tree_start.moves[0].san, "e4");
    assert_eq!(tree_start.moves[0].total_games, 3);
    assert_eq!(tree_start.moves[0].white_wins, 1);
    assert_eq!(tree_start.moves[0].draws, 1);
    assert_eq!(tree_start.moves[0].black_wins, 1);

    // Query starting position with filtered game IDs (e.g. only Game 1 and Game 2 -> White 1 and White 2)
    let filtered_ids = vec![0usize, 1];
    let tree_filtered_pgn = crate::position_index::PositionIndex::calculate_tree_for_pgn(
        &pgn_db.entries,
        pgn_db.mmap_ref(),
        "",
        Some(&filtered_ids),
        Some(500),
    ).expect("calculate_tree_for_pgn should succeed with filtered IDs");
    assert_eq!(tree_filtered_pgn.total_games, 2);
    assert_eq!(tree_filtered_pgn.white_wins, 1);
    assert_eq!(tree_filtered_pgn.black_wins, 1);
    assert_eq!(tree_filtered_pgn.draws, 0);

    // Query position after 1.e4 (1...e5 vs 1...c5)
    let fen_after_e4 = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
    let tree_e4 = crate::position_index::PositionIndex::calculate_tree_for_pgn(
        &pgn_db.entries,
        pgn_db.mmap_ref(),
        fen_after_e4,
        None,
        Some(500),
    ).expect("calculate_tree_for_pgn should succeed for 1.e4");

    assert_eq!(tree_e4.total_games, 3);
    assert_eq!(tree_e4.moves.len(), 2);
    let e5_move = tree_e4.moves.iter().find(|m| m.san == "e5").expect("e5 should be present");
    assert_eq!(e5_move.total_games, 2);
    let c5_move = tree_e4.moves.iter().find(|m| m.san == "c5").expect("c5 should be present");
    assert_eq!(c5_move.total_games, 1);

    // Query deeper position (after 1.e4 e5 2.Nf3 Nc6)
    let fen_after_nc6 = "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3";
    let tree_nc6 = crate::position_index::PositionIndex::calculate_tree_for_pgn(
        &pgn_db.entries,
        pgn_db.mmap_ref(),
        fen_after_nc6,
        None,
        Some(500),
    ).expect("calculate_tree_for_pgn should succeed at ply 4");

    assert_eq!(tree_nc6.total_games, 2);
    assert_eq!(tree_nc6.moves.len(), 2);
    assert!(tree_nc6.moves.iter().any(|m| m.san == "Bc4"));
    assert!(tree_nc6.moves.iter().any(|m| m.san == "Bb5"));

    // 2. Test Dynamic SCID Tree
    let scid_path = dir.path().join("tree_test.si5");
    let mut scid_db = crate::db::ScidDatabaseWrapper::create(&scid_path, ScidFormat::Si5).unwrap();
    scid_db.add_game(r#"[Event "Game 1"]
[White "White 1"]
[Black "Black 1"]
[Result "1-0"]
[WhiteElo "2400"]
[BlackElo "2300"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. c3 Nf6 5. d4 exd4 1-0"#).unwrap();
    scid_db.add_game(r#"[Event "Game 2"]
[White "White 2"]
[Black "Black 2"]
[Result "0-1"]
[WhiteElo "2500"]
[BlackElo "2600"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 0-1"#).unwrap();
    scid_db.add_game(r#"[Event "Game 3"]
[White "White 3"]
[Black "Black 3"]
[Result "1/2-1/2"]
[WhiteElo "2700"]
[BlackElo "2700"]

1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 1/2-1/2"#).unwrap();
    scid_db.save().unwrap();

    let tree_scid_start = crate::position_index::PositionIndex::calculate_tree_for_scid(
        scid_db.entries(),
        scid_db.games_path(),
        "",
        None,
        Some(500),
    ).expect("calculate_tree_for_scid should succeed for starting position");

    assert_eq!(tree_scid_start.total_games, 3);
    assert_eq!(tree_scid_start.moves.len(), 1);
    assert_eq!(tree_scid_start.moves[0].san, "e4");

    // SCID with filtered IDs (e.g. only game 2 -> 1.e4 e5 2.Nf3 Nc6 3.Bb5)
    let tree_scid_filtered = crate::position_index::PositionIndex::calculate_tree_for_scid(
        scid_db.entries(),
        scid_db.games_path(),
        fen_after_nc6,
        Some(&[1usize]),
        Some(500),
    ).expect("calculate_tree_for_scid should succeed with filtered IDs");
    assert_eq!(tree_scid_filtered.total_games, 1);
    assert_eq!(tree_scid_filtered.moves.len(), 1);
    assert_eq!(tree_scid_filtered.moves[0].san, "Bb5");

    let tree_scid_nc6 = crate::position_index::PositionIndex::calculate_tree_for_scid(
        scid_db.entries(),
        scid_db.games_path(),
        fen_after_nc6,
        None,
        Some(500),
    ).expect("calculate_tree_for_scid should succeed at ply 4");

    assert_eq!(tree_scid_nc6.total_games, 2);
    assert_eq!(tree_scid_nc6.moves.len(), 2);
    assert!(tree_scid_nc6.moves.iter().any(|m| m.san == "Bc4"));
    assert!(tree_scid_nc6.moves.iter().any(|m| m.san == "Bb5"));
}

#[test]
fn test_compact_single_file_pgn_index() {
    let pgn_text = r#"[Event "World Championship"]
[Site "London"]
[Date "2018.11.09"]
[White "Carlsen, Magnus"]
[Black "Caruana, Fabiano"]
[Result "1/2-1/2"]
[ECO "B31"]
[WhiteElo "2835"]
[BlackElo "2832"]

1. e4 c5 2. Nf3 Nc6 3. Bb5 g6 1/2-1/2

[Event "World Championship"]
[Site "London"]
[Date "2018.11.12"]
[White "Caruana, Fabiano"]
[Black "Carlsen, Magnus"]
[Result "1/2-1/2"]
[ECO "D37"]
[WhiteElo "2832"]
[BlackElo "2835"]

1. d4 Nf6 2. c4 e6 3. Nf3 d5 1/2-1/2

[Event "Candidates 2024"]
[Site "Toronto"]
[Date "2024.04.04"]
[White "Caruana, Fabiano"]
[Black "Nakamura, Hikaru"]
[Result "1/2-1/2"]
[ECO "C55"]
[WhiteElo "2803"]
[BlackElo "2789"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Nf6 1/2-1/2
"#;

    let dir = tempdir().unwrap();
    let pgn_path = dir.path().join("championship.pgn");
    std::fs::write(&pgn_path, pgn_text).unwrap();

    // 1. First open: generates and saves .pgn.idx
    let pgn_db = crate::pgn_db::PgnDatabaseWrapper::open(&pgn_path).unwrap();
    assert_eq!(pgn_db.game_count(), 3);
    assert_eq!(pgn_db.names.players.len(), 4); // "?", "Carlsen, Magnus", "Caruana, Fabiano", "Nakamura, Hikaru"
    assert_eq!(pgn_db.names.events.len(), 3);  // "?", "World Championship", "Candidates 2024"
    assert_eq!(pgn_db.names.sites.len(), 3);   // "?", "London", "Toronto"

    let idx_path = dir.path().join("championship.pgn.idx");
    assert!(idx_path.exists(), "Single companion .pgn.idx must be created");

    // Check index file size is tiny
    let idx_size = std::fs::metadata(&idx_path).unwrap().len();
    assert!(idx_size > 64 && idx_size < 1024, "Index should be very compact (< 1KB for 3 games)");

    // 2. Re-open: should load directly from companion .pgn.idx without rescanning
    let pgn_db2 = crate::pgn_db::PgnDatabaseWrapper::open(&pgn_path).unwrap();
    assert_eq!(pgn_db2.game_count(), 3);
    assert_eq!(pgn_db2.names.players.len(), 4);

    // 3. Test game summary reconstruction
    let g0 = pgn_db2.get_summary(0);
    assert_eq!(g0.white, "Carlsen, Magnus");
    assert_eq!(g0.black, "Caruana, Fabiano");
    assert_eq!(g0.event, "World Championship");
    assert_eq!(g0.site, "London");
    assert_eq!(g0.eco, "B31");
    assert_eq!(g0.date, "2018.11.09");
    assert_eq!(g0.white_elo, 2835);
    assert_eq!(g0.black_elo, 2832);

    // 4. Test querying with player name filter
    let (carlsen_games, count) = pgn_db2.query_games(&crate::db::GameFilter {
        player: Some("Carlsen".to_string()),
        ..Default::default()
    }, 0, 50);
    assert_eq!(count, 2);
    assert_eq!(carlsen_games.len(), 2);

    // 5. Test querying with event filter
    let (candidates_games, count) = pgn_db2.query_games(&crate::db::GameFilter {
        event: Some("Candidates".to_string()),
        ..Default::default()
    }, 0, 50);
    assert_eq!(count, 1);
    assert_eq!(candidates_games[0].white, "Caruana, Fabiano");
    assert_eq!(candidates_games[0].black, "Nakamura, Hikaru");
}

#[test]
fn test_scidpos5_inverted_index_filtered_and_unfiltered() {
    let pgn_text = r#"[Event "Game 1"]
[White "Player A"]
[Black "Player B"]
[Result "1-0"]
[WhiteElo "2400"]
[BlackElo "2300"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 1-0

[Event "Game 2"]
[White "Player C"]
[Black "Player D"]
[Result "0-1"]
[WhiteElo "2500"]
[BlackElo "2600"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 0-1

[Event "Game 3"]
[White "Player E"]
[Black "Player F"]
[Result "1/2-1/2"]
[WhiteElo "2700"]
[BlackElo "2700"]

1. e4 c5 2. Nf3 d6 3. d4 cxd4 1/2-1/2
"#;

    let dir = tempdir().unwrap();
    let pgn_path = dir.path().join("inverted_test.pgn");
    std::fs::write(&pgn_path, pgn_text).unwrap();

    let pgn_db = crate::pgn_db::PgnDatabaseWrapper::open(&pgn_path).unwrap();

    // 1. Build SCIDPOS5 index (storing all game IDs)
    let pos_idx = crate::position_index::PositionIndex::build_for_pgn(
        &pgn_path,
        &pgn_db.entries,
        pgn_db.mmap_ref(),
        16,
        None,
        None,
        Some(1),
        |_, _, _| {},
    ).expect("build_for_pgn should succeed");

    assert_eq!(pos_idx.header.magic, *crate::position_index::POS_INDEX_MAGIC);
    assert_eq!(&pos_idx.header.magic, b"SCIDPOS5");
    assert!(pos_idx.header.unique_positions > 0);

    // 2. Query starting position unfiltered
    let start_tree = pos_idx.query_tree_with_filter("", None).expect("Should find starting position");
    assert_eq!(start_tree.total_games, 3);
    assert_eq!(start_tree.moves.len(), 1);
    assert_eq!(start_tree.moves[0].san, "e4");
    assert_eq!(start_tree.moves[0].total_games, 3);
    assert_eq!(start_tree.moves[0].white_wins, 1);
    assert_eq!(start_tree.moves[0].black_wins, 1);
    assert_eq!(start_tree.moves[0].draws, 1);

    // 3. Query starting position with filter: only Game 1 and Game 2
    let filtered_gids = vec![0usize, 1];
    let filtered_tree = pos_idx.query_tree_with_filter("", Some(&filtered_gids)).expect("Should filter starting position");
    assert_eq!(filtered_tree.total_games, 2);
    assert_eq!(filtered_tree.moves.len(), 1);
    assert_eq!(filtered_tree.moves[0].san, "e4");
    assert_eq!(filtered_tree.moves[0].total_games, 2);
    assert_eq!(filtered_tree.moves[0].white_wins, 1);
    assert_eq!(filtered_tree.moves[0].black_wins, 1);
    assert_eq!(filtered_tree.moves[0].draws, 0);

    // 4. Query position after 1.e4 with filter (only Game 3 -> 1...c5)
    let fen_after_e4 = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
    let filter_g3 = vec![2usize];
    let tree_g3 = pos_idx.query_tree_with_filter(fen_after_e4, Some(&filter_g3)).expect("Should filter after 1.e4");
    assert_eq!(tree_g3.total_games, 1);
    assert_eq!(tree_g3.moves.len(), 1);
    assert_eq!(tree_g3.moves[0].san, "c5");
    assert_eq!(tree_g3.moves[0].draws, 1);
    assert_eq!(tree_g3.moves[0].total_games, 1);

    // 5. Inverted search: get all game IDs where 1.e4 e5 2.Nf3 Nc6 is played
    let fen_nc6 = "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3";
    let (_target_pos, target_hash) = crate::position_index::parse_target_position(fen_nc6).unwrap();
    let games_with_pos = pos_idx.get_all_position_games(target_hash);
    assert_eq!(games_with_pos, Some(vec![0, 1]));
}

#[test]
fn test_scid_pos_idx_multithreaded_build_and_query() {
    let dir = tempdir().unwrap();
    let scid_path = dir.path().join("multithread_tree_test.si5");
    let mut scid_db = crate::db::ScidDatabaseWrapper::create(&scid_path, ScidFormat::Si5).unwrap();

    // Add multiple games with different moves and results
    for i in 0..100 {
        let pgn = if i % 3 == 0 {
            format!("[Event \"Test\"]\n[Result \"1-0\"]\n\n1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 1-0")
        } else if i % 3 == 1 {
            format!("[Event \"Test\"]\n[Result \"0-1\"]\n\n1. e4 c5 2. Nf3 d6 3. d4 cxd4 0-1")
        } else {
            format!("[Event \"Test\"]\n[Result \"1/2-1/2\"]\n\n1. d4 Nf6 2. c4 e6 3. Nf3 d5 1/2-1/2")
        };
        scid_db.add_game(&pgn).unwrap();
    }
    scid_db.save().unwrap();

    let games_path = scid_db.games_path().to_path_buf();
    let entries = scid_db.entries();
    let db_path_buf = scid_db.index_path().to_path_buf();

    // Build companion .pos.idx with multi-threading and max_games limit
    let pos_idx = crate::position_index::PositionIndex::build_for_scid(
        &db_path_buf,
        entries,
        &games_path,
        16,
        Some(10),
        None,
        Some(4),
        |_, _, _| {},
    ).expect("build_for_scid should succeed");

    // Scan diagnostics
    let diag = pos_idx.scan_diagnostics().expect("scan_diagnostics should succeed");
    assert!(diag.total_game_sets > 0);

    // Query starting position
    let start_tree = pos_idx.query_tree("").expect("Should find starting position");
    assert_eq!(start_tree.total_games, 100);
    assert_eq!(start_tree.moves.len(), 2); // e4 and d4

    // Query 1. e4
    let fen_e4 = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
    let e4_tree = pos_idx.query_tree(fen_e4).expect("Should find 1. e4");
    assert_eq!(e4_tree.total_games, 67); // 34 (1.e4 e5) + 33 (1.e4 c5)
    assert_eq!(e4_tree.moves.len(), 2); // e5 and c5

    // Query 1. d4
    let fen_d4 = "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq d3 0 1";
    let d4_tree = pos_idx.query_tree(fen_d4).expect("Should find 1. d4");
    assert_eq!(d4_tree.total_games, 33);
    assert_eq!(d4_tree.moves.len(), 1); // Nf6
}

#[test]
fn test_candidate_acceleration_correctness() {
    let dir = tempdir().unwrap();
    let scid_path = dir.path().join("candidate_test.si5");
    let mut scid_db = crate::db::ScidDatabaseWrapper::create(&scid_path, ScidFormat::Si5).unwrap();

    let pgn_samples = [
        "[Event \"WCh\"]\n[White \"Kasparov\"]\n[Black \"Karpov\"]\n[Result \"1-0\"]\n[ECO \"B90\"]\n[Date \"1985.10.15\"]\n\n1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6 1-0",
        "[Event \"WCh\"]\n[White \"Karpov\"]\n[Black \"Kasparov\"]\n[Result \"0-1\"]\n[ECO \"B90\"]\n[Date \"1985.10.17\"]\n\n1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6 0-1",
        "[Event \"Candidates\"]\n[White \"Fischer\"]\n[Black \"Petrosian\"]\n[Result \"1-0\"]\n[ECO \"B90\"]\n[Date \"1971.10.01\"]\n\n1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6 1-0",
        "[Event \"Olympiad\"]\n[White \"Tal\"]\n[Black \"Larsen\"]\n[Result \"1/2-1/2\"]\n[ECO \"C50\"]\n[Date \"1965.05.20\"]\n\n1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 1/2-1/2",
        "[Event \"Candidates\"]\n[White \"Kasparov\"]\n[Black \"Anand\"]\n[Result \"1-0\"]\n[ECO \"C50\"]\n[Date \"1995.09.11\"]\n\n1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 1-0",
    ];

    for pgn in pgn_samples {
        scid_db.add_game(pgn).unwrap();
    }
    scid_db.save().unwrap();

    // 1. Full scan baseline (without .pos.idx)
    let fen_najdorf = "rnbqkb1r/1p2pppp/p2p1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 6";
    let matcher = crate::position_search::parse_position_matcher(fen_najdorf, None, Some("exact")).unwrap();
    let full_scan_matches = crate::position_search::search_position_matcher_mmap_with_progress(
        scid_db.entries(),
        scid_db.games_path(),
        &matcher,
        Some(16),
        |_, _, _| {},
    ).unwrap();
    let full_scan_ids: Vec<usize> = full_scan_matches.iter().map(|m| m.game_id).collect();
    assert_eq!(full_scan_ids, vec![0, 1, 2]);

    // 2. Build companion .pos.idx
    let db_path = scid_db.index_path().to_path_buf();
    let games_path = scid_db.games_path().to_path_buf();
    crate::position_index::PositionIndex::build_for_scid(
        &db_path,
        scid_db.entries(),
        &games_path,
        16,
        None,
        None,
        None,
        |_, _, _| {},
    ).unwrap();

    // 3. Approach B: Accelerated Position Search
    let accelerated_res = scid_db.search_position(fen_najdorf, None, None, None).unwrap();
    let accelerated_ids: Vec<usize> = accelerated_res.matches.iter().map(|m| m.game_id).collect();
    assert_eq!(accelerated_ids, full_scan_ids, "Position search results must be 100% identical");

    // 4. Combined Position + Header Filter (FEN + Result 1-0)
    let filter_win = GameFilter {
        fen: Some(fen_najdorf.to_string()),
        result: Some("1-0".to_string()),
        ..Default::default()
    };
    let (games_win, total_win) = scid_db.query_games(&filter_win, 0, 10);
    assert_eq!(total_win, 2);
    assert_eq!(games_win.len(), 2);
    assert_eq!(games_win[0].id, 0); // Kasparov vs Karpov 1-0
    assert_eq!(games_win[1].id, 2); // Fischer vs Petrosian 1-0

    // 5. Combined Position + White Player Filter (FEN + White "Kasparov")
    let filter_kasparov = GameFilter {
        fen: Some(fen_najdorf.to_string()),
        white: Some("Kasparov".to_string()),
        ..Default::default()
    };
    let (games_kasp, total_kasp) = scid_db.query_games(&filter_kasparov, 0, 10);
    assert_eq!(total_kasp, 1);
    assert_eq!(games_kasp[0].id, 0);

    // 6. Italian Game Search (1.e4 e5 2.Nf3 Nc6 3.Bc4 Bc5)
    let fen_italian = "r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4";
    let filter_italian = GameFilter {
        fen: Some(fen_italian.to_string()),
        ..Default::default()
    };
    let (games_ita, total_ita) = scid_db.query_games(&filter_italian, 0, 10);
    assert_eq!(total_ita, 2);
    assert_eq!(games_ita[0].id, 3);
    assert_eq!(games_ita[1].id, 4);
}

#[test]
fn test_pos_idx_min_games_filter() {
    let dir = tempdir().unwrap();
    let scid_path = dir.path().join("min_games_test.si5");
    let mut scid_db = crate::db::ScidDatabaseWrapper::create(&scid_path, ScidFormat::Si5).unwrap();

    // 5 games with 1.e4, 3 games with 1.d4, and 1 rare game with 1.b4 (Sokolsky/Polish)
    for _ in 0..5 {
        scid_db.add_game("[Event \"Test\"]\n\n1. e4 e5 2. Nf3 Nc6 *").unwrap();
    }
    for _ in 0..3 {
        scid_db.add_game("[Event \"Test\"]\n\n1. d4 d5 2. c4 c6 *").unwrap();
    }
    scid_db.add_game("[Event \"Rare\"]\n\n1. b4 e5 2. Bb2 Bxb4 *").unwrap();
    scid_db.save().unwrap();

    let db_path = scid_db.index_path().to_path_buf();
    let games_path = scid_db.games_path().to_path_buf();

    // 1. Build index with min_games = 1 (default: everything indexed)
    let full_idx = crate::position_index::PositionIndex::build_for_scid(
        &db_path,
        scid_db.entries(),
        &games_path,
        16,
        None,
        Some(1),
        None,
        |_, _, _| {},
    ).unwrap();

    let fen_b4 = "rnbqkbnr/pppppppp/8/8/1P6/8/P1PPPPPP/RNBQKBNR b KQkq b3 0 1";
    assert!(full_idx.query_tree(fen_b4).is_some(), "Rare position 1.b4 should exist when min_games=1");
    let full_unique = full_idx.header.unique_positions;

    // 2. Rebuild index with min_games = 3 (filter out positions appearing < 3 times)
    let filtered_idx = crate::position_index::PositionIndex::build_for_scid(
        &db_path,
        scid_db.entries(),
        &games_path,
        16,
        None,
        Some(3),
        None,
        |_, _, _| {},
    ).unwrap();

    assert!(filtered_idx.header.unique_positions < full_unique, "Filtered unique positions count must be smaller");
    assert!(filtered_idx.query_tree(fen_b4).is_none(), "Rare position 1.b4 (only 1 game) must be excluded when min_games=3");

    // 1.e4 (5 games) and 1.d4 (3 games) must remain indexed
    let fen_e4 = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
    let fen_d4 = "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq d3 0 1";
    assert!(filtered_idx.query_tree(fen_e4).is_some(), "1.e4 (5 games) should be indexed");
    assert!(filtered_idx.query_tree(fen_d4).is_some(), "1.d4 (3 games) should be indexed");
}




