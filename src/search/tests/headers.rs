use super::fixtures::*;
use crate::search::*;

#[test]
fn test_header_metadata_search() {
    // 1. Match Player Name
    let q_player = SearchQuery::Header(HeaderPredicate::Player {
        name: "Morphy".to_string(),
        op: ComparisonOp::Contains,
        case_sensitive: false,
    });
    assert!(GameSearchEvaluator::evaluate_pgn(&q_player, OPERA_GAME).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_player, KASPAROV_TOPALOV).is_match);

    // 2. Match Elo Rating Range
    let q_elo = SearchQuery::Header(HeaderPredicate::WhiteElo {
        op: ComparisonOp::GreaterThan,
        value: 2800,
    });
    assert!(GameSearchEvaluator::evaluate_pgn(&q_elo, KASPAROV_TOPALOV).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_elo, OPERA_GAME).is_match);

    // 3. Match Elo Difference (Kasparov 2812 vs Topalov 2700 = diff 112)
    let q_elo_diff = SearchQuery::Header(HeaderPredicate::EloDiff {
        op: ComparisonOp::GreaterThan,
        value: 100,
        absolute: true,
    });
    assert!(GameSearchEvaluator::evaluate_pgn(&q_elo_diff, KASPAROV_TOPALOV).is_match);
    assert!(GameSearchEvaluator::evaluate_pgn(&q_elo_diff, OPERA_GAME).is_match); // 2700 - 2300 = 400

    // 4. Match ECO Prefix
    let q_eco = SearchQuery::Header(HeaderPredicate::Eco {
        code: "B8".to_string(),
        op: ComparisonOp::StartsWith,
    });
    assert!(GameSearchEvaluator::evaluate_pgn(&q_eco, KASPAROV_TOPALOV).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_eco, OPERA_GAME).is_match);

    // 5. Match Date (Partial Year, Year.Month, Full Date, Wildcards, Comparisons)
    let q_year_only = QueryParser::parse_str("date == 1999").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_year_only, KASPAROV_TOPALOV).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_year_only, OPERA_GAME).is_match);

    let q_year_month = QueryParser::parse_str(r#"date == "1858.11""#).unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_year_month, OPERA_GAME).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_year_month, KASPAROV_TOPALOV).is_match);

    let q_date_gte = QueryParser::parse_str("date >= 1990").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_date_gte, KASPAROV_TOPALOV).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_date_gte, OPERA_GAME).is_match);

    let q_date_lte = QueryParser::parse_str("date <= 1900").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_date_lte, OPERA_GAME).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_date_lte, KASPAROV_TOPALOV).is_match);

    // Test game with unknown PGN date components e.g. "1858.??.??"
    let unknown_date_pgn = r#"[Event "Paris"]
[Site "Paris"]
[Date "1858.??.??"]
[White "Morphy"]
[Black "Allies"]
[Result "1-0"]

