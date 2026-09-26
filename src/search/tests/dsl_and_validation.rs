use super::fixtures::*;
use crate::search::*;
use shakmaty::Color;

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

    // 4. Color mismatch in path: path black [Bg4]
    let bad_path_col = "path black [Bg4]";
    let err_path_col = QueryParser::parse_str(bad_path_col).unwrap_err();
    assert!(err_path_col.message.contains("Color mismatch"));
    assert!(err_path_col.help.as_ref().unwrap().contains("uppercase"));

    // 5. Impossible same-color capture in path: path [BxPh7] (White bishop takes white pawn)
    let bad_cap = "path [BxPh7]";
    let err_cap = QueryParser::parse_str(bad_cap).unwrap_err();
    assert!(err_cap.message.contains("cannot capture"));

    // 6. Impossible same-color capture in move filter: move from B to R
    let bad_move_cap = "move from B to R capture";
    let err_move_cap = QueryParser::parse_str(bad_move_cap).unwrap_err();
    assert!(err_move_cap.message.contains("cannot capture"));
}

#[test]
fn test_query_comments_support() {
    let q_str = r#"
        // This is a line comment with double slashes
        /* This is a block
           comment spanning lines */
        attacks B k // inline comment
    "#;
    let query = QueryParser::parse_str(q_str).unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&query, OPERA_GAME);
    assert!(res.is_match);
    assert!(res.matching_plies.contains(&21));

    // Verify that # is treated as checkmate symbol in SAN / path and not as a comment
    let q_san_hash = QueryParser::parse_str("path [Qb8+ ... Rd8#]").unwrap();
    let res_hash = GameSearchEvaluator::evaluate_pgn(&q_san_hash, OPERA_GAME);
    assert!(res_hash.is_match);
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
    assert_eq!(exp_fen.branches.len(), 10);
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

    // 3. Shift explanation: shiftvertical e1 -> 8 vertical branches (e1, e2, e3, e4, e5, e6, e7, e8)
    let shift_exp = QueryParser::explain("shiftvertical e1").unwrap();
    assert!(shift_exp.has_symmetries);
    assert_eq!(shift_exp.branches.len(), 8);
    assert_eq!(shift_exp.branches[0].dsl, "e1");
    assert_eq!(shift_exp.branches[1].dsl, "e2");
    assert_eq!(shift_exp.branches[2].dsl, "e3");
    assert_eq!(shift_exp.branches[3].dsl, "e4");
    assert_eq!(shift_exp.branches[4].dsl, "e5");
    assert_eq!(shift_exp.branches[5].dsl, "e6");
    assert_eq!(shift_exp.branches[6].dsl, "e7");
    assert_eq!(shift_exp.branches[7].dsl, "e8");

    // 4. Complex compound query to_dsl
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
        r#"path [Bg4+]"#,
        r#"path [bg4+]"#,
        r#"move legal mate count >= 2"#,
        r#"move legal mate count == 1"#,
        r#"move legal piece Q mate count >= 1"#,
        r#"move legal piece N mate count >= 1"#,
        r#"move legal capture mate count >= 1"#,
        r#"move legal en_passant mate count >= 1"#,
        r#"move legal castle mate count >= 1"#,
        r#"move legal count == 0"#,
        r#"move from Q to f7 capture"#,
        r#"move previous castle"#,
        r#"check and move previous piece B"#,
        r#"check and move mate"#,
        r#"check and move piece Q mate"#,
        r#"check and move capture mate"#,
        r#"move en_passant mate"#,
        r#"move castle mate"#,
        r#"move promote Q mate"#,
        r#"path [O-O-O ... --#]"#,
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
        r#"attacks(P, [qr])"#,
        r#"attacks(P, [q])"#,
        r#"attacks(P, [q, r])"#,
        r#"attacks(k, _)"#,
        r#"attacks(k, empty)"#,
        r#"attacks(K, .)"#,
        r#"attacks(A,a)&attacks(a,a)"#,
        r#"attacks(K,r)&a1-b3"#,
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
        r#"shifthorizontal { piece P on d4 and piece P on e4 }"#,
        r#"shiftvertical { piece R on f1 and piece K on f2 }"#,
        r#"shift { piece Q on f7 and piece B on c4 }"#,
        r#"shift:horizontal { pin(bishop, knight, king) }"#,
        r#"shift_all { outpost knight on c4 }"#,
        r#"shift { attacks(R, k) >= 1 }"#,
        r#"shifthorizontal { move from d2 to d4 }"#,
        r#"piece $minor in [N, B] { $minor on d5 and fork($minor, queen, rook) }"#,
        // Chapter 8: Boolean Logic, Timeline & Annotations
        r#"move_number <= 25 and result "1-0" and path [Bxh7 kxh7]"#,
        r#"move_number >= 35 and [Qq] == 0 and distance(K, k) <= 3"#,
        r#"avg_elo >= 2650 and (eco startswith "B" or eco startswith "E") and occurrences >= 2 { pin(bishop, knight, king) }"#,
        r#"ply in 1..20 { fork(knight, queen, rook) }"#,
        r#"move_number <= 10 and queens == 0"#,
        r#"occurrences >= 3 { check }"#,
        r#"attacks(R, b) and move previous e4"#,
        r#"attacks(R, b) and move e4"#,
        r#"cqlpath { e4 { attacks(R, b) } }"#,
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
fn test_query_semantic_validation_contradictions() {
    // 1. User's exact case: wtm + move previous from Q (impossible) vs wtm + move previous from q (valid)
    let err_wtm_prev_white = QueryParser::parse_str(
        r#"
        white == "Alexander2magnus"
        wtm
        mate
        move previous from Q
    "#,
    )
    .unwrap_err();
    assert!(err_wtm_prev_white
        .message
        .contains("previous move was played by Black"));

    let q_wtm_prev_black = QueryParser::parse_str(
        r#"
        white == "Alexander2magnus"
        wtm
        mate
        move previous from q
    "#,
    );
    assert!(
        q_wtm_prev_black.is_ok(),
        "Valid query with wtm and previous Black move must succeed"
    );

    // 2. btm + move previous from q (impossible) vs btm + move previous from Q (valid)
    let err_btm_prev_black = QueryParser::parse_str("btm and move previous from q").unwrap_err();
    assert!(err_btm_prev_black
        .message
        .contains("previous move was played by White"));

    let q_btm_prev_white = QueryParser::parse_str("btm and move previous from Q");
    assert!(q_btm_prev_white.is_ok());

    // 3. Current move contradiction: wtm + move from q (Black cannot move on White turn)
    let err_wtm_cur_black = QueryParser::parse_str("wtm and move from q").unwrap_err();
    assert!(err_wtm_cur_black
        .message
        .contains("Black cannot play a move when it is wtm (White to move)"));

    // 4. Current move contradiction: btm + move from Q (White cannot move on Black turn)
    let err_btm_cur_white = QueryParser::parse_str("btm and move from Q").unwrap_err();
    assert!(err_btm_cur_white
        .message
        .contains("White cannot play a move when it is btm (Black to move)"));

    // 5. Conflicting turn assertions: wtm and btm
    let err_conflicting_turns = QueryParser::parse_str("wtm and btm").unwrap_err();
    assert!(err_conflicting_turns
        .message
        .contains("Contradictory turn assertions"));

    // 6. Conflicting board states: mate and not check
    let err_mate_not_check = QueryParser::parse_str("mate and not check").unwrap_err();
    assert!(err_mate_not_check
        .message
        .contains("'mate' requires 'check'"));

    // 7. Conflicting board states: stalemate and check
    let err_stalemate_check = QueryParser::parse_str("stalemate and check").unwrap_err();
    assert!(err_stalemate_check
        .message
        .contains("'stalemate' requires that the king is NOT in check"));

    // 8. Conflicting board states: mate and stalemate
    let err_mate_stalemate = QueryParser::parse_str("mate and stalemate").unwrap_err();
    assert!(err_mate_stalemate
        .message
        .contains("'mate' and 'stalemate' are mutually exclusive"));

    // 9. King adjacency: Kd4 and kd5 (adjacent kings)
    let err_adjacent_kings = QueryParser::parse_str("Kd4 and kd5").unwrap_err();
    assert!(err_adjacent_kings.message.contains("adjacent"));

    let err_adjacent_kings_diag = QueryParser::parse_str("Kd4 and ke5").unwrap_err();
    assert!(err_adjacent_kings_diag.message.contains("adjacent"));

    let err_adjacent_kings_rank = QueryParser::parse_str("Kd4 and ke4").unwrap_err();
    assert!(err_adjacent_kings_rank.message.contains("adjacent"));

    // Legal separated kings must parse cleanly
    let ok_kings_separated = QueryParser::parse_str("Kd4 and kd6");
    assert!(ok_kings_separated.is_ok());

    let ok_kings_far = QueryParser::parse_str("Ke1 and ke8");
    assert!(ok_kings_far.is_ok());

    // 10. Conflicting piece placements on same square
    let err_same_square = QueryParser::parse_str("Kd4 and Qd4").unwrap_err();
    assert!(err_same_square
        .message
        .contains("Contradictory piece placement on square d4"));

    // 11. Pawns on impossible ranks (rank 1 or 8)
    let err_pawn_rank_1 = QueryParser::parse_str("Pd1").unwrap_err();
    assert!(err_pawn_rank_1
        .message
        .contains("White pawn cannot exist on rank 1"));

    let err_pawn_rank_8 = QueryParser::parse_str("pe8").unwrap_err();
    assert!(err_pawn_rank_8
        .message
        .contains("Black pawn cannot exist on rank 8"));

    let ok_pawn_rank_2 = QueryParser::parse_str("Pd2 and pe7");
    assert!(ok_pawn_rank_2.is_ok());

    // 12. Direct-check turn contradiction: Qd4 and kd5 and wtm (impossible) vs btm (valid)
    let err_direct_check_wtm = QueryParser::parse_str("Qd4 and kd5 and wtm").unwrap_err();
    assert!(err_direct_check_wtm
        .message
        .contains("directly attacked by White Queen on d4, but turn is 'wtm"));

    let ok_direct_check_btm = QueryParser::parse_str("Qd4 and kd5 and btm");
    assert!(
        ok_direct_check_btm.is_ok(),
        "Qd4 and kd5 with btm is a valid check position"
    );

    // 13. Direct check with 'not check' assertion contradiction
    let err_direct_check_not_check =
        QueryParser::parse_str("Qd4 and kd5 and btm and not check").unwrap_err();
    assert!(err_direct_check_not_check
        .message
        .contains("in direct check from White Queen on d4, but 'not check' was asserted"));

    // 14. Knight unblockable direct check on attacker turn
    let err_knight_check_wtm = QueryParser::parse_str("Nc3 and kd5 and wtm").unwrap_err();
    assert!(err_knight_check_wtm
        .message
        .contains("directly attacked by White Knight on c3, but turn is 'wtm"));

    // 15. Pawn direct check on attacker turn
    let err_pawn_check_wtm = QueryParser::parse_str("Pe4 and kd5 and wtm").unwrap_err();
    assert!(err_pawn_check_wtm
        .message
        .contains("directly attacked by White Pawn on e4, but turn is 'wtm"));

    // 16. Black attacking White king with btm
    let err_black_queen_check_btm = QueryParser::parse_str("qd4 and Kd5 and btm").unwrap_err();
    assert!(err_black_queen_check_btm
        .message
        .contains("directly attacked by Black Queen on d4, but turn is 'btm"));

    // 17. Multiple Kings of the same color
    let err_multiple_white_kings = QueryParser::parse_str("Ke1 and Ke8").unwrap_err();
    assert!(err_multiple_white_kings
        .message
        .contains("multiple White Kings"));

    // 18. Impossible King count (K == 0 or K > 1)
    let err_king_count_0 = QueryParser::parse_str("K == 0").unwrap_err();
    assert!(err_king_count_0.message.contains("King count cannot be 0"));

    let err_king_count_2 = QueryParser::parse_str("K == 2").unwrap_err();
    assert!(err_king_count_2.message.contains("King count cannot be 2"));

    // 19. Impossible Queen count (Q > 9)
    let err_queen_count_10 = QueryParser::parse_str("Q == 10").unwrap_err();
    assert!(err_queen_count_10
        .message
        .contains("Queen count cannot be 10"));

    // 20. Impossible Pawn count (P > 8)
    let err_pawn_count_9 = QueryParser::parse_str("P == 9").unwrap_err();
    assert!(err_pawn_count_9.message.contains("Pawn count cannot be 9"));

    // 21. Impossible Rook count (R > 10)
    let err_rook_count_11 = QueryParser::parse_str("R >= 11").unwrap_err();
    assert!(err_rook_count_11
        .message
        .contains("Rook count cannot be 11"));

    // 22. Promotion + Pawn cumulative overflow: 9 Queens + 1 Pawn = 9 pawns needed (impossible)
    let err_promo_pawn_overflow = QueryParser::parse_str("Q == 9 and P == 1").unwrap_err();
    assert!(err_promo_pawn_overflow
        .message
        .contains("requires 8 promoted pieces but also has 1 unpromoted pawns"));

    let err_total_pieces_overflow = QueryParser::parse_str("Q == 9 and P == 8").unwrap_err();
    assert!(err_total_pieces_overflow
        .message
        .contains("total pieces (maximum is 16)"));

    // 23. Valid promotion configuration: 9 Queens and 0 Pawns
    let ok_promo_9_queens = QueryParser::parse_str("Q == 9 and P == 0");
    assert!(
        ok_promo_9_queens.is_ok(),
        "9 Queens with 0 Pawns is a valid theoretical promotion maximum"
    );

    // 24. Castling rights vs King/Rook placement
    let err_castling_king_moved =
        QueryParser::parse_str("castling(white, kingside) and Kd4").unwrap_err();
    assert!(err_castling_king_moved
        .message
        .contains("White castling rights require White King on e1"));

    let err_castling_rook_missing =
        QueryParser::parse_str("castling(white, kingside) and Bh1").unwrap_err();
    assert!(err_castling_rook_missing
        .message
        .contains("White kingside castling requires a White Rook on h1"));

    // 25. Castling move while in check
    let err_castle_out_of_check = QueryParser::parse_str("check and move O-O").unwrap_err();
    assert!(err_castle_out_of_check
        .message
        .contains("castling is illegal while the King is currently in check"));

    // 26. Ply vs Turn arithmetic contradiction
    let err_ply_turn_contradiction = QueryParser::parse_str("wtm and ply == 10").unwrap_err();
    assert!(err_ply_turn_contradiction
        .message
        .contains("ply 10 is Black to move (even ply), but 'wtm' was asserted"));

    // 27. Move number vs Ply arithmetic contradiction
    let err_move_ply_contradiction =
        QueryParser::parse_str("move_number == 5 and ply == 12").unwrap_err();
    assert!(err_move_ply_contradiction
        .message
        .contains("Move 5 corresponds to ply 9 (wtm) or ply 10 (btm), but ply was asserted as 12"));

    let ok_move_ply_match = QueryParser::parse_str("wtm and move_number == 5 and ply == 9");
    assert!(ok_move_ply_match.is_ok());

    // 28. Bishop color distribution promotion overflow
    let err_light_bishops_promo =
        QueryParser::parse_str("white_light_bishops == 2 and P == 8").unwrap_err();
    assert!(err_light_bishops_promo
        .message
        .contains("requires 1 promoted pieces but also has 8 unpromoted pawns"));

    // 29. Triple Check impossibility
    let err_triple_check = QueryParser::parse_str("Ke1 and qe2 and qd1 and qf1").unwrap_err();
    assert!(err_triple_check
        .message
        .contains("Triple check is impossible"));
}

