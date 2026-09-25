use super::fixtures::*;
use crate::search::*;
use shakmaty::Color;
use std::collections::HashMap;

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

    // 4b. Legal Mate in 1 analysis: at ply 32, White has legal move 17. Rd8# delivering mate
    let q_legal_mate = QueryParser::parse_str("legal mate count >= 1").unwrap();
    let res_legal_mate = GameSearchEvaluator::evaluate_pgn(&q_legal_mate, OPERA_GAME);
    assert!(
        res_legal_mate.is_match,
        "Must match ply 32 where White has legal mate in 1 (17. Rd8#)"
    );
    assert_eq!(res_legal_mate.matching_plies, vec![32]);

    let q_legal_mate_rook = QueryParser::parse_str("legal mate piece R").unwrap();
    let res_legal_mate_r = GameSearchEvaluator::evaluate_pgn(&q_legal_mate_rook, OPERA_GAME);
    assert!(res_legal_mate_r.is_match);
    assert_eq!(res_legal_mate_r.matching_plies, vec![32]);

    let q_legal_mate_queen = QueryParser::parse_str("legal mate piece Q").unwrap();
    let res_legal_mate_q = GameSearchEvaluator::evaluate_pgn(&q_legal_mate_queen, OPERA_GAME);
    assert!(
        !res_legal_mate_q.is_match,
        "Queen was sacrificed, cannot mate"
    );

    // 4c. Multiple mates in 1 on custom puzzle: White has 3 distinct legal checkmates (Qg7#, Qh7#, Ra8#)
    let multi_mate_fen = "7k/1Q6/6K1/8/8/8/8/R7 w - - 0 1";
    let multi_mate_pos: shakmaty::Chess = shakmaty::fen::Fen::from_ascii(multi_mate_fen.as_bytes())
        .unwrap()
        .into_position(shakmaty::CastlingMode::Standard)
        .unwrap();
    let q_multi_mate = QueryParser::parse_str("legal mate count >= 2").unwrap();
    let res_multi = GameSearchEvaluator::evaluate_with_timeline(
        &q_multi_mate,
        &HashMap::new(),
        std::slice::from_ref(&multi_mate_pos),
        &[],
    );
    assert!(res_multi.is_match, "Position has multiple legal checkmates");

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
fn test_square_set_algebra_and_bitboard_engine() {
    let opera_pgn = r#"[Event "Paris"]
[Site "Paris FRA"]
[Date "1858.??.??"]
[White "Paul Morphy"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0
"#;

    // 1. Double attack check: attacks(R, k) >= 2 or attacks(R, k) >= 1
    let q_double_attack = QueryParser::parse_str("attacks(R, k) >= 1").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_double_attack, opera_pgn).is_match);

    // 2. Set Intersection: B [c4, g5] (Morphy has bishop on c4 and g5)
    let q_bishops_on_sqs = QueryParser::parse_str("B [c4, g5] >= 2").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_bishops_on_sqs, opera_pgn).is_match);

    // 3. Set Union: (N | B) [b5, g5] >= 2
    let q_union = QueryParser::parse_str("(N | B) [b5, g5] >= 2").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_union, opera_pgn).is_match);

    // 4. Set Difference: occupied \ [e4, d4]
    let q_diff = QueryParser::parse_str("(occupied \\ [e4, d4]) count >= 30").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_diff, opera_pgn).is_match);

    // 5. Set Complement: ~(occupied) (i.e. empty squares)
    let q_comp = QueryParser::parse_str("~occupied count >= 32").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_comp, opera_pgn).is_match);

    // 6. Set Size Comparison: attacks(white_pieces, [d1..d8]) > attacks(black_pieces, [d1..d8])
    let q_control_dominance =
        QueryParser::parse_str("attacks(white_pieces, [d1..d8]) > attacks(black_pieces, [d1..d8])")
            .unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_control_dominance, opera_pgn).is_match);

    // 7. Non-empty boolean truthiness
    let q_truthiness = QueryParser::parse_str("B [c4, g5]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_truthiness, opera_pgn).is_match);

    // 8. Roundtripping to canonical DSL
    let explained = crate::search::explain_query("attacks(R, k) >= 1", &q_double_attack);
    assert!(explained.canonical_dsl.contains("attacks(R, k) >= 1"));
}

