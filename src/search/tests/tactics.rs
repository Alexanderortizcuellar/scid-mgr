use super::fixtures::*;
use crate::search::*;
use std::collections::HashMap;

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
fn test_mating_themes_catalog_parsing_and_evaluation() {
    let dsl_content = include_str!("../../../themes/mates.dsl");
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
fn test_legal_mate_queries() {
    // Opera Game final position before 17. Rd8#:
    // At ply 32 (after 16... Nxb8), White's turn with Rd8# as a legal mate move.
    let opera_pgn = r#"[Event "Paris"]
[Site "Paris FRA"]
[Date "1858.??.??"]
[White "Paul Morphy"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6 7. Qb3 Qe7 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8 13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0
"#;

    // 1. Legal mate count >= 1
    let q_legal_mate = QueryParser::parse_str("legal mate count >= 1").unwrap();
    let res1 = GameSearchEvaluator::evaluate_pgn(&q_legal_mate, opera_pgn);
    assert!(res1.is_match);
    // At ply 32 (White to move after 16... Nxb8), White has 17. Rd8# as legal checkmate
    assert_eq!(res1.matching_plies, vec![32]);

    // 2. Specific piece legal mate: "legal mate piece R"
    let q_rook_mate = QueryParser::parse_str("legal mate piece R count >= 1").unwrap();
    let res_rook = GameSearchEvaluator::evaluate_pgn(&q_rook_mate, opera_pgn);
    assert!(res_rook.is_match);
    assert_eq!(res_rook.matching_plies, vec![32]); // 17. Rd8#

    // 3. Knight mate should NOT match in Opera game
    let q_knight_mate = QueryParser::parse_str("legal mate piece N count >= 1").unwrap();
    let res_knight = GameSearchEvaluator::evaluate_pgn(&q_knight_mate, opera_pgn);
    assert!(!res_knight.is_match);
}

#[test]
fn test_play_and_leads_to_hypothetical_moves() {
    use shakmaty::fen::Fen;
    use shakmaty::{CastlingMode, Chess};

    // Classic underpromotion study:
    // White: King on g6, Pawn on h7, Pawn on f7.
    // Black: King on h8.
    // FEN: "7k/5P1P/6K1/8/8/8/8/8 w - - 0 1"
    // White to move:
    // - 1. f8=Q# -> Checkmate!
    // - 1. f8=R# -> Checkmate!
    // - 1. f8=B -> Stalemate!
    // - 1. f8=N -> Stalemate!
    let fen_str = "7k/5P1P/6K1/8/8/8/8/8 w - - 0 1";
    let fen: Fen = fen_str.parse().unwrap();
    let pos: Chess = fen.into_position(CastlingMode::Standard).unwrap();

    // 1. Check stalemate on B promotion using `leads_to`
    let q_b_stalemate = QueryParser::parse_str("legal promote B leads_to { stalemate }").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_b_stalemate,
        &pos,
        0,
        None
    ));

    // 2. Check stalemate on B promotion using `play`
    let q_b_play_stalemate = QueryParser::parse_str("play promote B { stalemate }").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_b_play_stalemate,
        &pos,
        0,
        None
    ));

    // 3. Queen promotion does NOT lead to stalemate
    let q_q_stalemate = QueryParser::parse_str("legal promote Q leads_to { stalemate }").unwrap();
    assert!(!crate::search::evaluator::matches_single_ply(
        &q_q_stalemate,
        &pos,
        0,
        None
    ));

    // 4. Queen promotion leads to mate
    let q_q_mate = QueryParser::parse_str("play legal promote Q { mate }").unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_q_mate, &pos, 0, None
    ));

    // 5. Bishop promotion does NOT lead to mate
    let q_b_mate = QueryParser::parse_str("play legal promote B { mate }").unwrap();
    assert!(!crate::search::evaluator::matches_single_ply(
        &q_b_mate, &pos, 0, None
    ));

    // 6. Both conditions combined:
    let q_combined = QueryParser::parse_str(
        "legal promote B leads_to { stalemate } and legal promote Q leads_to { mate }",
    )
    .unwrap();
    assert!(crate::search::evaluator::matches_single_ply(
        &q_combined,
        &pos,
        0,
        None
    ));

    // 7. Test in full game timeline (Fool's mate):
    // Position at ply 3 (2. g4) has legal Qh4 leading to mate:
    let pgn = r#"[Event "Fool's Mate"]