#[test]
fn test_parent_and_child_scoping() {
    let pgn = r#"[Event "Fool's Mate"]
[Site "?"]
[Date "2024.01.01"]
[Round "1"]
[White "Player1"]
[Black "Player2"]
[Result "0-1"]

1. f3 e5 2. g4 Qh4# 0-1
"#;
    // Plies in game:
    // ply 0: start pos
    // ply 1: 1. f3
    // ply 2: 1... e5
    // ply 3: 2. g4
    // ply 4: 2... Qh4# (mate)

    // Test 1: Checkmate position (ply 4) has parent with check == false and child == nothing
    let q_mate_parent_not_check = QueryParser::parse_str("mate and parent { not check }").unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&q_mate_parent_not_check, pgn);
    assert!(res.is_match);
    assert_eq!(res.matching_plies, vec![4]);

    // Test 2: Position before mate (ply 3: 2. g4) has child { mate }
    let q_child_mate = QueryParser::parse_str("child { mate }").unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&q_child_mate, pgn);
    assert!(res.is_match);
    assert_eq!(res.matching_plies, vec![3]);

    // Test 3: Grandchild mate from ply 2: child { child { mate } }
    let q_grandchild_mate = QueryParser::parse_str("child { child { mate } }").unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&q_grandchild_mate, pgn);
    assert!(res.is_match);
    assert_eq!(res.matching_plies, vec![2]);

    // Test 4: Nested parent at ply 4: parent { parent { wtm } }
    // At ply 4 (btm after Qh4#), ply 3 was btm, ply 2 was wtm.
    let q_parent_parent_wtm = QueryParser::parse_str("mate and parent { parent { wtm } }").unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&q_parent_parent_wtm, pgn);
    assert!(res.is_match);
    assert_eq!(res.matching_plies, vec![4]);

    // Test 5: Child and parent combined
    let q_child_and_parent = QueryParser::parse_str("parent { wtm } and child { mate }").unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&q_child_and_parent, pgn);
    assert!(res.is_match);
    assert_eq!(res.matching_plies, vec![3]);

    // Test 6: Boundary conditions - ply 1 (1. f3) has parent at ply 0 (start pos, wtm)
    let q_ply1_parent = QueryParser::parse_str("ply == 1 and parent { wtm }").unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&q_ply1_parent, pgn);
    assert!(res.is_match);
    assert_eq!(res.matching_plies, vec![1]);

    // But ply 1 parent { parent { wtm } } does not exist because parent of parent of ply 1 is before start pos
    let q_ply1_parent_parent =
        QueryParser::parse_str("ply == 1 and parent { parent { wtm } }").unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&q_ply1_parent_parent, pgn);
    assert!(!res.is_match);

    // Final pos (ply 4) has no child
    let q_end_child = QueryParser::parse_str("ply == 4 and child { btm }").unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&q_end_child, pgn);
    assert!(!res.is_match);

    // Test 7: ToDsl roundtrip
    assert_eq!(
        explain::ToDsl::to_dsl(&q_mate_parent_not_check),
        "checkmate and parent { not check }"
    );
    assert_eq!(explain::ToDsl::to_dsl(&q_child_mate), "child { checkmate }");
}

