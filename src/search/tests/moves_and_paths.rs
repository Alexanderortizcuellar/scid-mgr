use super::fixtures::*;
use crate::search::*;
use shakmaty::Color;

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

    // 11. move en_passant / move ep
    let q11_ep = QueryParser::parse_str("move en_passant").unwrap();
    let res_ep = GameSearchEvaluator::evaluate_pgn(&q11_ep, game_pgn);
    assert!(res_ep.is_match, "Should match 3. exd6 (en passant)");
    assert_eq!(res_ep.matching_plies, vec![5]);

    let q11_ep_short = QueryParser::parse_str("move ep").unwrap();
    let res_ep_short = GameSearchEvaluator::evaluate_pgn(&q11_ep_short, game_pgn);
    assert!(res_ep_short.is_match);
    assert_eq!(res_ep_short.matching_plies, vec![5]);

    // Opera game has no en passant moves:
    let res_ep_opera = GameSearchEvaluator::evaluate_pgn(&q11_ep, OPERA_GAME);
    assert!(
        !res_ep_opera.is_match,
        "Opera game does not have en passant moves"
    );
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
fn test_previous_move_queries() {
    // Morphy's Opera Game:
    // 1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7
    // 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8
    // 13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0
    let opera_pgn = r#"[Event "Paris"]
[Site "Paris FRA"]
[Date "1858.??.??"]
[White "Paul Morphy"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0
"#;

    // 1. Check positions reached immediately after castling (12. O-O-O)
    let q_prev_castle = QueryParser::parse_str("move previous castle").unwrap();
    let res_prev_castle = GameSearchEvaluator::evaluate_pgn(&q_prev_castle, opera_pgn);
    assert!(res_prev_castle.is_match);
    // 12. O-O-O is white move 12 (ply 23)
    assert!(res_prev_castle.matching_plies.contains(&23));

    // 2. Syntax variations: "previous castle", "prev castle"
    let q_prev_castle_syntax = QueryParser::parse_str("previous castle").unwrap();
    let res_syntax = GameSearchEvaluator::evaluate_pgn(&q_prev_castle_syntax, opera_pgn);
    assert!(res_syntax.is_match);
    assert_eq!(res_syntax.matching_plies, res_prev_castle.matching_plies);

    // 3. Combined query: "check and move previous from B" (e.g. 11. Bxb5+ or 15. Bxd7+)
    // 11. Bxb5+ plays Bishop from c4 (ply 21), giving check!
    let q_check_prev_b = QueryParser::parse_str("check and move previous piece B").unwrap();
    let res_check_b = GameSearchEvaluator::evaluate_pgn(&q_check_prev_b, opera_pgn);
    assert!(res_check_b.is_match);
    assert!(res_check_b.matching_plies.contains(&21)); // After 11. Bxb5+

    // 4. Combined query: "check and move previous from [c4 d7]"
    let q_check_from_c4 = QueryParser::parse_str("check and move previous from [c4 d7]").unwrap();
    let res_check_from_c4 = GameSearchEvaluator::evaluate_pgn(&q_check_from_c4, opera_pgn);
    assert!(res_check_from_c4.is_match);
    assert!(res_check_from_c4.matching_plies.contains(&21));

    // 5. "not check and move previous castle"
    let q_not_check_prev_castle =
        QueryParser::parse_str("not check and move previous castle").unwrap();
    let res_not_chk_cst = GameSearchEvaluator::evaluate_pgn(&q_not_check_prev_castle, opera_pgn);
    assert!(res_not_chk_cst.is_match);
    assert!(res_not_chk_cst.matching_plies.contains(&23));

    // 6. Checkmate move delivered: 17. Rd8# (ply 33)
    let q_prev_mate = QueryParser::parse_str("move previous mate").unwrap();
    let res_prev_mate = GameSearchEvaluator::evaluate_pgn(&q_prev_mate, opera_pgn);
    assert!(res_prev_mate.is_match);
    assert_eq!(res_prev_mate.matching_plies, vec![33]);

    // 7. Explain / ToDsl roundtrip formatting
    let dsl = q_check_prev_b.to_dsl();
    assert!(dsl.contains("previous") && dsl.contains("B"));
}

#[test]
fn test_direction_filters_and_move_paths() {
    use shakmaty::Square;

    // 1. Basic & Compound Direction square set expansions
    let q_up1 = QueryParser::parse_str("piece P on up 1 d4").unwrap();
    if let SearchQuery::Position(PositionPattern::Squares(map)) = q_up1 {
        assert_eq!(map.keys().copied().collect::<Vec<_>>(), vec![Square::D5]);
    } else {
        panic!("Expected PositionPattern::Squares");
    }

    let q_up_range = QueryParser::parse_str("piece P on up 1 3 d4").unwrap();
    if let SearchQuery::Position(PositionPattern::MultiSquare { squares: sqs, .. }) = q_up_range {
        assert_eq!(sqs, vec![Square::D5, Square::D6, Square::D7]);
    } else {
        panic!("Expected PositionPattern::MultiSquare");
    }

    let q_down2 = QueryParser::parse_str("piece P on down 2 d4").unwrap();
    if let SearchQuery::Position(PositionPattern::Squares(map)) = q_down2 {
        assert_eq!(map.keys().copied().collect::<Vec<_>>(), vec![Square::D2]);
    } else {
        panic!("Expected PositionPattern::Squares");
    }

    let q_left1 = QueryParser::parse_str("piece P on left 1 [d4, e5]").unwrap();
    if let SearchQuery::Position(PositionPattern::MultiSquare { squares: sqs, .. }) = q_left1 {
        assert_eq!(sqs, vec![Square::C4, Square::D5]);
    } else {
        panic!("Expected PositionPattern::MultiSquare");
    }

    let q_diag1 = QueryParser::parse_str("piece P on diagonal 1 d4").unwrap();
    if let SearchQuery::Position(PositionPattern::MultiSquare { squares: sqs, .. }) = q_diag1 {
        assert_eq!(sqs, vec![Square::C3, Square::E3, Square::C5, Square::E5]);
    } else {
        panic!("Expected PositionPattern::MultiSquare");
    }

    // 2. Composed directions: up 2 right 1 d4 -> e6 (knight hop)
    let q_composed = QueryParser::parse_str("piece P on up 2 right 1 d4").unwrap();
    if let SearchQuery::Position(PositionPattern::Squares(map)) = q_composed {
        assert_eq!(map.keys().copied().collect::<Vec<_>>(), vec![Square::E6]);
    } else {
        panic!("Expected PositionPattern::Squares");
    }

    // 3. ray(up, d4) syntax
    let q_ray = QueryParser::parse_str("piece P on ray(up, d4)").unwrap();
    if let SearchQuery::Position(PositionPattern::MultiSquare { squares: sqs, .. }) = q_ray {
        assert_eq!(sqs, vec![Square::D5, Square::D6, Square::D7, Square::D8]);
    } else {
        panic!("Expected PositionPattern::MultiSquare");
    }

    // 4. Directional moves on Opera game:
    // 1. e4 (P--up 2) e5 2. Nf3 d6 3. d4 Bg4 ...
    let opera_pgn = r#"[Event "Paris"]
[Site "Paris FRA"]
[Date "1858.??.??"]
[White "Paul Morphy"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0
"#;

    // A. 1. e4 is a 2-step pawn advance up: path [P--up 2]
    let q_pawn_up2 = QueryParser::parse_str("path [P--up 2]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_pawn_up2, opera_pgn).is_match);

    // B. Standalone move filter: move piece P to up 2
    let q_move_up2 = QueryParser::parse_str("move piece P to up 2").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_move_up2, opera_pgn).is_match);

    // C. Queen move up in Opera game: 7. Qb3 (from f3 to b3 is horizontal, 5. Qxf3 is diagonal)
    let q_queen_horiz = QueryParser::parse_str("move piece Q horizontal").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_queen_horiz, opera_pgn).is_match);

    let q_queen_diag = QueryParser::parse_str("move piece Q diagonal").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_queen_diag, opera_pgn).is_match);

    // D. Rook moving down the board (e.g. 17. Rd8# is down to 8th rank from d1)
    let q_rook_up = QueryParser::parse_str("move piece R up").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_rook_up, opera_pgn).is_match);

    // E. Path sequence with directional move tokens
    let q_path_dirs = QueryParser::parse_str("path [P--up 2, ..., P--up 2]").unwrap();
    // 1. e4 (P--up 2), gap, 3. d4 (P--up 2)
    assert!(GameSearchEvaluator::evaluate_pgn(&q_path_dirs, opera_pgn).is_match);
}

