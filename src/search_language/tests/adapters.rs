use crate::search::*;

#[test]
fn test_zero_copy_scid_adapter() {
    use crate::pgn_utils::FastNameTables;
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
fn test_alex_pgn_queries() {
    let pgn_path = std::path::Path::new("C:/Users/ASUS/chess/database/games/chesscom/alex.pgn");
    if !pgn_path.exists() {
        return;
    }

    let pgn_db = crate::pgn_db::PgnDatabaseWrapper::open(pgn_path).unwrap();
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
        "Ph7 query must find matching games in alex.pgn"
    );
    assert_eq!(
        res_bare.len(),
        res_ph7.len(),
        "Both queries should match the exact same number of games"
    );
    let q_nxne5 = QueryParser::parse_str("path [Nxne5]").unwrap();
    let res_nxne5 = pgn_db.search_query(&q_nxne5);
    assert!(
        !res_nxne5.is_empty(),
        "Nxne5 (White Knight captures Black Knight on e5) must match games in alex.pgn"
    );
}
