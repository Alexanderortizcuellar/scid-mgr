use super::*;
use shakmaty::{Color, Piece, Role, Square};
use std::collections::HashMap;

const OPERA_GAME: &str = r#"[Event "Paris"]
[Site "Paris"]
[Date "1858.11.02"]
[Round "?"]
[White "Paul Morphy"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]
[ECO "C41"]
[WhiteElo "2700"]
[BlackElo "2300"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7
8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7
14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0
"#;

const KASPAROV_TOPALOV: &str = r#"[Event "Hoogovens Group A"]
[Site "Wijk aan Zee NED"]
[Date "1999.01.20"]
[Round "4"]
[White "Garry Kasparov"]
[Black "Veselin Topalov"]
[Result "1-0"]
[ECO "B88"]
[WhiteElo "2812"]
[BlackElo "2700"]

1. e4 d6 2. d4 Nf6 3. Nc3 g6 4. Be3 Bg7 5. Qd2 c6 6. f3 b5 7. Nge2 Nbd7 8. Bh6
Bxh6 9. Qxh6 Bb7 10. a3 e5 11. O-O-O Qe7 12. Kb1 a6 13. Nc1 O-O-O 14. Nb3 exd4
15. Rxd4 c5 16. Rd1 Nb6 17. g3 Kb8 18. Na5 Ba8 19. Bh3 d5 20. Qf4+ Ka7 21. Rhe1
d4 22. Nd5 Nbxd5 23. exd5 Qd6 24. Rxd4 cxd4 25. Re7+ Kb6 26. Qxd4+ Kxa5 27. b4+
Ka4 28. Qc3 Qxd5 29. Ra7 Bb7 30. Rxb7 Qc4 31. Qxf6 Kxa3 32. Qxa6+ Kxb4 33. c3+
Kxc3 34. Qa1+ Kd2 35. Qb2+ Kd1 36. Bf1 Rd2 37. Rd7 Rxd7 38. Bxc4 bxc4 39. Qxh8
Rd3 40. Qa8 c3 41. Qa4+ Ke1 42. f4 f5 43. Kc1 Rd2 44. Qa7 1-0
"#;

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
fn test_position_pattern_search() {
    // 1. Piece on specific square (White Queen on d8 in Opera checkmate)
    let mut squares = HashMap::new();
    squares.insert(
        Square::D8,
        SquareContent::Piece(Piece {
            color: Color::White,
            role: Role::Rook,
        }),
    );
    let q_rook_d8 = SearchQuery::Position(PositionPattern::Squares(squares));

    let res_opera = GameSearchEvaluator::evaluate_pgn(&q_rook_d8, OPERA_GAME);
    assert!(
        res_opera.is_match,
        "Opera Game should have White Rook on d8 at move 17"
    );
    assert_eq!(res_opera.matching_plies, vec![33]); // Ply 33 = 17. Rd8#

    // 2. Checkmate state
    let q_mate = SearchQuery::Position(PositionPattern::BoardState {
        is_check: None,
        is_checkmate: Some(true),
        is_stalemate: None,
    });
    assert!(GameSearchEvaluator::evaluate_pgn(&q_mate, OPERA_GAME).is_match);

    // 3. Piece count (White has 0 Queens on board in Opera game after Queen sacrifice 16. Qb8+)
    let q_no_white_queens = SearchQuery::Position(PositionPattern::PieceCount {
        content: SquareContent::Piece(Piece {
            color: Color::White,
            role: Role::Queen,
        }),
        squares: None,
        op: ComparisonOp::Equal,
        count: 0,
    });
    let res_opera_no_q = GameSearchEvaluator::evaluate_pgn(&q_no_white_queens, OPERA_GAME);
    assert!(
        res_opera_no_q.is_match,
        "Opera game reached position with 0 White queens after move 16... Nxb8"
    );
    assert_eq!(res_opera_no_q.matching_plies, vec![32, 33]); // Plies 32 (16... Nxb8) and 33 (17. Rd8#)

    // 4. Black has 0 Queens in Kasparov vs Topalov endgame
    let q_no_black_queens = SearchQuery::Position(PositionPattern::PieceCount {
        content: SquareContent::Piece(Piece {
            color: Color::Black,
            role: Role::Queen,
        }),
        squares: None,
        op: ComparisonOp::Equal,
        count: 0,
    });
    assert!(GameSearchEvaluator::evaluate_pgn(&q_no_black_queens, KASPAROV_TOPALOV).is_match);

    // 5. Material Predicate: White has exactly 1 Rook and 0 Queens at checkmate
    let q_material = SearchQuery::Material(MaterialPredicate {
        white_knights: Some(0),
        white_bishops: Some(1),
        white_rooks: Some(1),
        white_queens: Some(0),
        ..Default::default()
    });
    assert!(GameSearchEvaluator::evaluate_pgn(&q_material, OPERA_GAME).is_match);
}

#[test]
fn test_move_pattern_and_path_sequence_search() {
    // 1. Single Move Pattern: Queen sacrifice on b8
    let q_qb8 = SearchQuery::Move(MovePattern {
        san: Some("Qb8+".to_string()),
        ..Default::default()
    });
    let res = GameSearchEvaluator::evaluate_pgn(&q_qb8, OPERA_GAME);
    assert!(res.is_match, "Opera game contains Qb8+");
    assert_eq!(res.matching_plies, vec![31]);

    // 2. Consecutive Move Sequence Path: 1. e4 e5 2. Nf3 d6
    let q_path = SearchQuery::Path(PathPattern {
        moves: vec![
            MovePattern {
                san: Some("e4".to_string()),
                ..Default::default()
            },
            MovePattern {
                san: Some("e5".to_string()),
                ..Default::default()
            },
            MovePattern {
                san: Some("Nf3".to_string()),
                ..Default::default()
            },
            MovePattern {
                san: Some("d6".to_string()),
                ..Default::default()
            },
        ],
        consecutive: true,
        max_gap_plies: None,
        start_ply_range: Some(0..2),
        ..Default::default()
    });

    assert!(GameSearchEvaluator::evaluate_pgn(&q_path, OPERA_GAME).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_path, KASPAROV_TOPALOV).is_match);

    // 3. Non-consecutive sequence / theme: [e4] -> ... -> [Bg5] -> ... -> [Rd8#]
    let q_theme = SearchQuery::Path(PathPattern {
        moves: vec![
            MovePattern {
                san: Some("e4".to_string()),
                ..Default::default()
            },
            MovePattern {
                san: Some("Bg5".to_string()),
                ..Default::default()
            },
            MovePattern {
                san: Some("Rd8#".to_string()),
                ..Default::default()
            },
        ],
        consecutive: false,
        max_gap_plies: Some(30),
        start_ply_range: None,
        ..Default::default()
    });
    assert!(GameSearchEvaluator::evaluate_pgn(&q_theme, OPERA_GAME).is_match);
}

#[test]
fn test_composite_boolean_and_ply_range_queries() {
    // 1. AND Query: Kasparov as White AND ECO B88 AND reaches move Rd7
    let q_composite = SearchQuery::And(vec![
        SearchQuery::Header(HeaderPredicate::White {
            name: "Kasparov".to_string(),
            op: ComparisonOp::Contains,
            case_sensitive: false,
        }),
        SearchQuery::Header(HeaderPredicate::Eco {
            code: "B88".to_string(),
            op: ComparisonOp::Equal,
        }),
        SearchQuery::Move(MovePattern {
            san: Some("Rd7".to_string()),
            ..Default::default()
        }),
    ]);
    assert!(GameSearchEvaluator::evaluate_pgn(&q_composite, KASPAROV_TOPALOV).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_composite, OPERA_GAME).is_match);

    // 2. NOT Query: Morphy played White but result was NOT Draw
    let q_not_draw = SearchQuery::And(vec![
        SearchQuery::Header(HeaderPredicate::White {
            name: "Morphy".to_string(),
            op: ComparisonOp::Contains,
            case_sensitive: false,
        }),
        SearchQuery::Not(Box::new(SearchQuery::Header(HeaderPredicate::Result {
            expected: "1/2-1/2".to_string(),
        }))),
    ]);
    assert!(GameSearchEvaluator::evaluate_pgn(&q_not_draw, OPERA_GAME).is_match);

    // 2b. DSL Negation: `not check`, `not attacks(K, p)`, `not empty on e4`
    let q_dsl_not_check =
        QueryParser::parse_str("turn white and not check and white_pieces on d1").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_dsl_not_check, OPERA_GAME).is_match);

    let q_dsl_not_attacks = QueryParser::parse_str("not attacks [wk, bp]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_dsl_not_attacks, OPERA_GAME).is_match);

    let q_dsl_not_empty = QueryParser::parse_str("not empty on e4").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_dsl_not_empty, OPERA_GAME).is_match);

    // 3. Ply Range: Move Bg4 must happen in opening (first 10 plies)
    let q_early_bg4 = SearchQuery::PlyRange {
        range: 0..10,
        query: Box::new(SearchQuery::Move(MovePattern {
            san: Some("Bg4".to_string()),
            ..Default::default()
        })),
    };
    assert!(GameSearchEvaluator::evaluate_pgn(&q_early_bg4, OPERA_GAME).is_match);
    // Ply 6 (3... Bg4)
}

#[test]
fn test_pawn_structure_search() {
    // 1. Passed Pawn search: Black has a passed pawn in Kasparov-Topalov (after 40... c3)
    let q_black_passed = SearchQuery::Pawn(PawnPredicate::PassedPawns {
        color: Color::Black,
        op: ComparisonOp::GreaterThanOrEqual,
        count: 1,
    });
    let res = GameSearchEvaluator::evaluate_pgn(&q_black_passed, KASPAROV_TOPALOV);
    assert!(
        res.is_match,
        "Kasparov-Topalov features Black passed pawn on c-file in endgame"
    );

    // 2. Doubled Pawns: Black has doubled pawns in Kasparov-Topalov on c-file (after 38. Bxc4 bxc4)
    let q_black_doubled = SearchQuery::Pawn(PawnPredicate::DoubledPawns {
        color: Color::Black,
        op: ComparisonOp::GreaterThanOrEqual,
        count: 1,
    });
    let res_doubled = GameSearchEvaluator::evaluate_pgn(&q_black_doubled, KASPAROV_TOPALOV);
    assert!(
        res_doubled.is_match,
        "Black has doubled pawns on c-file after move 38"
    );

    // 3. Pawn Islands: At the start of the game, both sides have 1 pawn island
    let q_white_islands = SearchQuery::PlyRange {
        range: 0..1,
        query: Box::new(SearchQuery::Pawn(PawnPredicate::PawnIslands {
            color: Color::White,
            op: ComparisonOp::Equal,
            count: 1,
        })),
    };
    assert!(GameSearchEvaluator::evaluate_pgn(&q_white_islands, OPERA_GAME).is_match);
}