#[test]
fn test_cqli_direction_spatial_shifts_and_rotations() {
    use crate::search::squares::SquareSetEvaluator;
    use shakmaty::fen::Fen;
    use shakmaty::{CastlingMode, Chess};

    // 1. Test parsing of CQL / CQLi spatial shift expressions
    let q1 = QueryParser::parse_str("northwest 2 Q & up 1 k & R").unwrap();
    let q2 = QueryParser::parse_str("right 1 k & _").unwrap();
    assert!(matches!(q1, SearchQuery::SquareSet(_)));
    assert!(matches!(q2, SearchQuery::SquareSet(_)));

    // 2. Test full CQLi query: mate + flipcolor rotate90 { northwest 2 Q & up 1 k & R \n right 1 k & _ }
    let cql_query = r#"
        mate
        flipcolor rotate90 {
            northwest 2 Q & up 1 k & R
            right 1 k & _
        }
    "#;
    let parsed_cql = QueryParser::parse_str(cql_query).unwrap();
    assert!(matches!(parsed_cql, SearchQuery::And(_)));

    // 3. Test Evaluation on a concrete position:
    // White Queen on g6, White Rook on e8, Black King on e7, White Pawn on c6, f7 empty
    let fen_str = "4R3/4k3/2P3Q1/8/8/8/8/K7 b - - 0 1";
    let fen: Fen = fen_str.parse().unwrap();
    let pos: Chess = fen.into_position(CastlingMode::Chess960).unwrap();

    // Check individual shift evaluations
    let eval1 = SquareSetEvaluator::eval_expr(
        &SquareSetExpr::Intersection(
            Box::new(SquareSetExpr::Shift {
                direction: Direction::NorthWest,
                min_dist: 2,
                max_dist: 2,
                expr: Box::new(SquareSetExpr::Piece(SquareContent::Piece(
                    shakmaty::Piece {
                        color: Color::White,
                        role: shakmaty::Role::Queen,
                    },
                ))),
            }),
            Box::new(SquareSetExpr::Intersection(
                Box::new(SquareSetExpr::Shift {
                    direction: Direction::Up,
                    min_dist: 1,
                    max_dist: 1,
                    expr: Box::new(SquareSetExpr::Piece(SquareContent::Piece(
                        shakmaty::Piece {
                            color: Color::Black,
                            role: shakmaty::Role::King,
                        },
                    ))),
                }),
                Box::new(SquareSetExpr::Piece(SquareContent::Piece(
                    shakmaty::Piece {
                        color: Color::White,
                        role: shakmaty::Role::Rook,
                    },
                ))),
            )),
        ),
        &pos,
        &std::collections::HashMap::new(),
    );
    assert!(!eval1.is_empty(), "Square e8 must match the intersection");

    // Match full CQL query using matches_single_ply
    let is_matched = crate::search::evaluator::matches_single_ply(&parsed_cql, &pos, 0, None);
    assert!(is_matched, "The CQLi query must match the mating geometry!");
}

