use super::fixtures::*;
use crate::search::*;
use shakmaty::{Color, Piece, Role, Square};
use std::collections::HashMap;

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