#[test]
fn test_cql_dsl_query_parser() {
    // 1. Parse simple header & checkmate query
    let dsl_1 = r#"cql ( white "Morphy" mate )"#;
    let q1 = QueryParser::parse_str(dsl_1).expect("Failed to parse DSL 1");
    assert!(GameSearchEvaluator::evaluate_pgn(&q1, OPERA_GAME).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q1, KASPAROV_TOPALOV).is_match);

    // 2. Parse composite Elo + ECO + move sequence query
    let dsl_2 = r#"player "Kasparov" elo >= 2800 eco: "B88" move "Rd7""#;
    let q2 = QueryParser::parse_str(dsl_2).expect("Failed to parse DSL 2");
    assert!(GameSearchEvaluator::evaluate_pgn(&q2, KASPAROV_TOPALOV).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q2, OPERA_GAME).is_match);

    // 3. Parse consecutive opening line
    let dsl_3 = r#"line [e4 e5 Nf3 d6]"#;
    let q3 = QueryParser::parse_str(dsl_3).expect("Failed to parse DSL 3");
    assert!(GameSearchEvaluator::evaluate_pgn(&q3, OPERA_GAME).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q3, KASPAROV_TOPALOV).is_match);

    // 4. Parse gapped theme path
    let dsl_4 = r#"path [e4 ... Bg5 ... Rd8#]"#;
    let q4 = QueryParser::parse_str(dsl_4).expect("Failed to parse DSL 4");
    assert!(GameSearchEvaluator::evaluate_pgn(&q4, OPERA_GAME).is_match);

    // 5. Parse pawn structure query: Black has passed pawn
    let dsl_5 = r#"passedpawns black >= 1"#;
    let q5 = QueryParser::parse_str(dsl_5).expect("Failed to parse DSL 5");
    assert!(GameSearchEvaluator::evaluate_pgn(&q5, KASPAROV_TOPALOV).is_match);

    // 6. Parse piece on square query: White Rook on d8
    let dsl_6 = r#"piece wr on d8 checkmate"#;
    let q6 = QueryParser::parse_str(dsl_6).expect("Failed to parse DSL 6");
    assert!(GameSearchEvaluator::evaluate_pgn(&q6, OPERA_GAME).is_match);

    // 7. Parse boolean logic with NOT and OR
    let dsl_7 = r#"white "Morphy" and not result "1/2-1/2""#;
    let q7 = QueryParser::parse_str(dsl_7).expect("Failed to parse DSL 7");
    assert!(GameSearchEvaluator::evaluate_pgn(&q7, OPERA_GAME).is_match);
}

#[test]
fn test_tactical_motifs_and_geometry() {
    // 1. Pin: White Bishop pins Black Knight against King in Opera Game (9. Bg5 / 14... Qe6 / 15. Bxd7+)
    let dsl_pin = r#"cql ( pin [wb, bn, bk] )"#;
    let q_pin = QueryParser::parse_str(dsl_pin).expect("Failed to parse pin DSL");
    let res_pin = GameSearchEvaluator::evaluate_pgn(&q_pin, OPERA_GAME);
    assert!(
        res_pin.is_match,
        "Opera game contains White Bishop pinning Black Knight against King"
    );

    // 2. Generic Pin test: any pin
    let dsl_any_pin = r#"pin"#;
    let q_any_pin = QueryParser::parse_str(dsl_any_pin).expect("Failed to parse any pin");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_any_pin, OPERA_GAME).is_match);

    // 3. Chebyshev Distance between Kings in Endgame
    let dsl_dist = r#"distance(wk, bk) <= 2"#;
    let q_dist = QueryParser::parse_str(dsl_dist).expect("Failed to parse distance");
    let res_dist = GameSearchEvaluator::evaluate_pgn(&q_dist, KASPAROV_TOPALOV);
    assert!(
        res_dist.is_match,
        "Kasparov-Topalov endgame brings Kings close together"
    );

    // 4. Rook on 7th Rank
    let dsl_rook7 = r#"rook_7th white"#;
    let q_rook7 = QueryParser::parse_str(dsl_rook7).expect("Failed to parse rook_7th");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_rook7, KASPAROV_TOPALOV).is_match); // Kasparov played 25. Re7+

    // 5. Open File Search
    let dsl_open_d = r#"open_file [d]"#;
    let q_open_d = QueryParser::parse_str(dsl_open_d).expect("Failed to parse open_file");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_open_d, OPERA_GAME).is_match);
}

#[test]
fn test_multi_square_and_symmetry_transformations() {
    // 1. Multi-square piece matching: White Knight on any of [c3, b5, f3]
    let dsl_multi = r#"piece wn on [c3, b5, f3]"#;
    let q_multi = QueryParser::parse_str(dsl_multi).expect("Failed to parse multi-square");
    let res_multi = GameSearchEvaluator::evaluate_pgn(&q_multi, OPERA_GAME);
    assert!(
        res_multi.is_match,
        "Opera game contains White Knight on c3/b5/f3"
    );

    // 2. Horizontal Symmetry (Left-Right file mirror):
    // Morphy's checkmate with White Rook on d8 mirrors to White Rook on e8 under horizontal flip
    let dsl_sym = r#"flip:horizontal ( piece wr on e8 )"#;
    let q_sym = QueryParser::parse_str(dsl_sym).expect("Failed to parse symmetry");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_sym, OPERA_GAME).is_match);

    // 3. Color Inversion Symmetry
    let dsl_color_sym = r#"flip:color ( piece br on d1 )"#;
    let q_color_sym =
        QueryParser::parse_str(dsl_color_sym).expect("Failed to parse color symmetry");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_color_sym, OPERA_GAME).is_match);
}

#[test]
fn test_wildcard_fen_and_casing_conventions() {
    // 1. Standard chess casing: uppercase N = White Knight, lowercase n = Black Knight
    let dsl_casing = r#"piece N on c3 and piece n on f6"#;
    let q_casing = QueryParser::parse_str(dsl_casing).expect("Failed to parse casing");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_casing, OPERA_GAME).is_match);

    // 2. Uppercase Q = White Queen, lowercase q = Black Queen
    let dsl_queens = r#"piece Q on b3 and piece q on e7"#;
    let q_queens = QueryParser::parse_str(dsl_queens).expect("Failed to parse queens");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_queens, OPERA_GAME).is_match);

    // 2b. Attacks motif using single-letter case: White Bishop attacks Black Queen
    let dsl_atk = r#"attacks(B, q)"#;
    let q_atk =
        QueryParser::parse_str(dsl_atk).expect("Failed to parse attacks with single letter pieces");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_atk, OPERA_GAME).is_match);

    // 3. Wildcard FEN: Rank wildcards with '*'
    // Match Morphy's starting move 1. e4 e5 (4th rank has e4/e5 pawns, other ranks wildcard)
    let dsl_fen_wildcard = r#"fen "*/*/*/*/*/*/*/*""#;
    let q_fen_all = QueryParser::parse_str(dsl_fen_wildcard).expect("Failed to parse fen wildcard");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_fen_all, OPERA_GAME).is_match);

    // Match 8th rank having White Rook on d8 checkmate: "*R*/*"
    let dsl_fen_mate = r#"fen "*R*/*""#;
    let q_fen_mate = QueryParser::parse_str(dsl_fen_mate).expect("Failed to parse mate fen");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_fen_mate, OPERA_GAME).is_match);

    // 4. Global keywords: white_pieces, black_pieces, occupied
    let dsl_globals = r#"white_pieces count >= 2 and black_pieces count >= 2"#;
    let q_globals = QueryParser::parse_str(dsl_globals).expect("Failed to parse globals");
    assert!(GameSearchEvaluator::evaluate_pgn(&q_globals, OPERA_GAME).is_match);
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
fn test_power_expressions() {
    use crate::search::pattern::calculate_power;
    use shakmaty::Board;

    // Starting position power:
    // White: 8*1 (P) + 2*3 (N) + 2*3 (B) + 2*5 (R) + 1*9 (Q) = 8 + 6 + 6 + 10 + 9 = 39.
    // Black: 39. Total = 78.
    let starting_board = Board::default();
    assert_eq!(calculate_power(&starting_board, Color::White), 39);
    assert_eq!(calculate_power(&starting_board, Color::Black), 39);

    // Morphy's Opera Game PGN: White sacrifices material (Bishop on f7 / Knight / Rook / Queen sacrifice)
    let morphy_pgn = r#"[Event "Paris Opera"]
[Site "Paris FRA"]
[Date "1858.??.??"]
[White "Paul Morphy"]
[Black "Duke of Brunswick and Count Isouard"]
[Result "1-0"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7
8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7
14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0
"#;

    // 1. Initial power equals 39
    let q_white_power_39 = QueryParser::parse_str("white_power == 39").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_white_power_39, morphy_pgn).is_match);

    // 2. Total power >= 78 at start
    let q_total_power = QueryParser::parse_str("power >= 78").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_total_power, morphy_pgn).is_match);

    // 3. Side vs side power comparison: white_power > black_power, black_power > white_power
    let q_black_ahead = QueryParser::parse_str("black_power > white_power").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_black_ahead, morphy_pgn).is_match);

    let q_black_more = QueryParser::parse_str("black_power >= white_power").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_black_more, morphy_pgn).is_match);

    // 4. Number-first comparisons: `34 >= black_power`
    let q_num_first = QueryParser::parse_str("34 >= black_power").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_num_first, morphy_pgn).is_match);

    // 5. Power difference query: power_diff >= 5
    let q_diff = QueryParser::parse_str("power_diff >= 5").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_diff, morphy_pgn).is_match);

    // 6. Functional power syntax: power(white) <= 30
    let q_fn_syntax = QueryParser::parse_str("power(white) <= 30").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_fn_syntax, morphy_pgn).is_match);
}