#[test]
fn test_bracket_set_comparisons() {
    use crate::search::query::*;
    use shakmaty::fen::Fen;
    use shakmaty::{CastlingMode, Chess};

    // 1. Test parsing set-to-set comparisons
    let q1 = QueryParser::parse_str("[Aa] == [KkPp]").unwrap();
    assert!(matches!(
        q1,
        SearchQuery::SquareSet(SetPredicate::SetComparison {
            op: ComparisonOp::Equal,
            ..
        })
    ));

    let q2 = QueryParser::parse_str("[Aa] == []").unwrap();
    assert!(matches!(
        q2,
        SearchQuery::SquareSet(SetPredicate::SetComparison {
            op: ComparisonOp::Equal,
            ..
        })
    ));

    let q3 = QueryParser::parse_str("[Qq] > [Rr]").unwrap();
    assert!(matches!(
        q3,
        SearchQuery::SquareSet(SetPredicate::SetComparison {
            op: ComparisonOp::GreaterThan,
            ..
        })
    ));

    let q4 = QueryParser::parse_str("[Aa] != [KkPp]").unwrap();
    assert!(matches!(
        q4,
        SearchQuery::SquareSet(SetPredicate::SetComparison {
            op: ComparisonOp::NotEqual,
            ..
        })
    ));

    // 2. Evaluate on pure King & Pawn endgame: White K on e2, P on e4; Black K on e7, P on d5
    let kp_fen = "8/4k3/8/3p4/4P3/8/4K3/8 w - - 0 1";
    let fen: Fen = kp_fen.parse().unwrap();
    let pos_kp: Chess = fen.into_position(CastlingMode::Chess960).unwrap();

    // All pieces on board are Kings and Pawns
    assert!(crate::search::evaluator::matches_single_ply(
        &q1, &pos_kp, 0, None
    ));
    assert!(!crate::search::evaluator::matches_single_ply(
        &q4, &pos_kp, 0, None
    ));

    // No queens on board -> [Qq] == []
    let q_no_queens = QueryParser::parse_str("[Qq] == []").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_no_queens,
        &pos_kp,
        0,
        None
    ));

    // Board is not empty -> [Aa] == [] is false
    assert!(!crate::search::evaluator::matches_single_ply(
        &q2, &pos_kp, 0, None
    ));

    // 2 Kings == 2 Pawns -> [Kk] == [Pp]
    let q_kings_eq_pawns = QueryParser::parse_str("[Kk] == [Pp]").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_kings_eq_pawns,
        &pos_kp,
        0,
        None
    ));

    // 3. Evaluate on position with an added Knight on g4
    let kpn_fen = "8/4k3/8/3p4/4P1N1/8/4K3/8 w - - 0 1";
    let fen_kpn: Fen = kpn_fen.parse().unwrap();
    let pos_kpn: Chess = fen_kpn.into_position(CastlingMode::Chess960).unwrap();

    // Not a pure pawn endgame anymore
    assert!(!crate::search::evaluator::matches_single_ply(
        &q1, &pos_kpn, 0, None
    ));
    assert!(crate::search::evaluator::matches_single_ply(
        &q4, &pos_kpn, 0, None
    ));

    // More total pieces than kings & pawns -> [Aa] > [KkPp]
    let q_more_pieces = QueryParser::parse_str("[Aa] > [KkPp]").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_more_pieces,
        &pos_kpn,
        0,
        None
    ));

    // More knights than bishops -> [Nn] > [Bb]
    let q_knights_gt_bishops = QueryParser::parse_str("[Nn] > [Bb]").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_knights_gt_bishops,
        &pos_kpn,
        0,
        None
    ));
}