#[test]
fn test_single_color_path_and_move_repetition_quantifiers() {
    let opera_pgn = r#"[Event "Paris"]
[Site "Paris FRA"]
[Date "1858.??.??"]
[White "Paul Morphy"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0
"#;

    // 1. Single color path: Morphy's sequence 1. e4, 2. Nf3, 3. d4 without writing opponent moves
    let q_single = QueryParser::parse_str("path singlecolor [e4, Nf3, d4]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_single, opera_pgn).is_match);

    // 2. White explicit single color path
    let q_white = QueryParser::parse_str("path white [e4, Nf3, d4]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_white, opera_pgn).is_match);

    // 3. Black explicit single color path: 1... e5 2... d6 3... bg4
    let q_black = QueryParser::parse_str("path black [e5, d6, bg4]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_black, opera_pgn).is_match);

    // 4. Single-color queen maneuvers: 5. Qxf3, 7. Qb3
    // Morphy's Queen moves from f3 to b3 (with one white move Bc4 in between)
    let q_queen_tour = QueryParser::parse_str("path white [Qxf3, ..., Qb3]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_queen_tour, opera_pgn).is_match);

    // 5. Repetition quantifiers: e.g. piece moves with bounds `{1, 5}`
    // White plays pawn moves: 1. e4, 3. d4 -> 2 pawn moves in first 3 turns
    let q_rep = QueryParser::parse_str("path white [P--*{1, 3}]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_rep, opera_pgn).is_match);

    // 6. Token repetition syntax in path: e.g. `Q--b3{1}`
    let q_token_rep = QueryParser::parse_str("path [Qb3{1}]").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_token_rep, opera_pgn).is_match);

    // 7. DSL explain roundtrip check
    let explained = crate::search::explain_query("path white [e4, Nf3, d4]", &q_white);
    assert!(explained.canonical_dsl.contains("white"));
}

#[test]
fn test_cql_path_specification_and_interleaved_filters() {
    let opera_pgn = r#"[Event "Paris"]
[Site "Paris"]
[Date "1858.??.??"]
[Round "?"]
[White "Morphy, Paul"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7
8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7
14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0"#;

    // 1. Opera Game mating finish with SAN suffixes (+, #)
    let q_mate_finish = QueryParser::parse_str("cqlpath { Qb8+ Nxb8 Rd8# }").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_mate_finish, opera_pgn).is_match);

    // 2. Opera Game mating finish with explicit keyword filters (check, mate)
    let q_keyword_finish = QueryParser::parse_str("cqlpath { Qb8 check Nxb8 Rd8 mate }").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_keyword_finish, opera_pgn).is_match);

    // 3. Opening non-check moves assertion
    let q_opening_not_check = QueryParser::parse_str("cqlpath { e4 not check e5 }").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_opening_not_check, opera_pgn).is_match);

    // 4. Repeated check chain with turnstile alias
    let q_repeated = QueryParser::parse_str("turnstile { (e4 e5)+ }").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_repeated, opera_pgn).is_match);

    // 5. Interleaved general filter block in sequence
    let q_filter_block =
        QueryParser::parse_str("sequence { e4 { [p] on e7 } e5 { [p] on e5 } }").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_filter_block, opera_pgn).is_match);
}