#[test]
fn test_square_ranges_and_diagonal_expansion() {
    // 1. Diagonal ray interpolation: A1 to H8 diagonal
    let dsl_diag_dots = r#"piece B on [a1..h8]"#;
    let q_diag_dots = QueryParser::parse_str(dsl_diag_dots).expect("Failed to parse a1..h8");
    if let SearchQuery::Position(PositionPattern::MultiSquare { squares, .. }) = q_diag_dots {
        assert_eq!(
            squares.len(),
            8,
            "a1..h8 should expand to 8 diagonal squares"
        );
    } else {
        panic!("Expected MultiSquare pattern");
    }

    // 2. Functional diagonal notation: diag(a1, h8) and diag[a1-h8]
    let dsl_diag_fn = r#"piece B on diag(a1, h8)"#;
    let q_diag_fn = QueryParser::parse_str(dsl_diag_fn).expect("Failed to parse diag(a1, h8)");
    if let SearchQuery::Position(PositionPattern::MultiSquare { squares, .. }) = q_diag_fn {
        assert_eq!(squares.len(), 8);
    } else {
        panic!("Expected MultiSquare pattern");
    }

    // 3. Rectangular rank range: a1-h2 (all 16 squares on 1st & 2nd rank)
    let dsl_rank_range = r#"piece K on [a1-h2]"#;
    let q_rank_range = QueryParser::parse_str(dsl_rank_range).expect("Failed to parse [a1-h2]");
    if let SearchQuery::Position(PositionPattern::MultiSquare { squares, .. }) = q_rank_range {
        assert_eq!(squares.len(), 16, "a1-h2 should expand to 16 squares");
    } else {
        panic!("Expected MultiSquare pattern");
    }

    // 4. CQL shorthand rank range: a-h1-2
    let dsl_cql_range = r#"piece K on [a-h1-2]"#;
    let q_cql_range = QueryParser::parse_str(dsl_cql_range).expect("Failed to parse [a-h1-2]");
    if let SearchQuery::Position(PositionPattern::MultiSquare { squares, .. }) = q_cql_range {
        assert_eq!(squares.len(), 16);
    } else {
        panic!("Expected MultiSquare pattern");
    }

    // 5. Compound ranges: flank files [a1-a8, h1-h8]
    let dsl_flanks = r#"piece R on [a1-a8, h1-h8]"#;
    let q_flanks = QueryParser::parse_str(dsl_flanks).expect("Failed to parse flank files");
    if let SearchQuery::Position(PositionPattern::MultiSquare { squares, .. }) = q_flanks {
        assert_eq!(squares.len(), 16, "Two full 8-square files = 16 squares");
    } else {
        panic!("Expected MultiSquare pattern");
    }

    // 6. Evaluation against Opera Game:
    // White King in Opera game is on e1 then castled to c1 (both are in [a1-h2] / [a-h1-2])
    let dsl_eval = r#"piece K on [a1-h2]"#;
    let q_eval = QueryParser::parse_str(dsl_eval).unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_eval, OPERA_GAME).is_match);

    // Morphy's Bishop on c4 / g5 (both are on diagonals)
    let dsl_bishop_diag = r#"piece B on [a2..g8]"#; // contains c4
    let q_bishop_diag = QueryParser::parse_str(dsl_bishop_diag).unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_bishop_diag, OPERA_GAME).is_match);
}

#[test]
fn test_mating_themes_catalog_parsing_and_evaluation() {
    let dsl_content = include_str!("../../themes/mates.dsl");
    let mut parsed_count = 0;

    for line in dsl_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((name, query_expr)) = trimmed.split_once('=') {
            let name = name.trim();
            let query_expr = query_expr.trim();

            let parsed = QueryParser::parse_str(query_expr);
            assert!(
                parsed.is_ok(),
                "Failed to parse mating theme '{}': {:?} -> Error: {:?}",
                name,
                query_expr,
                parsed.err()
            );
            parsed_count += 1;
        }
    }

    assert_eq!(
        parsed_count, 30,
        "Expected all 30 mating theme queries to parse successfully"
    );

    // Test Opera Game matches opera_mate and back_rank_mate
    let q_opera_mate = QueryParser::parse_str(
        "checkmate and attacks(R, k) and attacks(B, R) and attacks(k, R) and piece k on [d8, e8]",
    )
    .unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_opera_mate, OPERA_GAME).is_match);

    let q_back_rank =
        QueryParser::parse_str("checkmate and piece k on [a8-h8] and piece [R, Q] on [a8-h8]")
            .unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_back_rank, OPERA_GAME).is_match);

    // Test Smothered Mate query requires simultaneous checkmate + Knight attack + Zero empty flight squares
    let q_smothered_white =
        QueryParser::parse_str("checkmate and attacks(N, k) and not attacks(k, empty)").unwrap();

    let q_smothered_black =
        QueryParser::parse_str("checkmate and attacks(n, K) and not attacks(K, empty)").unwrap();

    // Opera Game and Kasparov-Topalov must NOT match smothered mate
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_smothered_white, OPERA_GAME).is_match);
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_smothered_white, KASPAROV_TOPALOV).is_match);

    // Caro-Kann Smothered Mate: 1. e4 c6 2. d4 d5 3. Nc3 dxe4 4. Nxe4 Nd7 5. Qe2 Ngf6 6. Nd6# 1-0
    let caro_kann_smothered = r#"[Event "Caro-Kann Smothered Mate"]
[White "Player1"]
[Black "Player2"]
[Result "1-0"]

1. e4 c6 2. d4 d5 3. Nc3 dxe4 4. Nxe4 Nd7 5. Qe2 Ngf6 6. Nd6# 1-0"#;

    let res_white = GameSearchEvaluator::evaluate_pgn(&q_smothered_white, caro_kann_smothered);
    assert!(
        res_white.is_match,
        "Caro-Kann must match White smothered mate"
    );
    assert_eq!(
        res_white.matching_plies,
        vec![11],
        "Matching ply must be ply 11"
    );

    // Blackburne Shilling Gambit: Black mates with 7... Nf3# against surrounded e1 King
    let blackburne_pgn = r#"[Event "Casual Game"]
[White "Amateur"]
[Black "Blackburne"]
[Result "0-1"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Nd4 4. Nxe5 Qg5 5. Nxf7 Qxg2 6. Rf1 Qxe4+ 7. Be2 Nf3# 0-1"#;

    let res_black = GameSearchEvaluator::evaluate_pgn(&q_smothered_black, blackburne_pgn);
    assert!(
        res_black.is_match,
        "Blackburne Shilling must match Black smothered mate"
    );
    assert_eq!(
        res_black.matching_plies,
        vec![14],
        "Matching ply must be ply 14"
    );

    // Unified flipcolor query matches BOTH games with ONE query!
    let q_flipcolor = QueryParser::parse_str(
        "flipcolor { checkmate and attacks(N, k) and not attacks(k, empty) }",
    )
    .unwrap();

    let res_fc_white = GameSearchEvaluator::evaluate_pgn(&q_flipcolor, caro_kann_smothered);
    assert!(
        res_fc_white.is_match,
        "flipcolor must match White smothered mate"
    );
    assert_eq!(res_fc_white.matching_plies, vec![11]);

    let res_fc_black = GameSearchEvaluator::evaluate_pgn(&q_flipcolor, blackburne_pgn);
    assert!(
        res_fc_black.is_match,
        "flipcolor must match Black smothered mate"
    );
    assert_eq!(res_fc_black.matching_plies, vec![14]);
}

#[test]
fn test_light_and_dark_square_and_bishop_filtering() {
    // 1. Parsing light and dark square sets in square ranges
    let q_light_bishop = QueryParser::parse_str("piece B on light").unwrap();
    let q_dark_bishop = QueryParser::parse_str("piece B on dark").unwrap();
    let q_bracket_light = QueryParser::parse_str("piece [B, b] on [light]").unwrap();
    let q_bracket_dark = QueryParser::parse_str("piece [B, b] on [dark]").unwrap();

    // In Opera game, Morphy's light-squared bishop moves to c4 (c4 is light square!)
    assert!(GameSearchEvaluator::evaluate_pgn(&q_light_bishop, OPERA_GAME).is_match);
    assert!(GameSearchEvaluator::evaluate_pgn(&q_bracket_light, OPERA_GAME).is_match);

    // Morphy's dark-squared bishop moves to g5 (g5 is dark square!)
    assert!(GameSearchEvaluator::evaluate_pgn(&q_dark_bishop, OPERA_GAME).is_match);
    assert!(GameSearchEvaluator::evaluate_pgn(&q_bracket_dark, OPERA_GAME).is_match);

    // 2. Material bishop square counts:
    // In Opera Game: White has a light bishop (c4) and a dark bishop (c1/g5); Black has a dark bishop (c8).
    let q_mat_wb_light = QueryParser::parse_str("white_light_bishops >= 1").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_mat_wb_light, OPERA_GAME).is_match);

    let q_mat_wb_dark = QueryParser::parse_str("white_dark_bishops >= 1").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_mat_wb_dark, OPERA_GAME).is_match);

    let q_total_dark = QueryParser::parse_str("dark_bishops == 2").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_total_dark, OPERA_GAME).is_match);

    // 3. Prefix modifiers: dark queen, light queen, dark white_pieces, light white_pieces
    // In Opera Game: White Queen starts on d1 (light square) and moves to b3 (light square), Black Queen starts on d8 (dark square) and moves to e7 / e6 (dark/light)
    let q_dark_queen = QueryParser::parse_str("dark queen").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_dark_queen, OPERA_GAME).is_match);

    let q_light_queen = QueryParser::parse_str("light queen").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_light_queen, OPERA_GAME).is_match);

    let q_dark_white_pieces = QueryParser::parse_str("dark white_pieces count >= 4").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_dark_white_pieces, OPERA_GAME).is_match);

    let q_light_white_pieces = QueryParser::parse_str("light white_pieces >= 6").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_light_white_pieces, OPERA_GAME).is_match);

    let q_piece_dark_queen = QueryParser::parse_str("piece dark queen").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_piece_dark_queen, OPERA_GAME).is_match);

    let q_dark_queen_d_file = QueryParser::parse_str("dark queen on [d1..d8]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_dark_queen_d_file, OPERA_GAME).is_match);

    let q_light_brackets = QueryParser::parse_str("light [B, b] >= 1").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_light_brackets, OPERA_GAME).is_match);

    // Further piece combinations:
    let q_dark_knight = QueryParser::parse_str("dark knight >= 1").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_dark_knight, OPERA_GAME).is_match);

    let q_dark_black_pieces = QueryParser::parse_str("dark black_pieces >= 4").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_dark_black_pieces, OPERA_GAME).is_match);

    let q_dark_empty = QueryParser::parse_str("dark empty >= 8").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_dark_empty, OPERA_GAME).is_match);

    let q_light_rook = QueryParser::parse_str("light rook").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_light_rook, OPERA_GAME).is_match);
}