#[test]
fn test_diagonal_flips_and_board_shifts() {
    use shakmaty::fen::Fen;
    use shakmaty::{CastlingMode, Chess};

    // 1. Test Main Diagonal Flip (a1-h8 reflection: (f, r) -> (r, f))
    // e2 (f=4, r=1) -> b5 (f=1, r=4)
    let q_diag1 = QueryParser::parse_str("flipmaindiagonal { piece P on b5 }").unwrap();
    let q_diag2 = QueryParser::parse_str("flip_diag { piece P on b5 }").unwrap();
    let q_diag3 = QueryParser::parse_str("flip:diagonal { piece P on b5 }").unwrap();

    let fen_e2: Fen = "8/4k3/8/8/8/8/4P3/4K3 w - - 0 1".parse().unwrap();
    let pos_e2: Chess = fen_e2.into_position(CastlingMode::Chess960).unwrap();

    assert!(crate::search::evaluator::matches_single_ply(
        &q_diag1, &pos_e2, 0, None
    ));
    assert!(crate::search::evaluator::matches_single_ply(
        &q_diag2, &pos_e2, 0, None
    ));
    assert!(crate::search::evaluator::matches_single_ply(
        &q_diag3, &pos_e2, 0, None
    ));

    // 2. Test Anti-Diagonal Flip (a8-h1 reflection: (f, r) -> (7-r, 7-f))
    // e2 (f=4, r=1) -> g4 (f=7-1=6 (g), r=7-4=3 (rank 4))
    let q_antidiag1 = QueryParser::parse_str("flipantidiagonal { piece P on g4 }").unwrap();
    let q_antidiag2 = QueryParser::parse_str("flip_antidiag { piece P on g4 }").unwrap();
    let q_antidiag3 = QueryParser::parse_str("flip:antidiagonal { piece P on g4 }").unwrap();

    assert!(crate::search::evaluator::matches_single_ply(
        &q_antidiag1,
        &pos_e2,
        0,
        None
    ));
    assert!(crate::search::evaluator::matches_single_ply(
        &q_antidiag2,
        &pos_e2,
        0,
        None
    ));
    assert!(crate::search::evaluator::matches_single_ply(
        &q_antidiag3,
        &pos_e2,
        0,
        None
    ));

    // 3. Test Bare flip { ... } defaults to horizontal flip
    // e2 (f=4, r=1) -> d2 (f=7-4=3 (d), r=1)
    let q_bare_flip = QueryParser::parse_str("flip { piece P on d2 }").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_bare_flip,
        &pos_e2,
        0,
        None
    ));

    // 4. Test Shift Horizontal (df across files)
    // White pawn on e2 shifted horizontally matches pawn on c2, d2, e2, f2, etc.
    let q_shift_h = QueryParser::parse_str("shifthorizontal { piece P on a2 }").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_shift_h, &pos_e2, 0, None
    ));

    let q_shift_h2 = QueryParser::parse_str("shift_horizontal { piece P on a3 }").unwrap();
    // a3 has different rank, horizontal shift alone cannot match e2
    assert!(!crate::search::evaluator::matches_single_ply(
        &q_shift_h2,
        &pos_e2,
        0,
        None
    ));

    // 5. Test Shift Vertical (dr across ranks)
    let q_shift_v = QueryParser::parse_str("shiftvertical { piece P on e5 }").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_shift_v, &pos_e2, 0, None
    ));

    let q_shift_v2 = QueryParser::parse_str("shift_v { piece P on f5 }").unwrap();
    // f5 has different file, vertical shift alone cannot match e2
    assert!(!crate::search::evaluator::matches_single_ply(
        &q_shift_v2,
        &pos_e2,
        0,
        None
    ));

    // 6. Test Shift All / 2D board shift
    let q_shift_all = QueryParser::parse_str("shift { piece P on b3 }").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_shift_all,
        &pos_e2,
        0,
        None
    ));

    // 7. Test Shift with compound pawn structure
    // Pawns on d4, e4 shifted horizontally matches pawns on e4, f4 or c4, d4
    let center_pawns_fen: Fen = "8/4k3/8/8/3PP3/8/8/4K3 w - - 0 1".parse().unwrap();
    let pos_center: Chess = center_pawns_fen
        .into_position(CastlingMode::Chess960)
        .unwrap();

    let q_pawn_pair =
        QueryParser::parse_str("shifthorizontal { piece P on a4 and piece P on b4 }").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_pawn_pair,
        &pos_center,
        0,
        None
    ));
}