#[test]
fn test_cql_line_specification_forward_backward_and_modifiers() {
    let opera_pgn = r#"[Event "Paris"]
[Site "Paris"]
[Date "1858.??.??"]
[Round "?"]
[White "Morphy, Paul"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7
8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7
14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0"#;

    // 1. Canonical CQLi Forward line: check -> move previous capture -> mate
    let q_forward_line =
        QueryParser::parse_str("line --> check --> move previous capture --> mate").unwrap();
    let res_forward = GameSearchEvaluator::evaluate_pgn(&q_forward_line, opera_pgn);
    assert!(
        res_forward.is_match,
        "Forward line with check -> capture -> mate must match Opera Game"
    );
    assert!(res_forward.matching_plies.contains(&31));

    // 2. Canonical CQLi Grouping chain with quantifier: ( check --> move previous capture )+
    let q_group = QueryParser::parse_str("line --> ( check --> move previous capture )+").unwrap();
    let res_group = GameSearchEvaluator::evaluate_pgn(&q_group, opera_pgn);
    assert!(
        res_group.is_match,
        "Parenthesized chain group with + must match check -> capture sequence"
    );

    // 3. Move token transition in line: line --> Qb8+ --> Nxb8 --> Rd8#
    let q_moves_line = QueryParser::parse_str("line --> Qb8+ --> Nxb8 --> Rd8#").unwrap();
    let res_moves = GameSearchEvaluator::evaluate_pgn(&q_moves_line, opera_pgn);
    assert!(
        res_moves.is_match,
        "Line with move tokens must match sequence"
    );

    // 4. Backward look-behind with lastposition modifier: mate and line lastposition <-- check*
    let q_look_behind = QueryParser::parse_str("mate and line lastposition <-- check*").unwrap();
    let res_look_behind = GameSearchEvaluator::evaluate_pgn(&q_look_behind, opera_pgn);
    assert!(
        res_look_behind.is_match,
        "Mate with backward check look-behind must match"
    );

    // 5. Length restrictions and nestban deduplication: line 1 10 nestban --> check+
    let q_check_streak = QueryParser::parse_str("line 1 10 nestban --> check+").unwrap();
    let res_streak = GameSearchEvaluator::evaluate_pgn(&q_check_streak, opera_pgn);
    assert!(
        res_streak.is_match,
        "Check streak with range and nestban must match"
    );

    // 6. Singlecolor modifier
    let q_single_color = QueryParser::parse_str("line singlecolor --> check+").unwrap();
    let res_single_color = GameSearchEvaluator::evaluate_pgn(&q_single_color, opera_pgn);
    assert!(res_single_color.is_match, "Single color line must match");

    // 7. Parse error on mixed directional arrows
    let mixed_err = QueryParser::parse_str("line --> check <-- mate").unwrap_err();
    assert!(mixed_err.message.contains("Mixing '-->' and '<--'"));

    // 8. Backward compatibility check: legacy SCID line [e4 e5 Nf3 d6]
    let q_legacy = QueryParser::parse_str("line [e4 e5 Nf3 d6]").unwrap();
    let res_legacy = GameSearchEvaluator::evaluate_pgn(&q_legacy, opera_pgn);
    assert!(
        res_legacy.is_match,
        "Legacy SCID bracketed line must still match consecutively"
    );

    // 9. Backward compatibility check: path [e4 ... Rd8#]
    let q_path = QueryParser::parse_str("path [e4 ... Rd8#]").unwrap();
    let res_path = GameSearchEvaluator::evaluate_pgn(&q_path, opera_pgn);
    assert!(
        res_path.is_match,
        "SCID path must continue to work unchanged"
    );

    // 10. Backward compatibility check: cqlpath { Qb8+ Nxb8 Rd8# }
    let q_cqlpath = QueryParser::parse_str("cqlpath { Qb8+ Nxb8 Rd8# }").unwrap();
    let res_cqlpath = GameSearchEvaluator::evaluate_pgn(&q_cqlpath, opera_pgn);
    assert!(
        res_cqlpath.is_match,
        "CQL 6.2 cqlpath must continue to work unchanged"
    );
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
fn test_move_capture_parameter_logic() {
    use crate::search::evaluator::GameSearchEvaluator;
    use crate::search::query::*;
    use shakmaty::fen::Fen;
    use shakmaty::{CastlingMode, Chess};

    // 1. Test parsing of various capture parameter syntaxes
    let q1 = QueryParser::parse_str("move from R to d5 capture p").unwrap();
    if let SearchQuery::Move(ref pat) = q1 {
        assert_eq!(pat.is_capture, Some(true));
        assert!(pat.captured_pieces.is_some());
        assert_eq!(
            pat.captured_pieces.as_ref().unwrap(),
            &vec![SquareContent::Piece(shakmaty::Piece {
                color: Color::Black,
                role: shakmaty::Role::Pawn,
            })]
        );
    } else {
        panic!("Expected Move query");
    }

    let q2 = QueryParser::parse_str("legal capture Q count >= 1").unwrap();
    if let SearchQuery::Move(ref pat) = q2 {
        assert!(pat.is_legal);
        assert_eq!(pat.is_capture, Some(true));
        assert_eq!(
            pat.captured_pieces.as_ref().unwrap(),
            &vec![SquareContent::Piece(shakmaty::Piece {
                color: Color::White,
                role: shakmaty::Role::Queen,
            })]
        );
        assert_eq!(
            pat.count_predicate,
            Some((ComparisonOp::GreaterThanOrEqual, 1))
        );
    } else {
        panic!("Expected Move query");
    }

    let q3 = QueryParser::parse_str("move capture [n, b]").unwrap();
    if let SearchQuery::Move(ref pat) = q3 {
        assert_eq!(pat.is_capture, Some(true));
        let caps = pat.captured_pieces.as_ref().unwrap();
        assert_eq!(caps.len(), 2);
    } else {
        panic!("Expected Move query");
    }

    // 2. Test evaluation against concrete PGN game (Opera Game)
    // In Morphy's Opera Game:
    // Move 4: dxe5 (White pawn captures Black pawn)
    // Move 4... Bxf3 (Black bishop captures White knight)
    // Move 5: Qxf3 (White queen captures Black bishop)
    // Move 10: Nxb5 (White knight captures Black pawn)
    // Move 13: Rxd7 (White rook captures Black knight on d7)
    // Move 15: Bxd7+ (White bishop captures Black knight on d7)
    // Move 16: Qb8+ Nxb8 (Black knight captures White Queen)
    // Move 17: Rd8#

    // Morphy's Queen was captured by Black Knight on b8
    let q_cap_queen = QueryParser::parse_str("move capture Q").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_cap_queen, OPERA_GAME).is_match);

    let q_black_cap_queen = QueryParser::parse_str("move black capture Q").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_black_cap_queen, OPERA_GAME).is_match);

    let q_white_cap_queen = QueryParser::parse_str("move white capture q").unwrap();
    // In Opera Game, Black Queen was never captured!
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_white_cap_queen, OPERA_GAME).is_match);

    // Morphy captured Black Bishop with Queen: `move from Q capture b`
    let q_queen_cap_bishop = QueryParser::parse_str("move from Q capture b").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_queen_cap_bishop, OPERA_GAME).is_match);

    // 3. Test en passant capture matching
    let ep_pgn = r#"[Event "EP Test"]