[Site "?"]
[Date "2024.01.01"]
[Round "1"]
[White "Player1"]
[Black "Player2"]
[Result "0-1"]

1. f3 e5 2. g4 Qh4# 0-1
"#;
    // At ply 3 (after 2. g4), Black has a legal move `Qh4` (or any legal move) that leads to checkmate:
    let q_play_mate = QueryParser::parse_str("play legal { mate }").unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&q_play_mate, pgn);
    assert!(res.is_match);
    assert_eq!(res.matching_plies, vec![3]);

    // Count exactly 1 legal move leading to mate at ply 3:
    let q_unique_mate = QueryParser::parse_str("legal count == 1 leads_to { mate }").unwrap();
    let res = GameSearchEvaluator::evaluate_pgn(&q_unique_mate, pgn);
    assert!(res.is_match);
    assert_eq!(res.matching_plies, vec![3]);
}

#[test]
fn test_play_and_not_move_missed_mate_regression() {
    let pgn = r#"[Event "Live Chess"]
[Site "Chess.com"]
[Date "2024.01.30"]
[Round "-"]
[White "frenllelzarraga"]
[Black "IamDiablo"]
[Result "1-0"]

1. c4 c5 2. Nc3 d6 3. e3 e5 4. d4 cxd4 5. exd4 exd4 6. Qxd4 Nc6 7. Qd1 Be6 8. a3 Qb6
9. Nd5 Bxd5 10. cxd5 O-O-O 11. dxc6 Qxc6 12. Be2 Re8 13. Nf3 Nf6 14. O-O g5 15. Nd4 Qe4
16. Bf3 Qh4 17. Qc2+ Kb8 18. Nb5 Rc8 19. Qd2 Ng4 20. Bxg4 Qxg4 21. Qxg5 Qc4 22. a4 a6
23. Nc3 h6 24. Qe3 h5 25. Rd1 Bh6 26. Qd3 Qg4 27. Qxd6+ Ka8 28. Bxh6 Rcd8 29. Qf4 Rxd1+
30. Rxd1 Qg6 31. Bg5 Rg8 32. h4 f6 33. Qxf6 Qc2 34. Rd8+ Rxd8 35. Qxd8+ Ka7 36. Be3+ b6
37. Qxb6+ Ka8 38. Qa7# 1-0"#;

    // 1. A legal Queen move delivers checkmate at ply 74 (Qa7#):
    let q_play_q_mate = QueryParser::parse_str("play from Q { mate }").unwrap();
    let res1 = GameSearchEvaluator::evaluate_pgn(&q_play_q_mate, pgn);
    assert!(res1.is_match);
    assert_eq!(res1.matching_plies, vec![74]);

    // 2. The move actually played at ply 74 WAS from Q (Qa7#), so `and move from Q` MUST match:
    let q_played_q = QueryParser::parse_str("play from Q { mate } and move from Q").unwrap();
    let res2 = GameSearchEvaluator::evaluate_pgn(&q_played_q, pgn);
    assert!(res2.is_match);
    assert_eq!(res2.matching_plies, vec![74]);

    // 3. Since Move 75 was from Q, `and not move from Q` MUST NOT match:
    let q_missed = QueryParser::parse_str("play from Q { mate } and not move from Q").unwrap();
    let res3 = GameSearchEvaluator::evaluate_pgn(&q_missed, pgn);
    assert!(
        !res3.is_match,
        "Should NOT match when the played move was indeed from Q"
    );
}