#[test]
fn test_top_level_piece_designators_and_square_set_operators() {
    use shakmaty::fen::Fen;
    use shakmaty::{CastlingMode, Chess};

    // Standard starting position
    let start_fen: Fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        .parse()
        .unwrap();
    let pos_start: Chess = start_fen.into_position(CastlingMode::Chess960).unwrap();

    // 1. Standalone Queen (Q) / Pawn (P) / Role (queen, pawn)
    let q_white_queen = QueryParser::parse_str("Q").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_white_queen,
        &pos_start,
        0,
        None
    ));

    let q_queen_role = QueryParser::parse_str("queen").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_queen_role,
        &pos_start,
        0,
        None
    ));

    // 2. Set Union (|) between piece designators
    // Q | P: White Queen or White Pawn squares (non-empty)
    let q_q_or_p = QueryParser::parse_str("Q | P").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_q_or_p, &pos_start, 0, None
    ));

    // queen | pawn: Any Queen or Pawn
    let q_queen_pawn = QueryParser::parse_str("queen | pawn").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_queen_pawn,
        &pos_start,
        0,
        None
    ));

    // [Q, R] | [B, N]
    let q_bracket_union = QueryParser::parse_str("[Q, R] | [B, N]").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_bracket_union,
        &pos_start,
        0,
        None
    ));

    // 3. Set Intersection (&)
    // Q & P: A single square cannot hold both a Queen and a Pawn -> always empty -> false
    let q_q_and_p = QueryParser::parse_str("Q & P").unwrap();
    assert!(!crate::search::evaluator::matches_single_ply(
        &q_q_and_p, &pos_start, 0, None
    ));

    // Q & d1: White Queen on d1 (start position has White Queen on d1) -> true
    let q_q_and_d1 = QueryParser::parse_str("Q & d1").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_q_and_d1,
        &pos_start,
        0,
        None
    ));

    // Q & light: Start position White Queen is on d1 (light square) -> true
    let q_q_and_light = QueryParser::parse_str("Q & light").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_q_and_light,
        &pos_start,
        0,
        None
    ));

    // Q & dark: Start position White Queen is not on a dark square -> false
    let q_q_and_dark = QueryParser::parse_str("Q & dark").unwrap();
    assert!(!crate::search::evaluator::matches_single_ply(
        &q_q_and_dark,
        &pos_start,
        0,
        None
    ));

    // 4. Set Difference (\)
    // occupied \ P: All occupied squares without a White pawn
    let q_occ_diff_p = QueryParser::parse_str("occupied \\ P").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_occ_diff_p,
        &pos_start,
        0,
        None
    ));

    // [Q, R] \ d1: White Queen on d1 and Rooks on a1, h1 -> non-empty because of a1, h1
    let q_qr_diff_d1 = QueryParser::parse_str("[Q, R] \\ d1").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_qr_diff_d1,
        &pos_start,
        0,
        None
    ));

    // Q \ d1: Queen is only on d1 in start pos -> empty -> false
    let q_q_diff_d1 = QueryParser::parse_str("Q \\ d1").unwrap();
    assert!(!crate::search::evaluator::matches_single_ply(
        &q_q_diff_d1,
        &pos_start,
        0,
        None
    ));

    // 5. Piece designators in endgame position (King + Pawn study)
    let kp_fen: Fen = "8/4k3/8/3p4/4P3/8/4K3/8 w - - 0 1".parse().unwrap();
    let pos_kp: Chess = kp_fen.into_position(CastlingMode::Chess960).unwrap();

    // No Queens on board in this endgame
    assert!(!crate::search::evaluator::matches_single_ply(
        &q_white_queen,
        &pos_kp,
        0,
        None
    ));
    assert!(!crate::search::evaluator::matches_single_ply(
        &q_queen_role,
        &pos_kp,
        0,
        None
    ));

    // Q | P is true because White Pawn on e4 exists
    assert!(crate::search::evaluator::matches_single_ply(
        &q_q_or_p, &pos_kp, 0, None
    ));
}