[Site "?"]
[Date "2024.01.01"]
[Round "?"]
[White "P1"]
[Black "P2"]
[Result "*"]

1. e4 Nf6 2. e5 d5 3. exd6 *
"#;
    let q_ep_cap_p = QueryParser::parse_str("move en_passant capture p").unwrap();
    assert!(GameSearchEvaluator::evaluate_pgn(&q_ep_cap_p, ep_pgn).is_match);

    let q_ep_cap_r = QueryParser::parse_str("move en_passant capture r").unwrap();
    assert!(!GameSearchEvaluator::evaluate_pgn(&q_ep_cap_r, ep_pgn).is_match);

    // 4. Test legal move capture counts in a static position
    // White: Queen on d1, Rook on d4, King on e1. Black: Pawn on d5, Knight on e4, King on e6.
    let pos_fen = "8/8/4k3/3p4/3Rn3/8/8/3QK3 w - - 0 1";
    let fen: Fen = pos_fen.parse().unwrap();
    let pos: Chess = fen.into_position(CastlingMode::Chess960).unwrap();

    let q_legal_cap_p = QueryParser::parse_str("legal capture p count == 1").unwrap(); // Rxd5
    assert!(crate::search::evaluator::matches_single_ply(
        &q_legal_cap_p,
        &pos,
        0,
        None
    ));

    let q_legal_cap_n = QueryParser::parse_str("legal capture n count == 1").unwrap(); // Rxe4
    assert!(crate::search::evaluator::matches_single_ply(
        &q_legal_cap_n,
        &pos,
        0,
        None
    ));

    let q_legal_cap_b = QueryParser::parse_str("legal capture b count == 0").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_legal_cap_b,
        &pos,
        0,
        None
    ));

    // 5. Test flipcolor transformation with capture parameter
    let q_flip = QueryParser::parse_str("flipcolor { move capture p }").unwrap();
    // Under flipcolor, `capture p` (black pawn) becomes `capture P` (white pawn)
    // In Kasparov - Topalov, Kasparov's white pawns were captured by Black
    assert!(GameSearchEvaluator::evaluate_pgn(&q_flip, KASPAROV_TOPALOV).is_match);

    // 6. Test explain / round-trip
    let dsl_str = q1.to_dsl();
    assert!(dsl_str.contains("capture p"));
}

