use cql_lang::parser::QueryParser;
use cql_lang::query::*;
use scid_mgr::db::{ScidDatabaseWrapper, ScidFormat};
use scid_mgr::pgn_db::PgnDatabaseWrapper;
use scid_mgr::pgn_io::FastNameTables;
use scid_mgr::search_adapter::{
    quick_check_entry_headers, quick_check_pgn_entry_headers, ScidSearchAdapter,
};

#[test]
fn test_zero_copy_scid_adapter() {
    use chess_scid_rw::entry::IndexEntry;

    let mut fast_names =
        FastNameTables::from_name_tables(&chess_scid_rw::names::NameTables::default());
    let p1 = fast_names.player_id("Morphy, Paul");
    let p2 = fast_names.player_id("Allies");
    let names = fast_names.to_name_tables();

    let entry = IndexEntry {
        offset: 0,
        length: 0,
        white_id: p1,
        black_id: p2,
        event_id: 0,
        site_id: 0,
        round_id: 0,
        date: 0,
        result: 1, // 1-0
        eco_code: 0,
        white_elo: 2700,
        black_elo: 2300,
        non_standard_start: false,
        deleted: false,
    };

    let q = SearchQuery::Header(HeaderPredicate::White {
        name: "Morphy".to_string(),
        op: ComparisonOp::Contains,
        case_sensitive: false,
    });

    let res = ScidSearchAdapter::evaluate_scid_game(&q, &entry, &names, &[]);
    assert!(
        res.is_match,
        "SCID Adapter should match header in zero-copy mode"
    );
}

#[test]
fn test_scid_index_entry_header_prefiltering() {
    let dir = tempfile::tempdir().unwrap();
    let si5_path = dir.path().join("prefilter_test.si5");
    let mut scid_db = ScidDatabaseWrapper::create(&si5_path, ScidFormat::Si5).unwrap();

    let sample_game = r#"[Event "Tata Steel Masters"]
[Site "Wijk aan Zee"]
[Date "2023.01.18"]
[Round "5"]
[White "Carlsen, Magnus"]
[Black "Nakamura, Hikaru"]
[Result "1-0"]
[ECO "C65"]
[WhiteElo "2859"]
[BlackElo "2768"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 Nf6 1-0
"#;

    scid_db.add_game(sample_game).unwrap();
    let entry = &scid_db.entries()[0];
    let names = scid_db.names();

    let q_white = QueryParser::parse_str(r#"white "Carlsen""#).unwrap();
    let q_wrong_white = QueryParser::parse_str(r#"white "Kasparov""#).unwrap();
    let q_elo = QueryParser::parse_str(r#"white_elo >= 2800 and black_elo >= 2750"#).unwrap();
    let q_wrong_elo = QueryParser::parse_str(r#"white_elo < 2700"#).unwrap();
    let q_eco = QueryParser::parse_str(r#"eco "C65""#).unwrap();
    let q_wrong_eco = QueryParser::parse_str(r#"eco "B90""#).unwrap();
    let q_result = QueryParser::parse_str(r#"result "1-0""#).unwrap();
    let q_wrong_result = QueryParser::parse_str(r#"result "0-1""#).unwrap();

    assert_eq!(
        quick_check_entry_headers(&q_white, entry, names),
        Some(true)
    );
    assert_eq!(
        quick_check_entry_headers(&q_wrong_white, entry, names),
        Some(false)
    );
    assert_eq!(quick_check_entry_headers(&q_elo, entry, names), Some(true));
    assert_eq!(
        quick_check_entry_headers(&q_wrong_elo, entry, names),
        Some(false)
    );
    assert_eq!(quick_check_entry_headers(&q_eco, entry, names), Some(true));
    assert_eq!(
        quick_check_entry_headers(&q_wrong_eco, entry, names),
        Some(false)
    );
    assert_eq!(
        quick_check_entry_headers(&q_result, entry, names),
        Some(true)
    );
    assert_eq!(
        quick_check_entry_headers(&q_wrong_result, entry, names),
        Some(false)
    );
}

#[test]
fn test_pgn_index_entry_header_prefiltering() {
    let dir = tempfile::tempdir().unwrap();
    let pgn_path = dir.path().join("pgn_prefilter_test.pgn");

    let sample_game = r#"[Event "Tata Steel Masters"]
[Site "Wijk aan Zee"]
[Date "2023.01.18"]
[Round "5"]
[White "Carlsen, Magnus"]
[Black "Nakamura, Hikaru"]
[Result "1-0"]
[ECO "C65"]
[WhiteElo "2859"]
[BlackElo "2768"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 Nf6 1-0
"#;

    std::fs::write(&pgn_path, sample_game).unwrap();
    let pgn_db = PgnDatabaseWrapper::open(&pgn_path).unwrap();

    let entry = &pgn_db.entries[0];
    let names = &pgn_db.names;

    let q_white = QueryParser::parse_str(r#"white "Carlsen""#).unwrap();
    let q_wrong_white = QueryParser::parse_str(r#"white "Kasparov""#).unwrap();
    let q_elo = QueryParser::parse_str(r#"white_elo >= 2800 and black_elo >= 2750"#).unwrap();
    let q_wrong_elo = QueryParser::parse_str(r#"white_elo < 2700"#).unwrap();
    let q_eco = QueryParser::parse_str(r#"eco "C65""#).unwrap();
    let q_wrong_eco = QueryParser::parse_str(r#"eco "B90""#).unwrap();
    let q_result = QueryParser::parse_str(r#"result "1-0""#).unwrap();
    let q_wrong_result = QueryParser::parse_str(r#"result "0-1""#).unwrap();

    assert_eq!(
        quick_check_pgn_entry_headers(&q_white, entry, names),
        Some(true)
    );
    assert_eq!(
        quick_check_pgn_entry_headers(&q_wrong_white, entry, names),
        Some(false)
    );
    assert_eq!(
        quick_check_pgn_entry_headers(&q_elo, entry, names),
        Some(true)
    );
    assert_eq!(
        quick_check_pgn_entry_headers(&q_wrong_elo, entry, names),
        Some(false)
    );
    assert_eq!(
        quick_check_pgn_entry_headers(&q_eco, entry, names),
        Some(true)
    );
    assert_eq!(
        quick_check_pgn_entry_headers(&q_wrong_eco, entry, names),
        Some(false)
    );
    assert_eq!(
        quick_check_pgn_entry_headers(&q_result, entry, names),
        Some(true)
    );
    assert_eq!(
        quick_check_pgn_entry_headers(&q_wrong_result, entry, names),
        Some(false)
    );
}

#[test]
fn test_alex_pgn_queries() {
    let pgn_path = std::path::Path::new("C:/Users/ASUS/chess/database/games/chesscom/alex.pgn");
    if !pgn_path.exists() {
        return;
    }

    let pgn_db = PgnDatabaseWrapper::open(pgn_path).unwrap();
    let q_bare = QueryParser::parse_str("Ph4 and path [... h7]").unwrap();
    let q_ph7 = QueryParser::parse_str("Ph4 and path [... Ph7]").unwrap();

    let res_bare = pgn_db.search_query(&q_bare);
    let res_ph7 = pgn_db.search_query(&q_ph7);

    assert!(
        !res_bare.is_empty(),
        "Bare h7 query must find matching games in alex.pgn"
    );
    assert!(
        !res_ph7.is_empty(),
        "Explicit Ph7 query must find matching games in alex.pgn"
    );
}