#[test]
fn test_documented_shift_transformation_examples() {
    use shakmaty::fen::Fen;
    use shakmaty::{CastlingMode, Chess};

    // 1. Horizontal Shift: Connected center pawns (e4, f4) matching (d4, e4) template
    let fen_center_pawns: Fen = "8/4k3/8/8/4PP2/8/8/4K3 w - - 0 1".parse().unwrap();
    let pos_pawns: Chess = fen_center_pawns
        .into_position(CastlingMode::Chess960)
        .unwrap();
    let q_shift_pawns =
        QueryParser::parse_str("shifthorizontal { piece P on d4 and piece P on e4 }").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_shift_pawns,
        &pos_pawns,
        0,
        None
    ));

    // 2. Vertical Shift: Rook behind King on f-file (Rf3, Kf4) matching (e1, e2) template
    let fen_vertical: Fen = "8/4k3/8/8/5K2/5R2/8/8 w - - 0 1".parse().unwrap();
    let pos_vert: Chess = fen_vertical.into_position(CastlingMode::Chess960).unwrap();
    let q_shift_vert =
        QueryParser::parse_str("shiftvertical { piece R on f1 and piece K on f2 }").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_shift_vert,
        &pos_vert,
        0,
        None
    ));

    // 3. 2D All-board Shift: Queen & Bishop battery (Qe6, Bb3) matching (Qf7, Bc4) template
    let fen_battery: Fen = "3k4/8/4Q3/8/8/1B6/8/4K3 w - - 0 1".parse().unwrap();
    let pos_battery: Chess = fen_battery.into_position(CastlingMode::Chess960).unwrap();
    let q_shift_all = QueryParser::parse_str("shift { piece Q on f7 and piece B on c4 }").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_shift_all,
        &pos_battery,
        0,
        None
    ));

    // 4. Shift with Pin: White Bishop on b5 pinning Black Knight on d7 against Black King on e8 (Opera motif) shifted
    // Let's test on standard Opera game position:
    let opera_pgn = r#"[Event "Paris"]
[Site "Paris FRA"]
[Date "1858.??.??"]
[White "Paul Morphy"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0
"#;
    let q_shift_pin =
        QueryParser::parse_str("shift:horizontal { pin(bishop, knight, king) }").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_shift_pin, opera_pgn).is_match);

    // 5. Shift with Outpost
    let fen_outpost: Fen = "8/8/8/3N4/4P3/8/8/4K2k w - - 0 1".parse().unwrap(); // Knight on d5 defended by pawn on e4 -> outpost
    let pos_outpost: Chess = fen_outpost.into_position(CastlingMode::Chess960).unwrap();
    let q_shift_outpost = QueryParser::parse_str("shift_all { outpost knight on c4 }").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_shift_outpost,
        &pos_outpost,
        0,
        None
    ));

    // 6. Shift with Attack square set
    let q_shift_attack = QueryParser::parse_str("shift { attacks(R, k) >= 1 }").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_shift_attack, opera_pgn).is_match);

    // 7. Shift with Move
    let q_shift_move = QueryParser::parse_str("shifthorizontal { move from d2 to d4 }").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_shift_move, opera_pgn).is_match);

    // 8. Shift with Bracketed Square Sets: shiftvertical [a1, a2]
    let q_shift_sq_set = QueryParser::parse_str("shiftvertical [a1, a2]").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_shift_sq_set,
        &pos_pawns,
        0,
        None
    ));
    let exp_shift = QueryParser::explain("shiftvertical [a1, a2]").unwrap();
    assert_eq!(exp_shift.canonical_dsl, "shiftvertical { [a1, a2] }");
}