#[test]
fn test_move_promotion_keyword_boundary_and_roles() {
    // 1. Bare promote followed by 'and mate' should default to any promotion and preserve 'and mate'
    let q_bare_and = QueryParser::parse_str("move previous from P promote and mate").unwrap();
    match &q_bare_and {
        SearchQuery::And(children) => {
            assert_eq!(children.len(), 2);
            if let SearchQuery::Move(ref m) = children[0] {
                assert!(m.is_previous);
                assert_eq!(
                    m.promotions,
                    Some(vec![
                        shakmaty::Role::Queen,
                        shakmaty::Role::Rook,
                        shakmaty::Role::Bishop,
                        shakmaty::Role::Knight,
                    ])
                );
            } else {
                panic!("Expected first child to be Move");
            }
            if let SearchQuery::Position(PositionPattern::BoardState { is_checkmate, .. }) =
                children[1]
            {
                assert_eq!(is_checkmate, Some(true));
            } else {
                panic!("Expected second child to be mate PositionPattern");
            }
        }
        SearchQuery::Move(ref m) => {
            assert!(m.is_previous);
            assert_eq!(m.is_checkmate, Some(true));
            assert_eq!(
                m.promotions,
                Some(vec![
                    shakmaty::Role::Queen,
                    shakmaty::Role::Rook,
                    shakmaty::Role::Bishop,
                    shakmaty::Role::Knight,
                ])
            );
        }
        other => panic!("Unexpected query shape: {:?}", other),
    }

    // 2. Bare promote followed directly by 'mate' inside move filter
    let q_bare_mate = QueryParser::parse_str("move previous from P promote mate").unwrap();
    if let SearchQuery::Move(ref m) = q_bare_mate {
        assert!(m.is_previous);
        assert_eq!(m.is_checkmate, Some(true));
        assert_eq!(
            m.promotions,
            Some(vec![
                shakmaty::Role::Queen,
                shakmaty::Role::Rook,
                shakmaty::Role::Bishop,
                shakmaty::Role::Knight,
            ])
        );
    } else {
        panic!("Expected single Move query");
    }

    // 3. Specific promotion role Q
    let q_q = QueryParser::parse_str("move previous from P promote Q and mate").unwrap();
    match &q_q {
        SearchQuery::And(children) => {
            if let SearchQuery::Move(ref m) = children[0] {
                assert_eq!(m.promotion, Some(shakmaty::Role::Queen));
            } else {
                panic!("Expected Move child");
            }
        }
        other => panic!("Unexpected query: {:?}", other),
    }

    // 4. Bracketed roles [Q, R]
    let q_qr = QueryParser::parse_str("move previous from P promote [Q, R] and mate").unwrap();
    match &q_qr {
        SearchQuery::And(children) => {
            if let SearchQuery::Move(ref m) = children[0] {
                assert_eq!(
                    m.promotions,
                    Some(vec![shakmaty::Role::Queen, shakmaty::Role::Rook])
                );
            } else {
                panic!("Expected Move child");
            }
        }
        other => panic!("Unexpected query: {:?}", other),
    }

    // 5. String role list "rbn"
    let q_rbn = QueryParser::parse_str("move previous from P promote \"rbn\" and mate").unwrap();
    match &q_rbn {
        SearchQuery::And(children) => {
            if let SearchQuery::Move(ref m) = children[0] {
                assert_eq!(
                    m.promotions,
                    Some(vec![
                        shakmaty::Role::Rook,
                        shakmaty::Role::Bishop,
                        shakmaty::Role::Knight,
                    ])
                );
            } else {
                panic!("Expected Move child");
            }
        }
        other => panic!("Unexpected query: {:?}", other),
    }

    // 6. Invalid promotion roles should return ParseError
    assert!(QueryParser::parse_str("move previous from P promote A and mate").is_err());
    assert!(QueryParser::parse_str("move previous from P promote a and mate").is_err());
    assert!(QueryParser::parse_str("move previous from P promote X").is_err());
    assert!(QueryParser::parse_str("move previous from P promote [X]").is_err());
}