#[test]
fn test_bare_piece_syntax_and_cql_piece_counts() {
    // 1. Bare uppercase piece symbol: R == 1 (White Rook)
    let q_white_rook = QueryParser::parse_str("R == 1").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_white_rook, OPERA_GAME).is_match);

    // 2. Bare lowercase piece symbol: r == 1 (Black Rook)
    let q_black_rook = QueryParser::parse_str("r == 1").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_black_rook, OPERA_GAME).is_match);

    // 3. User's exact query format: white_pieces==2 and R==1 black_pieces==1 on an endgame FEN
    let endgame_pgn = r#"[Event "Endgame"]
[FEN "4k3/8/8/8/8/8/8/4K2R w - - 0 1"]
[Result "*"]

1. Rh8+ Kd7 2. Ke2 *"#;
    let q_user = QueryParser::parse_str("white_pieces==2 and R==1 black_pieces==1").unwrap();
    let res_user = GameSearchEvaluator::evaluate_pgn(&q_user, endgame_pgn);
    assert!(
        res_user.is_match,
        "Must match endgame FEN with 2 white pieces (K+R) and 1 black piece (k)"
    );

    // 4. Opera Game at checkmate: R == 1 and Q == 0 and r == 1 and k on e8
    let q_opera_end = QueryParser::parse_str("R == 1 and Q == 0 and r == 1 and k on e8").unwrap();
    let res_opera_end = GameSearchEvaluator::evaluate_pgn(&q_opera_end, OPERA_GAME);
    assert!(res_opera_end.is_match);
    assert_eq!(res_opera_end.matching_plies, vec![32, 33]);

    // 5. Bare bracketed piece lists and square placements
    let q_brackets = QueryParser::parse_str("[R, Q] >= 1").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_brackets, OPERA_GAME).is_match);

    let q_bare_placement = QueryParser::parse_str("R on d8").unwrap();
    let res_rd8 = GameSearchEvaluator::evaluate_pgn(&q_bare_placement, OPERA_GAME);
    assert!(res_rd8.is_match);
    assert_eq!(res_rd8.matching_plies, vec![33]);

    let q_black_king = QueryParser::parse_str("k on e8").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_black_king, OPERA_GAME).is_match);
}

#[test]
fn test_wtm_btm_any_color_and_legal_move_filters() {
    // 1. wtm (white to move) and btm (black to move)
    let q_wtm = QueryParser::parse_str("wtm and R on d8").unwrap();
    let res_wtm = GameSearchEvaluator::evaluate_pgn(&q_wtm, OPERA_GAME);
    // At ply 33, White played 17. Rd8# so it's Black's turn (btm) after the move!
    assert!(
        !res_wtm.is_match,
        "At ply 33 it is Black to move (btm), not wtm"
    );

    let q_btm = QueryParser::parse_str("btm and R on d8").unwrap();
    let res_btm = GameSearchEvaluator::evaluate_pgn(&q_btm, OPERA_GAME);
    assert!(res_btm.is_match);
    assert_eq!(res_btm.matching_plies, vec![33]);

    // 2. A (any white piece) and a (any black piece)
    let q_a_counts = QueryParser::parse_str("A >= 8 and a >= 8").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_a_counts, OPERA_GAME).is_match);

    let q_a_on_sq = QueryParser::parse_str("A on d8").unwrap();
    let res_a_d8 = GameSearchEvaluator::evaluate_pgn(&q_a_on_sq, OPERA_GAME);
    assert!(res_a_d8.is_match);
    assert_eq!(res_a_d8.matching_plies, vec![33]);

    // 3. FEN wildcard with ppA (2 black pawns followed by any white piece on rank 5)
    // In custom FEN: 8/8/8/2ppN3/8/8/8/4K2k w - - 0 1 (White Knight on e5, Black pawns on c5, d5)
    let pgn_ppa = r#"[Event "Wildcard FEN"]
[FEN "8/8/8/2ppN3/8/8/8/4K2k w - - 0 1"]
[Result "*"]

1. Ke2 Kg2 *"#;
    let q_fen_ppa = QueryParser::parse_str(r#"fen "*/*/*/*ppA*/*/*/*/*""#).unwrap();
    let res_fen_ppa = GameSearchEvaluator::evaluate_pgn(&q_fen_ppa, pgn_ppa);
    assert!(
        res_fen_ppa.is_match,
        "Must match FEN with 2 black pawns and 1 white piece on rank"
    );

    // 4. Legal moves count: legal == 0 and check and legal == 0 (checkmate at ply 33)
    let q_legal_zero = QueryParser::parse_str("legal == 0").unwrap();
    let res_legal_zero = GameSearchEvaluator::evaluate_pgn(&q_legal_zero, OPERA_GAME);
    assert!(
        res_legal_zero.is_match,
        "Opera Game ends in checkmate (0 legal moves)"
    );
    assert_eq!(res_legal_zero.matching_plies, vec![33]);

    let q_check_legal_zero = QueryParser::parse_str("check and legal == 0").unwrap();
    let res_mate = GameSearchEvaluator::evaluate_pgn(&q_check_legal_zero, OPERA_GAME);
    assert!(res_mate.is_match);
    assert_eq!(res_mate.matching_plies, vec![33]);

    // 5. move filter: move from [e1, e8] to [c1, g1, c8, g8] (castling move 12. O-O-O at ply 23)
    let q_castling = QueryParser::parse_str("move from [e1, e8] to [c1, g1, c8, g8]").unwrap();
    let res_castling = GameSearchEvaluator::evaluate_pgn(&q_castling, OPERA_GAME);
    assert!(
        res_castling.is_match,
        "Must match Morphy's 12. O-O-O queenside castling"
    );
    assert_eq!(res_castling.matching_plies, vec![23]);

    // 6. move from e2 to e4 (opening move 1. e4 at ply 1)
    let q_e4 = QueryParser::parse_str("move from e2 to e4").unwrap();
    let res_e4 = GameSearchEvaluator::evaluate_pgn(&q_e4, OPERA_GAME);
    assert!(res_e4.is_match);
    assert_eq!(res_e4.matching_plies, vec![1]);
}

#[test]
fn test_move_piece_targets_ply_movenumber_and_multiline_and() {
    // 1. move from B to r (White Bishop captures/moves to Black Rook square, e.g. 15. Bxd7+ at ply 29)
    let q_b_to_r = QueryParser::parse_str("move from B to r").unwrap();
    let res_b_to_r = GameSearchEvaluator::evaluate_pgn(&q_b_to_r, OPERA_GAME);
    assert!(
        res_b_to_r.is_match,
        "Morphy played 15. Bxd7+ capturing Rook with Bishop"
    );
    assert_eq!(res_b_to_r.matching_plies, vec![29]);

    // 2. move from A to a (Any White piece captures any Black piece)
    let q_a_to_a = QueryParser::parse_str("move from A to a").unwrap();
    let res_a_to_a = GameSearchEvaluator::evaluate_pgn(&q_a_to_a, OPERA_GAME);
    assert!(res_a_to_a.is_match);
    // In Opera Game: 10. Nxb5 (ply 19), 11. Bxb5+ (ply 21), 15. Bxd7+ (ply 29), 16. Qb8+ (ply 31), 17. Rd8# (ply 33)
    assert!(res_a_to_a.matching_plies.contains(&19));
    assert!(res_a_to_a.matching_plies.contains(&29));

    // 3. move from [B, N] to [p, r]
    let q_bn_to_pr = QueryParser::parse_str("move from [B, N] to [p, r]").unwrap();
    let res_bn = GameSearchEvaluator::evaluate_pgn(&q_bn_to_pr, OPERA_GAME);
    assert!(res_bn.is_match);
    assert!(res_bn.matching_plies.contains(&19)); // 10. Nxb5 (N takes p)
    assert!(res_bn.matching_plies.contains(&21)); // 11. Bxb5+ (B takes p)
    assert!(res_bn.matching_plies.contains(&29)); // 15. Bxd7+ (B takes r)

    // 4. Multiline query separated by newlines without 'and' keywords
    let multiline_dsl = r#"
        btm
        R on d8
        check
        legal == 0
    "#;
    let q_multiline = QueryParser::parse_str(multiline_dsl).unwrap();
    let res_multiline = GameSearchEvaluator::evaluate_pgn(&q_multiline, OPERA_GAME);
    assert!(
        res_multiline.is_match,
        "Multiline query with implicit AND must match Opera checkmate"
    );
    assert_eq!(res_multiline.matching_plies, vec![33]);

    // 5. ply == 33 and ply <= 5 and ply 1..10
    let q_ply_eq = QueryParser::parse_str("ply == 33 and check").unwrap();
    let res_ply_eq = GameSearchEvaluator::evaluate_pgn(&q_ply_eq, OPERA_GAME);
    assert!(res_ply_eq.is_match);
    assert_eq!(res_ply_eq.matching_plies, vec![33]);

    let q_ply_lte = QueryParser::parse_str("ply <= 2").unwrap();
    let res_ply_lte = GameSearchEvaluator::evaluate_pgn(&q_ply_lte, OPERA_GAME);
    assert!(res_ply_lte.is_match);
    assert_eq!(res_ply_lte.matching_plies, vec![0, 1, 2]);

    let q_ply_range_standalone = QueryParser::parse_str("ply 1..3").unwrap();
    let res_ply_range = GameSearchEvaluator::evaluate_pgn(&q_ply_range_standalone, OPERA_GAME);
    assert!(res_ply_range.is_match);
    assert_eq!(res_ply_range.matching_plies, vec![1, 2, 3]);

    // 6. movenumber == 1 (plies 1, 2) and movenumber == 17 (ply 33)
    let q_mov_1 = QueryParser::parse_str("movenumber == 1").unwrap();
    let res_mov_1 = GameSearchEvaluator::evaluate_pgn(&q_mov_1, OPERA_GAME);
    assert!(res_mov_1.is_match);
    assert_eq!(res_mov_1.matching_plies, vec![0, 1, 2]);

    let q_mov_17 = QueryParser::parse_str("movenumber == 17 and check").unwrap();
    let res_mov_17 = GameSearchEvaluator::evaluate_pgn(&q_mov_17, OPERA_GAME);
    assert!(res_mov_17.is_match);
    assert_eq!(res_mov_17.matching_plies, vec![33]);
}