#[test]
fn test_chained_directional_shifts_and_flips() {
    use shakmaty::fen::Fen;
    use shakmaty::{CastlingMode, Chess};

    // 1. Chained directional shift: `up 1 right 2 N`
    // White Knight on c3 (f=2, r=2).
    // right 2 N -> e3 (f=4, r=2)
    // up 1 right 2 N -> e4 (f=4, r=3)
    let q_knight_hop = QueryParser::parse_str("up 1 right 2 N").unwrap();
    let fen_c3: Fen = "8/8/8/8/8/2N5/8/4K2k w - - 0 1".parse().unwrap();
    let pos_c3: Chess = fen_c3.into_position(CastlingMode::Chess960).unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_knight_hop,
        &pos_c3,
        0,
        None
    ));

    // Intersection with e4: `(up 1 right 2 N) & e4` matches, but with d4 fails
    let q_e4 = QueryParser::parse_str("(up 1 right 2 N) & e4").unwrap();
    let q_d4 = QueryParser::parse_str("(up 1 right 2 N) & d4").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_e4, &pos_c3, 0, None
    ));
    assert!(!crate::search::evaluator::matches_single_ply(
        &q_d4, &pos_c3, 0, None
    ));

    // 2. Chained direction with flip: `flip northeast 1 up 1 Q`
    // In Original (Identity):
    // Queen on e1 -> up 1 (e2) -> northeast 1 (f3)
    // In Flipped (Horizontal mirror):
    // Queen on d1 (mirrored e1) -> up 1 (d2) -> northwest 1 (c3)
    let q_flip_q = QueryParser::parse_str("flip northeast 1 up 1 Q").unwrap();
    let fen_e1: Fen = "k7/8/8/8/8/8/8/4QK2 w - - 0 1".parse().unwrap(); // Queen on e1
    let pos_e1: Chess = fen_e1.into_position(CastlingMode::Chess960).unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_flip_q, &pos_e1, 0, None
    ));

    // Testing flipped square target:
    // With Queen on d1, original `northeast 1 up 1 Q` gives e3.
    // Flipped `northwest 1 up 1 Q` on d1 gives c3.
    let fen_d1: Fen = "k7/8/8/8/8/8/8/3QK3 w - - 0 1".parse().unwrap(); // Queen on d1
    let pos_d1: Chess = fen_d1.into_position(CastlingMode::Chess960).unwrap();
    // In flipped symmetry, Queen on d1 satisfies the horizontal mirror branch of `northeast 1 up 1 Q`!
    assert!(crate::search::evaluator::matches_single_ply(
        &q_flip_q, &pos_d1, 0, None
    ));

    // 3. Flipped directional shift with concrete square: `flip (northeast 1 up 1 e1 & occupied)`
    let q_orig = QueryParser::parse_str("northeast 1 up 1 e1 & occupied").unwrap();
    let q_flip_sq = QueryParser::parse_str("flip (northeast 1 up 1 e1 & occupied)").unwrap();
    let q_flip_all = QueryParser::parse_str("flip:all (northeast 1 up 1 e1 & occupied)").unwrap();

    // F3 occupied with Queen on f3 matches original branch
    let fen_f3_occ: Fen = "1k6/8/8/8/8/5Q2/8/6K1 w - - 0 1".parse().unwrap(); // Queen on f3 (f3 = northeast 1 up 1 e1)
    let pos_f3: Chess = fen_f3_occ.into_position(CastlingMode::Chess960).unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_orig, &pos_f3, 0, None
    ));
    assert!(crate::search::evaluator::matches_single_ply(
        &q_flip_all,
        &pos_f3,
        0,
        None
    ));

    // C3 occupied with Queen on c3 matches flipped branch (c3 = northwest 1 up 1 d1)
    let fen_c3_occ: Fen = "1k6/8/8/8/8/2Q5/8/6K1 w - - 0 1".parse().unwrap();
    let pos_c3_occ: Chess = fen_c3_occ.into_position(CastlingMode::Chess960).unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_flip_sq,
        &pos_c3_occ,
        0,
        None
    ));
    assert!(crate::search::evaluator::matches_single_ply(
        &q_flip_all,
        &pos_c3_occ,
        0,
        None
    ));

    // 4. Static shift constant folding: `up 1 right 2 e3` resolves to `g4`
    let exp_static = QueryParser::explain("up 1 right 2 e3").unwrap();
    assert_eq!(exp_static.canonical_dsl, "g4");
}