1. e4 e5 1-0
"#;
    let q_unknown_match = QueryParser::parse_str("date == 1858").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_unknown_match, unknown_date_pgn).is_match);

    // 6. Regular Expression Player Match (~ operator & regex() syntax)
    let q_regex1 = QueryParser::parse_str(r#"player ~ "(?i)kasparov|morphy""#).unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_regex1, KASPAROV_TOPALOV).is_match);
    assert!(GameSearchEvaluator::evaluate_pgn(&q_regex1, OPERA_GAME).is_match);

    let q_regex2 = QueryParser::parse_str(r#"white:regex("Garry.*")"#).unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_regex2, KASPAROV_TOPALOV).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_regex2, OPERA_GAME).is_match);

    let q_regex3 = QueryParser::parse_str(r#"black ~ "Duke|Count""#).unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_regex3, OPERA_GAME).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_regex3, KASPAROV_TOPALOV).is_match);
}

#[test]
fn test_tag_and_custom_headers() {
    let pgn_with_custom_tags = r#"[Event "Blitz Match"]
[Site "Chess.com"]
[Date "2024.03.15"]
[White "Alexander"]
[Black "Opponent"]
[Result "1-0"]
[TimeControl "300"]
[Annotator "Stockfish 16"]
[Termination "Normal"]

1. e4 e5 2. Nf3 1-0
"#;

    // 1. Tag equality: tag "TimeControl" == "300"
    let q1 = QueryParser::parse_str(r#"tag "TimeControl" == "300""#).unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q1, pgn_with_custom_tags).is_match);

    // 2. Header alias: header "Annotator" contains "Stockfish"
    let q2 = QueryParser::parse_str(r#"header "Annotator" contains "Stockfish""#).unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q2, pgn_with_custom_tags).is_match);

    // 3. Tag inequality / mismatch
    let q3_fail = QueryParser::parse_str(r#"tag "TimeControl" == "600""#).unwrap();
    assert!(!GameSearchEvaluator::evaluate_pgn(&q3_fail, pgn_with_custom_tags).is_match);

    // 4. Combined tag and move query
    let q4 = QueryParser::parse_str(r#"tag "TimeControl" == "300" and line [e4 e5 Nf3]"#).unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q4, pgn_with_custom_tags).is_match);
}

#[test]
fn test_scid_index_entry_header_prefiltering() {
    let dir = tempfile::tempdir().unwrap();
    let si5_path = dir.path().join("prefilter_test.si5");
    let mut scid_db =
        crate::db::ScidDatabaseWrapper::create(&si5_path, crate::db::ScidFormat::Si5).unwrap();

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
        evaluator::quick_check_entry_headers(&q_white, entry, names),
        Some(true)
    );
    assert_eq!(
        evaluator::quick_check_entry_headers(&q_wrong_white, entry, names),
        Some(false)
    );
    assert_eq!(
        evaluator::quick_check_entry_headers(&q_elo, entry, names),
        Some(true)
    );
    assert_eq!(
        evaluator::quick_check_entry_headers(&q_wrong_elo, entry, names),
        Some(false)
    );
    assert_eq!(
        evaluator::quick_check_entry_headers(&q_eco, entry, names),
        Some(true)
    );
    assert_eq!(
        evaluator::quick_check_entry_headers(&q_wrong_eco, entry, names),
        Some(false)
    );
    assert_eq!(
        evaluator::quick_check_entry_headers(&q_result, entry, names),
        Some(true)
    );
    assert_eq!(
        evaluator::quick_check_entry_headers(&q_wrong_result, entry, names),
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
    let pgn_db = crate::pgn_db::PgnDatabaseWrapper::open(&pgn_path).unwrap();

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
        evaluator::quick_check_pgn_entry_headers(&q_white, entry, names),
        Some(true)
    );
    assert_eq!(
        evaluator::quick_check_pgn_entry_headers(&q_wrong_white, entry, names),
        Some(false)
    );
    assert_eq!(
        evaluator::quick_check_pgn_entry_headers(&q_elo, entry, names),
        Some(true)
    );
    assert_eq!(
        evaluator::quick_check_pgn_entry_headers(&q_wrong_elo, entry, names),
        Some(false)
    );
    assert_eq!(
        evaluator::quick_check_pgn_entry_headers(&q_eco, entry, names),
        Some(true)
    );
    assert_eq!(
        evaluator::quick_check_pgn_entry_headers(&q_wrong_eco, entry, names),
        Some(false)
    );
    assert_eq!(
        evaluator::quick_check_pgn_entry_headers(&q_result, entry, names),
        Some(true)
    );
    assert_eq!(
        evaluator::quick_check_pgn_entry_headers(&q_wrong_result, entry, names),
        Some(false)
    );

    // Also test full search execution
    let results_match = pgn_db.search_query(&q_white);
    assert_eq!(results_match.len(), 1);
    let results_miss = pgn_db.search_query(&q_wrong_white);
    assert_eq!(results_miss.len(), 0);
}

#[test]
fn test_comments_and_nag_annotations() {
    let annotated_pgn = r#"[Event "Test Annotations"]
[Site "Online"]
[Date "2024.01.01"]
[Round "1"]
[White "Player 1"]
[Black "Player 2"]
[Result "1-0"]

1. e4 {Novelty in this line} e5 2. Nf3! Nc6 3. Bc4 $3 Bc5 4. Bxf7+?? Kxf7 1-0
"#;

    // 1. Match comment text
    let dsl_comment = r#"comment:"novelty""#;
    let q_comment = QueryParser::parse_str(dsl_comment).expect("Failed to parse comment query");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_comment, annotated_pgn).is_match);

    // 2. Match NAG symbol ! (NAG 1)
    let dsl_nag_excl = r#"nag:!"#;
    let q_nag = QueryParser::parse_str(dsl_nag_excl).expect("Failed to parse nag query");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_nag, annotated_pgn).is_match);

    // 3. Match NAG blunder ?? (NAG 4)
    let dsl_nag_blunder = r#"nag:[4]"#;
    let q_blunder = QueryParser::parse_str(dsl_nag_blunder).expect("Failed to parse blunder query");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_blunder, annotated_pgn).is_match);

    // 4. Strip comments utility
    let stripped = AnnotationManager::strip_comments(annotated_pgn);
    assert!(!stripped.contains("Novelty in this line"));
    assert!(stripped.contains("1. e4"));
}