#[test]
fn test_initial_and_terminal_filters() {
    // Fools Mate: 1. f3 e5 2. g4 Qh4# (4 plies, total positions: 5 (ply 0, 1, 2, 3, 4))
    let fools_mate_pgn = r#"[Event "Casual Game"]
[Site "Berlin"]
[Date "1861.??.??"]
[White "White Player"]
[Black "Black Player"]
[Result "0-1"]

1. f3 e5 2. g4 Qh4# 0-1"#;

    // 1. Terminal ply exact matching vs general ply
    // `ply == 4` matches any game that reached at least 4 plies
    let q_ply_4 = QueryParser::parse_str("ply == 4").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_ply_4, fools_mate_pgn).is_match);

    // `terminal { ply == 4 }` matches ONLY games whose terminal position is exactly ply 4
    let q_term_ply_4 = QueryParser::parse_str("terminal { ply == 4 }").unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&q_term_ply_4, fools_mate_pgn);
    assert!(res.is_match);
    assert_eq!(res.matching_plies, vec![4]);

    // `terminal { ply == 10 }` does NOT match Fools Mate (since Fools Mate ended at ply 4, not 10)
    let q_term_ply_10 = QueryParser::parse_str("terminal { ply == 10 }").unwrap();
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_term_ply_10, fools_mate_pgn).is_match);

    // 2. Terminal mate
    let q_term_mate = QueryParser::parse_str("terminal { mate }").unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&q_term_mate, fools_mate_pgn);
    assert!(res.is_match);
    assert_eq!(res.matching_plies, vec![4]);

    // Unary syntax without braces: `terminal mate`
    let q_term_mate_bare = QueryParser::parse_str("terminal mate").unwrap();
    let res_bare = GameSearchEvaluator::evaluate_pgn(&q_term_mate_bare, fools_mate_pgn);
    assert!(res_bare.is_match);
    assert_eq!(res_bare.matching_plies, vec![4]);

    // 3. Terminal attacks: Black Queen attacks White King in terminal position
    let q_term_attacks = QueryParser::parse_str("terminal attacks(q, K)").unwrap();
    let res_atk = GameSearchEvaluator::evaluate_pgn(&q_term_attacks, fools_mate_pgn);
    assert!(res_atk.is_match);
    assert_eq!(res_atk.matching_plies, vec![4]);

    // White Bishop attacking Black Queen in terminal position does NOT match Fools Mate
    let q_term_b_atk_q = QueryParser::parse_str("terminal attacks(B, q)").unwrap();
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_term_b_atk_q, fools_mate_pgn).is_match);

    // 4. Initial filter:
    // `initial { wtm }` matches initial position (ply 0)
    let q_init_wtm = QueryParser::parse_str("initial { wtm }").unwrap();
    let res_init = GameSearchEvaluator::evaluate_pgn(&q_init_wtm, fools_mate_pgn);
    assert!(res_init.is_match);
    assert_eq!(res_init.matching_plies, vec![0]);

    // `initial { piece P on e2 }`
    let q_init_pe2 = QueryParser::parse_str("initial { piece P on e2 }").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_init_pe2, fools_mate_pgn).is_match);

    // `initial { btm }` does not match standard start position
    let q_init_btm = QueryParser::parse_str("initial { btm }").unwrap();
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_init_btm, fools_mate_pgn).is_match);

    // `initial { ply == 10 }` does not match (initial is ply 0)
    let q_init_ply10 = QueryParser::parse_str("initial { ply == 10 }").unwrap();
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_init_ply10, fools_mate_pgn).is_match);

    // 5. Combined initial and terminal logic
    let q_init_and_term =
        QueryParser::parse_str("initial { piece P on e2 } and terminal { mate }").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_init_and_term, fools_mate_pgn).is_match);

    // 6. ToDsl formatting & roundtrip parsing
    let dsl_term = explain::ToDsl::to_dsl(&q_term_ply_4);
    assert_eq!(dsl_term, "terminal { ply == 4 }");
    let reparsed = QueryParser::parse_str(&dsl_term).unwrap();
    assert_eq!(explain::ToDsl::to_dsl(&reparsed), dsl_term);

    let dsl_init = explain::ToDsl::to_dsl(&q_init_pe2);
    assert_eq!(dsl_init, "initial { P on e2 }");
    let reparsed_init = QueryParser::parse_str(&dsl_init).unwrap();
    assert_eq!(explain::ToDsl::to_dsl(&reparsed_init), dsl_init);
}