#[test]
fn test_offset_function_syntax_and_evaluation() {
    use shakmaty::fen::Fen;
    use shakmaty::{CastlingMode, Chess};

    // 1. Static square offset: `offset(e3, 2, 1)` -> g4
    let exp = QueryParser::explain("offset(e3, 2, 1)").unwrap();
    assert_eq!(exp.canonical_dsl, "g4");

    let exp_neg = QueryParser::explain("offset(e3, -2, -1)").unwrap();
    assert_eq!(exp_neg.canonical_dsl, "c2");

    // 2. Dynamic piece offset: `offset(N, 2, 1)` on Knight on c3 (c3 + (2,1) -> e4)
    let q_offset_n = QueryParser::parse_str("offset(N, 2, 1)").unwrap();
    let fen_c3: Fen = "8/8/8/8/8/2N5/8/4K2k w - - 0 1".parse().unwrap();
    let pos_c3: Chess = fen_c3.into_position(CastlingMode::Chess960).unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_offset_n,
        &pos_c3,
        0,
        None
    ));

    let q_match_e4 = QueryParser::parse_str("offset(N, 2, 1) & e4").unwrap();
    let q_match_d4 = QueryParser::parse_str("offset(N, 2, 1) & d4").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_match_e4,
        &pos_c3,
        0,
        None
    ));
    assert!(!crate::search::evaluator::matches_single_ply(
        &q_match_d4,
        &pos_c3,
        0,
        None
    ));

    // 3. Offset with King: `offset(k, -1, 0)` on Black King on e8 -> d8
    let q_offset_k = QueryParser::parse_str("offset(k, -1, 0) & d8").unwrap();
    let fen_e8: Fen = "4k3/8/8/8/8/8/8/4K3 w - - 0 1".parse().unwrap();
    let pos_e8: Chess = fen_e8.into_position(CastlingMode::Chess960).unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_offset_k,
        &pos_e8,
        0,
        None
    ));

    // 4. Offset with Symmetries & Flips:
    // `flip (offset(N, 2, 1) & h4)`
    // - Original: (offset (+2, +1) from N) & h4. (Knight on f3 gives h4)
    // - Flipped: (offset (-2, +1) from N) & a4. (Knight on c3 gives a4)
    let q_flip_offset_n = QueryParser::parse_str("flip (offset(N, 2, 1) & h4)").unwrap();
    let fen_c3_knight: Fen = "8/8/8/8/8/2N5/8/4K2k w - - 0 1".parse().unwrap();
    let pos_c3_knight: Chess = fen_c3_knight.into_position(CastlingMode::Chess960).unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_flip_offset_n,
        &pos_c3_knight,
        0,
        None
    ));

    // 5. Static offset under rotation: `rotate90 offset(e3, 2, 1)`
    // e3 + (2,1) = g4 (f=6, r=3)
    // rotate90 turns (f=6, r=3) -> (f=3, r=1) = d2
    let exp_rot = QueryParser::explain("rotate90 offset(e3, 2, 1)").unwrap();
    assert_eq!(exp_rot.branches.len(), 4);
    assert_eq!(exp_rot.branches[0].dsl, "g4"); // 0 deg
    assert_eq!(exp_rot.branches[1].dsl, "d2"); // 90 deg
    assert_eq!(exp_rot.branches[2].dsl, "b5"); // 180 deg
    assert_eq!(exp_rot.branches[3].dsl, "e7"); // 270 deg
}