#[test]
fn test_pin_named_parameters() {
    // In Opera game:
    // At ply 17 (after 9. Bg5): White Bishop pins Black Knight on f6 against Black Queen on d8
    let q_pin_named = QueryParser::parse_str("pin from B to q through n").unwrap();
    let res_pin = GameSearchEvaluator::evaluate_pgn(&q_pin_named, OPERA_GAME);
    assert!(
        res_pin.is_match,
        "Morphy's 9. Bg5 creates a pin on Nf6 against Qd8"
    );
    assert!(res_pin.matching_plies.contains(&17));

    // Natural English words: pin from bishop to queen through knight
    let q_pin_words = QueryParser::parse_str("pin from bishop to queen through knight").unwrap();
    let res_words = GameSearchEvaluator::evaluate_pgn(&q_pin_words, OPERA_GAME);
    assert!(res_words.is_match);
    assert!(res_words.matching_plies.contains(&17));

    // Arbitrary parameter ordering: pin through n to q from B
    let q_pin_reordered = QueryParser::parse_str("pin through n to q from B").unwrap();
    let res_reordered = GameSearchEvaluator::evaluate_pgn(&q_pin_reordered, OPERA_GAME);
    assert!(res_reordered.is_match);
    assert!(res_reordered.matching_plies.contains(&17));
}

#[test]
fn test_descriptive_parse_error_diagnostics() {
    // 1. Unknown token / unexpected identifier with line & column
    let bad_query = "btm\npiece B on invld_sq\ncheck";
    let err = QueryParser::parse_str(bad_query).unwrap_err();
    assert_eq!(err.line, 2);
    assert!(err.column > 1);
    assert!(err.snippet.is_some());
    assert!(err.help.is_some());
    let formatted = format!("{}", err);
    assert!(formatted.contains("Syntax error at line 2"));
    assert!(formatted.contains("piece B on invld_sq"));
    assert!(formatted.contains('^'));
    assert!(formatted.contains("Hint:"));

    // 2. Missing closing parenthesis
    let bad_paren = "attacks(wk, bp";
    let err_paren = QueryParser::parse_str(bad_paren).unwrap_err();
    assert_eq!(err_paren.line, 1);
    let formatted_paren = format!("{}", err_paren);
    assert!(formatted_paren.contains("Syntax error at line 1"));

    // 3. Invalid turn specifier
    let bad_turn = "turn invalid_color";
    let err_turn = QueryParser::parse_str(bad_turn).unwrap_err();
    assert_eq!(err_turn.line, 1);
    let formatted_turn = format!("{}", err_turn);
    assert!(formatted_turn.contains("Invalid turn color"));
}

#[test]
fn test_variable_binding_and_fork_queries() {
    // Game with a royal knight fork or tactical motif
    // In Opera game, at ply 21 (11. Bxb5+): White Bishop attacks Black King
    let q_var = QueryParser::parse_str(
        r#"
        $attacker = B
        attacks($attacker, k)
    "#,
    )
    .unwrap();
    let res_var = GameSearchEvaluator::evaluate_pgn(&q_var, OPERA_GAME);
    assert!(res_var.is_match);
    assert!(res_var.matching_plies.contains(&21)); // 11. Bxb5+

    // Test block syntax: piece $b in B { attacks $b k }
    let q_block = QueryParser::parse_str(
        r#"
        piece $b in B {
            attacks $b k
        }
    "#,
    )
    .unwrap();
    let res_block = GameSearchEvaluator::evaluate_pgn(&q_block, OPERA_GAME);
    assert!(res_block.is_match);
    assert!(res_block.matching_plies.contains(&21));

    // Test direct attacks without "on" (e.g. attacks B k, attacks N q)
    let q_direct = QueryParser::parse_str("attacks B k").unwrap();
    let res_direct = GameSearchEvaluator::evaluate_pgn(&q_direct, OPERA_GAME);
    assert!(res_direct.is_match);
    assert!(res_direct.matching_plies.contains(&21));

    // Test multi-condition bound variable on specific FEN position
    // Setup position where Knight on c7 attacks King on e8 and Rook on a8
    let fork_fen = "r3k3/2N5/8/8/8/8/8/4K3 b - - 0 1";
    let pos: shakmaty::Chess = shakmaty::fen::Fen::from_ascii(fork_fen.as_bytes())
        .unwrap()
        .into_position(shakmaty::CastlingMode::Standard)
        .unwrap();

    let q_bound_fork = QueryParser::parse_str(
        r#"
        $ForkingKnight = N
        attacks($ForkingKnight, k) and attacks($ForkingKnight, r)
    "#,
    )
    .unwrap();

    let res_single_pos =
        GameSearchEvaluator::evaluate_with_timeline(&q_bound_fork, &HashMap::new(), &[pos], &[]);
    assert!(
        res_single_pos.is_match,
        "$ForkingKnight must verify that the same knight attacks both k and r"
    );
}

#[test]
fn test_fork_exact_slots_and_piece_options() {
    // 1. Position where White Pawn attacks 2 Black Rooks (b4 pawn attacks a5 rook and c5 rook)
    // FEN: 8/8/8/r1r5/1P6/8/8/4K2k w - - 0 1
    let pawn_2_rooks_fen = "8/8/8/r1r5/1P6/8/8/4K2k w - - 0 1";
    let pos_2_rooks: shakmaty::Chess = shakmaty::fen::Fen::from_ascii(pawn_2_rooks_fen.as_bytes())
        .unwrap()
        .into_position(shakmaty::CastlingMode::Standard)
        .unwrap();

    // 2. Position where White Pawn attacks 1 Black Queen and 1 Black Rook
    // FEN: 8/8/8/q1r5/1P6/8/8/4K2k w - - 0 1
    let pawn_qr_fen = "8/8/8/q1r5/1P6/8/8/4K2k w - - 0 1";
    let pos_qr: shakmaty::Chess = shakmaty::fen::Fen::from_ascii(pawn_qr_fen.as_bytes())
        .unwrap()
        .into_position(shakmaty::CastlingMode::Standard)
        .unwrap();

    // Query: fork(P, q, r) -> requires 1 Queen and 1 Rook!
    let q_p_q_r = QueryParser::parse_str("fork(P, q, r)").unwrap();

    let res_2_rooks = GameSearchEvaluator::evaluate_with_timeline(
        &q_p_q_r,
        &HashMap::new(),
        std::slice::from_ref(&pos_2_rooks),
        &[],
    );
    assert!(
        !res_2_rooks.is_match,
        "fork(P, q, r) must NOT match a pawn attacking 2 rooks"
    );

    let res_qr = GameSearchEvaluator::evaluate_with_timeline(
        &q_p_q_r,
        &HashMap::new(),
        std::slice::from_ref(&pos_qr),
        &[],
    );
    assert!(
        res_qr.is_match,
        "fork(P, q, r) MUST match a pawn attacking a queen and a rook"
    );

    // Query with array of options per slot: fork([P, N], [q, k], [r, b])
    let q_slots = QueryParser::parse_str("fork([P, N], [q, k], [r, b])").unwrap();
    assert!(
        GameSearchEvaluator::evaluate_with_timeline(
            &q_slots,
            &HashMap::new(),
            std::slice::from_ref(&pos_qr),
            &[]
        )
        .is_match
    );
    assert!(
        !GameSearchEvaluator::evaluate_with_timeline(
            &q_slots,
            &HashMap::new(),
            std::slice::from_ref(&pos_2_rooks),
            &[]
        )
        .is_match
    );

    // Named arguments: fork(attacker in [P, N], target in [q, k], target in [r, b])
    let q_named =
        QueryParser::parse_str("fork(attacker in [P, N], target in [q, k], target in [r, b])")
            .unwrap();
    assert!(
        GameSearchEvaluator::evaluate_with_timeline(&q_named, &HashMap::new(), &[pos_qr], &[])
            .is_match
    );

    // Pool query: fork(attacker in [P], targets in [q, r]) matches 2 rooks because both are from pool [q, r]
    let q_pool = QueryParser::parse_str("fork(attacker in [P], targets in [q, r])").unwrap();
    assert!(
        GameSearchEvaluator::evaluate_with_timeline(&q_pool, &HashMap::new(), &[pos_2_rooks], &[])
            .is_match
    );
}

#[test]
fn test_query_comments_support() {
    let q_str = r#"
        // This is a line comment with double slashes
        # This is a hash comment
        /* This is a block
           comment spanning lines */
        attacks B k // inline comment
    "#;
    let query = QueryParser::parse_str(q_str).unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&query, OPERA_GAME);
    assert!(res.is_match);
    assert!(res.matching_plies.contains(&21));
}

