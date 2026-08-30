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
    let mut filter = GameFilter::default();
    filter.player = Some("Morphy".to_string());
    let (results, count) = db.query_games(&filter, 0, 10);
    assert_eq!(count, 1);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].white, "Paul Morphy");

    filter = GameFilter::default();
    filter.eco = Some("B".to_string());
    let (results, count) = db.query_games(&filter, 0, 10);
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
