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
}