#[test]
fn test_compact_piece_placements() {
    // 1. Kd4 parses and matches position where White King is on d4
    let q_kd4 = QueryParser::parse_str("Kd4").unwrap();
    let fen_kd4 = "8/8/8/8/3K4/8/8/4k3 w - - 0 1";
    let pos_kd4: shakmaty::Chess = shakmaty::fen::Fen::from_ascii(fen_kd4.as_bytes())
        .unwrap()
        .into_position(shakmaty::CastlingMode::Standard)
        .unwrap();
    let res_kd4 =
        GameSearchEvaluator::evaluate_with_timeline(&q_kd4, &HashMap::new(), &[pos_kd4], &[]);
    assert!(res_kd4.is_match);

    // 2. qd4 and _e4
    let q_multi = QueryParser::parse_str("qd4 and _e4 and Ke1").unwrap();
    let fen_multi = "4k3/8/8/8/3q4/8/8/4K3 w - - 0 1";
    let pos_multi: shakmaty::Chess = shakmaty::fen::Fen::from_ascii(fen_multi.as_bytes())
        .unwrap()
        .into_position(shakmaty::CastlingMode::Standard)
        .unwrap();
    let res_multi =
        GameSearchEvaluator::evaluate_with_timeline(&q_multi, &HashMap::new(), &[pos_multi], &[]);
    assert!(res_multi.is_match);
}

#[test]
fn test_path_piece_identifiers_search() {
    // Greek gift game snippet: 17. Bxh7+ Kxh7 18. Ng5+
    let greek_gift_pgn = r#"[Event "Greek Gift Test"]
[White "Player1"]
[Black "Player2"]
[Result "1-0"]

1. e4 b6 2. d4 Bb7 3. Nc3 e6 4. Bd3 c5 5. Be3 a6 6. a4 Nf6 7. Nf3 d5 8. e5
Nfd7 9. O-O c4 10. Be2 Be7 11. b3 cxb3 12. cxb3 O-O 13. Bd3 Nc6 14. Na2 a5
15. Rc1 Nb4 16. Nxb4 Bxb4 17. Bxh7+ Kxh7 18. Ng5+ Kg8 19. Qh5 f6 20. Qh7# 1-0
"#;

    // 1. Standard Bxh7 kxh7 (White Bishop captures on h7, Black King captures on h7)
    let q1 = QueryParser::parse_str("path [Bxh7 kxh7]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q1, greek_gift_pgn).is_match);

    // 2. Piece identifier: Bishop takes black pawn on h7, followed by black king takes on h7
    let q2 = QueryParser::parse_str("path [Bxph7 kxh7]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q2, greek_gift_pgn).is_match);

    // 3. Piece identifier: Bishop takes pawn on h7 (Bxpd5 should NOT match, but Bxph7 should)
    let q3_fail = QueryParser::parse_str("path [Bxpd5]").unwrap();
    assert!(!GameSearchEvaluator::evaluate_pgn(&q3_fail, greek_gift_pgn).is_match);

    // 4. Bracketed target piece list: Bishop takes queen or rook or pawn on h7
    let q4 = QueryParser::parse_str("path [Bx[q,r,p]h7 kxh7]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q4, greek_gift_pgn).is_match);

    // 5. King takes anywhere on board (kxd5 vs kxh7)
    let q5 = QueryParser::parse_str("path [kxh7]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q5, greek_gift_pgn).is_match);

    // 6. Non-consecutive path: Bishop captures on h7 ... and later White Queen to h7 checkmate
    let q6 = QueryParser::parse_str("path [Bxph7 ... Qh7]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q6, greek_gift_pgn).is_match);
}

#[test]
fn test_wildcard_moves_and_promotions_and_en_passant() {
    // Game with promotion and en passant:
    // 1. e4 Nf6 2. e5 d5 3. exd6 (en passant!) c5 4. d7+ Kxd7 5. a4 e5 6. a5 e4 7. a6 e3 8. axb7 exf2+ 9. Kxf2 Ne4+ 10. Ke1 Qh4+ 11. g3 Nxg3 12. bxa8=R (underpromotion to rook!)
    let game_pgn = r#"[Event "Wildcard & Promo Test"]
[White "WhitePlayer"]
[Black "BlackPlayer"]
[Result "*"]

1. e4 Nf6 2. e5 d5 3. exd6 c5 4. d7+ Kxd7 5. a4 e5 6. a5 e4 7. a6 e3 8. axb7 exf2+ 9. Kxf2 Ne4+ 10. Ke1 Qh4+ 11. g3 Nxg3 12. bxa8=R *
"#;

    // 1. Wildcard moves: 1. e4 -- 2. e5 --
    let q1 = QueryParser::parse_str("line [e4 -- e5 --]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q1, game_pgn).is_match);

    // 2. Any White move followed by Black move: A-- a--
    let q2 = QueryParser::parse_str("line [A-- a--]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q2, game_pgn).is_match);

    // 3. En passant capture matching: White Pawn takes black pawn via ep on d6
    let q3 = QueryParser::parse_str("path [Pxpd6]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q3, game_pgn).is_match);

    // 4. White Pawn capture anywhere: Px-- or Px*
    let q4 = QueryParser::parse_str("path [Px--]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q4, game_pgn).is_match);

    // 5. Underpromotion wildcard: --=R (promotes to Rook)
    let q5 = QueryParser::parse_str("path [--=R]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q5, game_pgn).is_match);

    // 6. Multi-piece Underpromotion list: --="RBN"
    let q6 = QueryParser::parse_str(r#"path [--="RBN"]"#).unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q6, game_pgn).is_match);

    // 7. Should NOT match promotion to Queen
    let q7_fail = QueryParser::parse_str("path [--=Q]").unwrap();
    assert!(!GameSearchEvaluator::evaluate_pgn(&q7_fail, game_pgn).is_match);

    // 8. Capture & Underpromotion: Pxb8=R or Px=R
    let q8 = QueryParser::parse_str("path [Px=R]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q8, game_pgn).is_match);

    // 9. Wildcard with '_'
    let q9 = QueryParser::parse_str("line [e4 _ e5 _]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q9, game_pgn).is_match);

    // 10. move filter with promote parameter
    let q10_promote_r = QueryParser::parse_str("move promote R").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q10_promote_r, game_pgn).is_match);

    let q10_promote_compact = QueryParser::parse_str("move promote [BNR]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q10_promote_compact, game_pgn).is_match);

    let q10_promote_list = QueryParser::parse_str("move promote [B, N, R]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q10_promote_list, game_pgn).is_match);

    let q10_promote_any = QueryParser::parse_str("move promote").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q10_promote_any, game_pgn).is_match);

    let q10_promote_q_fail = QueryParser::parse_str("move promote Q").unwrap();
    assert!(!GameSearchEvaluator::evaluate_pgn(&q10_promote_q_fail, game_pgn).is_match);

    let q10_legal_promote = QueryParser::parse_str("legal promote [R, Q] count >= 1").unwrap();
    // At ply 21 (before 12. bxa8=R), White has legal promotions on b7
    let res_legal_promo = GameSearchEvaluator::evaluate_pgn(&q10_legal_promote, game_pgn);
    assert!(res_legal_promo.is_match);
}

#[test]
fn test_bracketed_piece_group_counts() {
    // 1. [Qq] == 0 on endgame position without queens
    let q_no_queens = QueryParser::parse_str("[Qq] == 0").unwrap();
    let no_queen_fen = "8/8/4k3/8/8/4K3/4P3/8 w - - 0 1";
    let pos_no_q: shakmaty::Chess = shakmaty::fen::Fen::from_ascii(no_queen_fen.as_bytes())
        .unwrap()
        .into_position(shakmaty::CastlingMode::Standard)
        .unwrap();
    let res1 = GameSearchEvaluator::evaluate_with_timeline(
        &q_no_queens,
        &HashMap::new(),
        std::slice::from_ref(&pos_no_q),
        &[],
    );
    assert!(res1.is_match);

    // 2. [RBN] == 0 on pure pawn + king endgame
    let q_no_minor_or_rooks = QueryParser::parse_str("[RBN] == 0").unwrap();
    let res2 = GameSearchEvaluator::evaluate_with_timeline(
        &q_no_minor_or_rooks,
        &HashMap::new(),
        &[pos_no_q],
        &[],
    );
    assert!(res2.is_match);

    // 3. Position with 1 White Queen: [Qq] == 1 should match
    let q_one_queen = QueryParser::parse_str("[Qq] == 1").unwrap();
    let queen_fen = "8/8/4k3/8/3Q4/4K3/8/8 b - - 0 1";
    let pos_q: shakmaty::Chess = shakmaty::fen::Fen::from_ascii(queen_fen.as_bytes())
        .unwrap()
        .into_position(shakmaty::CastlingMode::Standard)
        .unwrap();
    let res3 = GameSearchEvaluator::evaluate_with_timeline(
        &q_one_queen,
        &HashMap::new(),
        std::slice::from_ref(&pos_q),
        &[],
    );
    assert!(res3.is_match);

    // 4. Position with Queen fails [Qq] == 0
    let res4 =
        GameSearchEvaluator::evaluate_with_timeline(&q_no_queens, &HashMap::new(), &[pos_q], &[]);
    assert!(!res4.is_match);
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
fn test_path_regex_quantifiers_and_consecutive_default() {
    // Game where 1. e4 e5 2. Nf3 Nc6 3. Bc4 Nf6 4. d3 Bb4+ 5. c3 Ba5 6. a4 Bb6 7. b4 d6 8. a5 Bxf2+
    let trap_pgn = r#"[Event "Trap Test"]
[White "Player1"]
[Black "Player2"]
[Result "*"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Nf6 4. d3 Bb4+ 5. c3 Ba5 6. a4 Bb6 7. b4 d6 8. a5 Bxf2+ 9. Kxf2 *
"#;

    // 1. Strict consecutive: path [ a5 bxf2+ ] -> must match plies [15, 16]
    let q_consecutive = QueryParser::parse_str("path [ a5 bxf2+ ]").unwrap();
    let res_c = GameSearchEvaluator::evaluate_pgn(&q_consecutive, trap_pgn);
    assert!(res_c.is_match);
    assert_eq!(res_c.matching_plies, vec![15, 16]);

    // 2. Strict consecutive failure: path [ e4 d6 ] -> fails because e4 is immediately followed by e5, not d6
    let q_fail_consecutive = QueryParser::parse_str("path [ e4 d6 ]").unwrap();
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_fail_consecutive, trap_pgn).is_match);

    // 3. Gap wildcard with ... (0 or more moves)
    let q_dots = QueryParser::parse_str("path [ e4 ... d6 ... bxf2+ ]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_dots, trap_pgn).is_match);

    // 4. Regex * wildcard: path [ e4 --* d6 --* bxf2+ ]
    let q_star = QueryParser::parse_str("path [ e4 --* d6 --* bxf2+ ]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_star, trap_pgn).is_match);

    // 5. Regex + wildcard: path [ a5 --+ bxf2+ ] -> fails because a5 is followed by 0 intermediate moves (bxf2+ is next)
    let q_plus_fail = QueryParser::parse_str("path [ a5 --+ bxf2+ ]").unwrap();
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_plus_fail, trap_pgn).is_match);

    // But e4 --+ d6 succeeds because there are intermediate moves between e4 and d6
    let q_plus_ok = QueryParser::parse_str("path [ e4 --+ d6 ]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_plus_ok, trap_pgn).is_match);

    // 6. Range quantifier: --{min, max}
    // Between e4 (ply 1) and e5 (ply 2): 0 intermediate moves -> --{0, 2}
    let q_range = QueryParser::parse_str("path [ e4 --{0, 2} e5 ]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_range, trap_pgn).is_match);

    // Between a5 (ply 14) and bxf2+ (ply 15): 0 intermediate moves -> --{1, 5} must FAIL
    let q_range_fail = QueryParser::parse_str("path [ a5 --{1, 5} bxf2+ ]").unwrap();
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_range_fail, trap_pgn).is_match);
}

#[test]
fn test_position_and_move_or_path_combination() {
    let trap_pgn = r#"[Event "Position + Move Test"]
[White "Player1"]
[Black "Player2"]
[Result "*"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Nf6 4. d3 Bb4+ 5. c3 Ba5 6. a4 Bb6 7. b4 d6 8. a5 Bxf2+ 9. Kxf2 *
"#;

    // 1. Position + Move SAN: Bc4 and bb6 and Pa5 and move "Bxf2+"
    let q_pos_move = QueryParser::parse_str("Bc4 bb6 Pa5 and move \"Bxf2+\"").unwrap();
    let res1 = GameSearchEvaluator::evaluate_pgn(&q_pos_move, trap_pgn);
    assert!(
        res1.is_match,
        "Position pattern and subsequent departure move must match"
    );
    assert_eq!(res1.matching_plies, vec![15]);

    // 2. Position + Move piece parameters: Bc4 bb6 Pa5 and move from b6 to f2
    let q_pos_coords = QueryParser::parse_str("Bc4 bb6 Pa5 and move from b6 to f2").unwrap();
    let res2 = GameSearchEvaluator::evaluate_pgn(&q_pos_coords, trap_pgn);
    assert!(res2.is_match);
    assert_eq!(res2.matching_plies, vec![15]);

    // 3. Position + Anchored Path: Bc4 bb6 Pa5 and path [ bxf2+ ]
    let q_pos_path = QueryParser::parse_str("Bc4 bb6 Pa5 and path [ bxf2+ ]").unwrap();
    let res3 = GameSearchEvaluator::evaluate_pgn(&q_pos_path, trap_pgn);
    assert!(
        res3.is_match,
        "Path anchored on matched position must succeed"
    );
    assert_eq!(res3.matching_plies, vec![15]);

    // 4. Position + Multi-move Anchored Path: Bc4 bb6 Pa5 and path [ bxf2+ Kxf2 ]
    let q_pos_path2 = QueryParser::parse_str("Bc4 bb6 Pa5 and path [ bxf2+ Kxf2 ]").unwrap();
    let res4 = GameSearchEvaluator::evaluate_pgn(&q_pos_path2, trap_pgn);
    assert!(res4.is_match);
    assert_eq!(res4.matching_plies, vec![15]);

    // 5. Position matched but departure move does NOT match:
    let q_pos_fail = QueryParser::parse_str("Bc4 bb6 Pa5 and move \"Nxe4\"").unwrap();
    let res5 = GameSearchEvaluator::evaluate_pgn(&q_pos_fail, trap_pgn);
    assert!(
        !res5.is_match,
        "Position matched but wrong departure move must NOT match"
    );
}

#[test]
fn test_fen_transformations_and_symmetries() {
    let starting_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    // 1. Color Invert on starting position swaps ranks 1 and 8 and colors
    let sym_color = BoardSymmetry::ColorInvert;
    let fen_color = sym_color.transform_fen(starting_fen).unwrap();
    assert_eq!(
        fen_color,
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1"
    );

    // 2. Horizontal mirror swaps files (a <-> h) and castling (K <-> Q)
    let sym_h = BoardSymmetry::HorizontalMirror;
    let custom_fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
    let fen_h = sym_h.transform_fen(custom_fen).unwrap();
    assert_eq!(fen_h, "r2k3r/8/8/8/8/8/8/R2K3R w KQkq - 0 1");

    // 3. Vertical mirror swaps ranks without inverting piece color
    let sym_v = BoardSymmetry::VerticalMirror;
    let white_pawn_fen = "8/8/8/8/8/8/4P3/8 w - - 0 1";
    let fen_v = sym_v.transform_fen(white_pawn_fen).unwrap();
    assert_eq!(fen_v, "8/4P3/8/8/8/8/8/8 w - - 0 1");
}

#[test]
fn test_query_explain_and_to_dsl() {
    // 1. Header flipcolor
    let query_str = "flipcolor { white \"Carlsen\" }";
    let explanation = QueryParser::explain(query_str).unwrap();
    assert_eq!(explanation.branches.len(), 2);
    assert_eq!(explanation.branches[0].symmetry_name, "Identity (original)");
    assert_eq!(explanation.branches[0].dsl, "white \"Carlsen\"");
    assert_eq!(
        explanation.branches[1].symmetry_name,
        "Color Inverted (flipcolor)"
    );
    assert_eq!(explanation.branches[1].dsl, "black \"Carlsen\"");

    // 2. FEN symmetry explanation
    let fen_query = "flip:all { fen \"8/8/8/8/8/8/4P3/8 w - - 0 1\" }";
    let exp_fen = QueryParser::explain(fen_query).unwrap();
    assert_eq!(exp_fen.branches.len(), 6);
    assert!(exp_fen.has_symmetries);
    assert!(!exp_fen.branches[0].transformed_fens.is_empty());
    assert_eq!(
        exp_fen.branches[0].transformed_fens[0],
        "8/8/8/8/8/8/4P3/8 w - - 0 1"
    );
    // Color inverted branch: White pawn on e2 becomes Black pawn on e7
    let color_branch = exp_fen
        .branches
        .iter()
        .find(|b| b.symmetry_name.contains("Color Inverted"))
        .unwrap();
    assert_eq!(
        color_branch.transformed_fens[0],
        "8/4p3/8/8/8/8/8/8 b - - 0 1"
    );

    // 3. Complex compound query to_dsl
    let complex_q =
        QueryParser::parse_str("player \"Morphy\" and wtm and legal count == 0").unwrap();
    let dsl = complex_q.to_dsl();
    assert!(dsl.contains("player \"Morphy\""));
    assert!(dsl.contains("wtm"));
    assert!(dsl.contains("legal count == 0"));
}

#[test]
fn test_advanced_move_separator_promotions_and_captures() {
    use shakmaty::{Role, Square};

    // 1. Verify AST parsing for Pe7xd8=Q
    let q_cap_promo = QueryParser::parse_str("path [Pe7xd8=Q]").unwrap();
    if let SearchQuery::Path(ref pat) = q_cap_promo {
        let mv = &pat.moves[0];
        assert_eq!(mv.from, Some(Square::E7));
        assert_eq!(mv.to, Some(Square::D8));
        assert_eq!(mv.role, Some(Role::Pawn));
        assert_eq!(mv.color, Some(Color::White));
        assert_eq!(mv.is_capture, Some(true));
        assert_eq!(mv.promotion, Some(Role::Queen));
    } else {
        panic!("Expected Path query");
    }

    // 2. Verify AST parsing for P[x]d8=Q
    let q_px_d8 = QueryParser::parse_str("path [P[x]d8=Q]").unwrap();
    if let SearchQuery::Path(ref pat) = q_px_d8 {
        let mv = &pat.moves[0];
        assert_eq!(mv.to, Some(Square::D8));
        assert_eq!(mv.role, Some(Role::Pawn));
        assert_eq!(mv.color, Some(Color::White));
        assert_eq!(mv.is_capture, Some(true));
        assert_eq!(mv.promotion, Some(Role::Queen));
    } else {
        panic!("Expected Path query");
    }

    // 3. Verify AST parsing for P[x]a=Q
    let q_px_a = QueryParser::parse_str("path [P[x]a=Q]").unwrap();
    if let SearchQuery::Path(ref pat) = q_px_a {
        let mv = &pat.moves[0];
        assert_eq!(mv.role, Some(Role::Pawn));
        assert_eq!(mv.color, Some(Color::White));
        assert_eq!(mv.is_capture, Some(true));
        assert_eq!(mv.promotion, Some(Role::Queen));
        assert!(mv.to_pieces.is_some());
    } else {
        panic!("Expected Path query");
    }

    // 4. Verify AST parsing for Pxr=Q
    let q_pxr = QueryParser::parse_str("path [Pxr=Q]").unwrap();
    if let SearchQuery::Path(ref pat) = q_pxr {
        let mv = &pat.moves[0];
        assert_eq!(mv.role, Some(Role::Pawn));
        assert_eq!(mv.color, Some(Color::White));
        assert_eq!(mv.is_capture, Some(true));
        assert_eq!(mv.promotion, Some(Role::Queen));
    } else {
        panic!("Expected Path query");
    }

    // 5. Verify AST parsing for A--=Q and P--=Q
    let q_a_promo = QueryParser::parse_str("path [A--=Q]").unwrap();
    if let SearchQuery::Path(ref pat) = q_a_promo {
        let mv = &pat.moves[0];
        assert_eq!(mv.color, Some(Color::White));
        assert_eq!(mv.promotion, Some(Role::Queen));
    } else {
        panic!("Expected Path query");
    }

    let q_p_promo = QueryParser::parse_str("path [P--=Q]").unwrap();
    if let SearchQuery::Path(ref pat) = q_p_promo {
        let mv = &pat.moves[0];
        assert_eq!(mv.role, Some(Role::Pawn));
        assert_eq!(mv.color, Some(Color::White));
        assert_eq!(mv.promotion, Some(Role::Queen));
    } else {
        panic!("Expected Path query");
    }

    // 6. Verify Move Separator `--` AST parsing: Ph6--h7, P--h7, Nf3--g5, h4--h5
    let q_ph6_h7 = QueryParser::parse_str("path [Ph6--h7]").unwrap();
    if let SearchQuery::Path(ref pat) = q_ph6_h7 {
        let mv = &pat.moves[0];
        assert_eq!(mv.from, Some(Square::H6));
        assert_eq!(mv.to, Some(Square::H7));
        assert_eq!(mv.role, Some(Role::Pawn));
        assert_eq!(mv.color, Some(Color::White));
    } else {
        panic!("Expected Path query");
    }

    let q_p_h7 = QueryParser::parse_str("path [P--h7]").unwrap();
    if let SearchQuery::Path(ref pat) = q_p_h7 {
        let mv = &pat.moves[0];
        assert_eq!(mv.to, Some(Square::H7));
        assert_eq!(mv.role, Some(Role::Pawn));
        assert_eq!(mv.color, Some(Color::White));
    } else {
        panic!("Expected Path query");
    }

    let q_nf3_g5 = QueryParser::parse_str("path [Nf3--g5]").unwrap();
    if let SearchQuery::Path(ref pat) = q_nf3_g5 {
        let mv = &pat.moves[0];
        assert_eq!(mv.from, Some(Square::F3));
        assert_eq!(mv.to, Some(Square::G5));
        assert_eq!(mv.role, Some(Role::Knight));
        assert_eq!(mv.color, Some(Color::White));
    } else {
        panic!("Expected Path query");
    }

    let q_h4_h5 = QueryParser::parse_str("path [h4--h5]").unwrap();
    if let SearchQuery::Path(ref pat) = q_h4_h5 {
        let mv = &pat.moves[0];
        assert_eq!(mv.from, Some(Square::H4));
        assert_eq!(mv.to, Some(Square::H5));
    } else {
        panic!("Expected Path query");
    }

    // 7. Test evaluation on game with promotion and captures
    let game_pgn = r#"[Event "Promo & Capture Test"]
[White "White"]
[Black "Black"]
[Result "1-0"]

1. e4 d5 2. e5 d4 3. e6 d3 4. exf7+ Kd7 5. fxg8=Q *
"#;

    let q_promo_eval1 = QueryParser::parse_str("path [P--=Q]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_promo_eval1, game_pgn).is_match);

    let q_promo_eval2 = QueryParser::parse_str("path [A--=Q]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_promo_eval2, game_pgn).is_match);

    let q_promo_eval3 = QueryParser::parse_str("path [Pxn=Q]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_promo_eval3, game_pgn).is_match);

    let q_promo_eval4 = QueryParser::parse_str("path [Pf7xg8=Q]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_promo_eval4, game_pgn).is_match);

    let q_promo_eval5 = QueryParser::parse_str("path [P[x]g8=Q]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_promo_eval5, game_pgn).is_match);

    let q_promo_eval6 = QueryParser::parse_str("path [P[x]a=Q]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_promo_eval6, game_pgn).is_match);
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
    let bare_ids: Vec<usize> = res_bare.iter().map(|r| r.game_id).collect();
    let ph7_ids: Vec<usize> = res_ph7.iter().map(|r| r.game_id).collect();
    assert_eq!(
        bare_ids, ph7_ids,
        "Both queries should match the exact same game IDs"
    );
}

#[test]
fn test_manual_examples_all_valid() {
    let manual_queries = vec![
        // Chapter 1: Headers
        r#"player "Kasparov" and white_elo >= 2700 and date >= "2000" and result "1-0""#,
        r#"avg_elo >= 2700 and result != "1/2-1/2" and date >= "2018""#,
        r#"player ~ "Kasparov|Karpov" and eco startswith "E""#,
        r#"raw_elo_diff >= 300 and result "0-1""#,
        r#"tag "TimeControl" == "300+0""#,
        r#"header "Annotator" contains "Stockfish""#,
        r#"tag "Variant" != "Standard""#,
        // Chapter 2: Placement & Counts
        r#"Kd4 and qd8 and Pa5 and [Qq] == 0"#,
        r#"Knight on [c3, d5, f3]"#,
        r#"P on a1-h2 == 0"#,
        r#"B on a1..h8"#,
        r#"R on a1-8"#,
        r#"R on a-h1 == 0"#,
        r#"B on light"#,
        r#"b on dark"#,
        r#"queens == 0"#,
        r#"rooks >= 3"#,
        r#"knights == 4"#,
        r#"white_pawns <= 4"#,
        r#"black_bishops == 2"#,
        r#"[Qq] == 0"#,
        r#"[Rr] >= 3"#,
        r#"[BNbn] == 0"#,
        r#"[Aa] on [e4, d4, e5, d5] >= 2"#,
        r#"white_light_bishops == 1 and black_dark_bishops == 1"#,
        r#"light_bishops == 2 and dark_bishops == 0"#,
        r#"wtm and check"#,
        r#"btm and checkmate"#,
        r#"not check and legal == 0"#,
        r#"legal count >= 30"#,
        r#"fen "*/*/*/*ppA*/*/*/*/*""#,
        r#"fen "*/*/*/*/4k3/*/*/*""#,
        // Chapter 3: Moves & Paths
        r#"path [e4 ... d5 ... Pe7xd8=Q]"#,
        r#"path [Ph6--h7]"#,
        r#"path [Bxh7 kxh7]"#,
        r#"path [A--=Q]"#,
        r#"path [Bxh7 kxh7 ... Ng5+ kg8 ... Qh5]"#,
        r#"path [P--="RBN"]"#,
        r#"path [Ph6--h7 ... Ph7--h8=Q]"#,
        r#"Bc4 bb6 Pa5 and path [bxf2+ Kxf2]"#,
        r#"path [e4 --+ d5]"#,
        r#"path [e4 --{0, 2} e5]"#,
        r#"path [e4 --{1} d6]"#,
        // Chapter 4: Pawn Structures
        r#"passed_pawns white >= 1 and passed_pawns black == 0 and [Qq] == 0"#,
        r#"Pd4 and isolated white == 1 and white_pawns >= 5"#,
        r#"doubled_pawns black >= 1 and isolated black >= 1"#,
        r#"passed_pawns white >= 2 and passed_pawns black == 0"#,
        r#"isolated_pawns white == 1 and [Qq] == 0"#,
        r#"pawn_islands white <= 2 and pawn_islands black >= 3"#,
        r#"backward_pawns black >= 1"#,
        // Chapter 5: Tactics & Motifs
        r#"pin(rook, knight, king)"#,
        r#"pin(bishop, black_knight, black_king)"#,
        r#"pin(white_bishop, black_queen, black_king)"#,
        r#"fork(knight, queen, rook)"#,
        r#"fork(pawn, bishop, knight)"#,
        r#"fork(N on c7, e8, a8)"#,
        r#"skewer(bishop, king, queen)"#,
        r#"skewer(rook, king, rook)"#,
        r#"trapped black_bishop"#,
        r#"trapped black_queen"#,
        r#"outpost knight on d5"#,
        r#"outpost white_knight on e5"#,
        r#"distance(K, k) <= 2"#,
        r#"attacks(g5, f6)"#,
        r#"attacks(B, k)"#,
        r#"is_attacked e4 by black"#,
        // Chapter 6: Material & Power
        r#"opposite_bishops and [Qq] == 0 and [Rr] == 0 and [Nn] == 0"#,
        r#"knights == 2 and bishops == 0 and [Qq] == 0 and [Rr] == 0"#,
        r#"rooks == 3 and knights == 3 and bishops == 2"#,
        r#"material_diff >= 5"#,
        r#"total_power <= 16 and [Qq] == 0"#,
        r#"white_power >= 30 and black_power <= 15"#,
        r#"white_power > black_power"#,
        r#"power_diff >= 5"#,
        r#"same_colored_bishops"#,
        // Chapter 7: Transformations & Symmetries
        r#"flipcolor { white "Carlsen" and fork(knight, queen, rook) }"#,
        r#"fliphorizontal { Bc4 and Qh5 and attacks(h7, f7) }"#,
        r#"flipcolor { passed_pawns white >= 1 and [Qq] == 0 and [Rr] == 0 and result "1-0" }"#,
        r#"flip:all { fen "8/8/8/8/8/8/4P3/8 w - - 0 1" }"#,
        r#"piece $minor in [N, B] { $minor on d5 and fork($minor, queen, rook) }"#,
        // Chapter 8: Boolean Logic, Timeline & Annotations
        r#"move_number <= 25 and result "1-0" and path [Bxh7 kxh7]"#,
        r#"move_number >= 35 and [Qq] == 0 and distance(K, k) <= 3"#,
        r#"avg_elo >= 2650 and (eco startswith "B" or eco startswith "E") and occurrences >= 2 { pin(bishop, knight, king) }"#,
        r#"ply in 1..20 { fork(knight, queen, rook) }"#,
        r#"move_number <= 10 and queens == 0"#,
        r#"occurrences >= 3 { check }"#,
        r#"comment contains "blunder""#,
        r#"comment contains "??""#,
        r#"nag $3"#,
        r#"nag $4 and nag $2"#,
    ];

    for (idx, query_str) in manual_queries.iter().enumerate() {
        let parsed = QueryParser::parse_str(query_str);
        assert!(
            parsed.is_ok(),
            "Manual query #{} failed to parse: {}\nError: {:?}",
            idx + 1,
            query_str,
            parsed.err()
        );
    }
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
