#![allow(
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::single_match,
    clippy::type_complexity
)]

use crate::search::parser::lexer::ParseError;
use crate::search::query::{MovePattern, PositionPattern, SearchQuery, SquareContent};
use shakmaty::Color;

/// Validates an entire SearchQuery AST for logical contradictions and impossible conditions.
pub fn validate_query_semantics(query: &SearchQuery, pos: usize) -> Result<(), ParseError> {
    match query {
        SearchQuery::And(clauses) => {
            validate_and_clauses(clauses, pos)?;
            for clause in clauses {
                validate_query_semantics(clause, pos)?;
            }
        }
        SearchQuery::Or(clauses) => {
            for clause in clauses {
                validate_query_semantics(clause, pos)?;
            }
        }
        SearchQuery::Position(PositionPattern::Squares(map)) => {
            validate_square_map(map, pos)?;
        }
        SearchQuery::Position(PositionPattern::PieceCount {
            content,
            squares,
            op,
            count,
        }) => {
            validate_piece_count(content, squares.as_deref(), *op, *count, pos)?;
        }
        SearchQuery::SquareSet(crate::search::query::SetPredicate::CountComparison {
            expr: crate::search::query::SquareSetExpr::Piece(content),
            op,
            count,
        }) => {
            validate_piece_count(content, None, *op, *count, pos)?;
        }
        SearchQuery::Not(sub)
        | SearchQuery::Parent(sub)
        | SearchQuery::Child(sub)
        | SearchQuery::PlyRange { query: sub, .. }
        | SearchQuery::Occurrences { query: sub, .. }
        | SearchQuery::Play {
            outcome_query: sub, ..
        }
        | SearchQuery::WhatIf { query: sub, .. }
        | SearchQuery::VariableBinding { query: sub, .. }
        | SearchQuery::Symmetric { query: sub, .. }
        | SearchQuery::Shift { query: sub, .. }
        | SearchQuery::Initial(sub)
        | SearchQuery::Terminal(sub) => {
            validate_query_semantics(sub, pos)?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_square_map(
    map: &std::collections::HashMap<shakmaty::Square, SquareContent>,
    pos: usize,
) -> Result<(), ParseError> {
    let mut white_king_sq: Option<shakmaty::Square> = None;
    let mut black_king_sq: Option<shakmaty::Square> = None;

    for (&sq, content) in map {
        if let SquareContent::Piece(piece) = content {
            if piece.role == shakmaty::Role::Pawn {
                if sq.rank() == shakmaty::Rank::First || sq.rank() == shakmaty::Rank::Eighth {
                    let color_name = match piece.color {
                        Color::White => "White",
                        Color::Black => "Black",
                    };
                    return Err(ParseError::new(
                        format!(
                            "Impossible position: {} pawn cannot exist on rank {}",
                            color_name,
                            (sq.rank() as u8) + 1
                        ),
                        pos,
                    ).with_help("Pawns can never occupy the 1st or 8th rank in standard chess (they promote upon reaching the 8th/1st rank)."));
                }
            }

            if piece.role == shakmaty::Role::King {
                match piece.color {
                    Color::White => white_king_sq = Some(sq),
                    Color::Black => black_king_sq = Some(sq),
                }
            }
        }
    }

    if let (Some(w_sq), Some(b_sq)) = (white_king_sq, black_king_sq) {
        let f_diff = (w_sq.file() as i32 - b_sq.file() as i32).abs();
        let r_diff = (w_sq.rank() as i32 - b_sq.rank() as i32).abs();
        let chebyshev_dist = f_diff.max(r_diff);
        if chebyshev_dist <= 1 {
            return Err(ParseError::new(
                format!(
                    "Impossible position: White King on {} and Black King on {} are adjacent (distance = {})",
                    w_sq, b_sq, chebyshev_dist
                ),
                pos,
            ).with_help("In chess, opposing kings can never occupy adjacent squares (distance must be >= 2) because a king cannot move into check."));
        }
    }

    Ok(())
}

fn validate_and_clauses(clauses: &[SearchQuery], pos: usize) -> Result<(), ParseError> {
    let mut turn_constraint: Option<Color> = None;
    let mut check_constraint: Option<bool> = None;
    let mut is_mate = false;
    let mut is_stalemate = false;

    let mut move_num_constraint: Option<usize> = None;
    let mut ply_constraint: Option<usize> = None;
    let mut requires_opposite_bishops = false;
    let mut requires_same_bishops = false;

    // 1. Collect and check turns, board states, ply, move numbers, and material
    for clause in clauses {
        match clause {
            SearchQuery::Position(PositionPattern::Turn(c)) => {
                if let Some(existing) = turn_constraint {
                    if existing != *c {
                        return Err(ParseError::new(
                            format!(
                                "Contradictory turn assertions: '{:?}' and '{:?}' cannot both hold",
                                existing, c
                            ),
                            pos,
                        ).with_help("A position cannot be simultaneously White to move (wtm) and Black to move (btm)."));
                    }
                }
                turn_constraint = Some(*c);
            }
            SearchQuery::Position(PositionPattern::MoveNumber {
                op: crate::search::query::ComparisonOp::Equal,
                value,
            }) => {
                move_num_constraint = Some(*value);
            }
            SearchQuery::Position(PositionPattern::Ply {
                op: crate::search::query::ComparisonOp::Equal,
                value,
            }) => {
                ply_constraint = Some(*value);
            }
            SearchQuery::Material(m) => {
                if m.opposite_bishops == Some(true) {
                    requires_opposite_bishops = true;
                }
                if m.same_colored_bishops == Some(true) {
                    requires_same_bishops = true;
                }
            }
            SearchQuery::Position(PositionPattern::BoardState {
                is_check,
                is_checkmate,
                is_stalemate: is_stale,
            }) => {
                if let Some(chk) = is_check {
                    if let Some(existing_chk) = check_constraint {
                        if existing_chk != *chk {
                            return Err(ParseError::new(
                                "Contradictory conditions: 'check' and 'not check' cannot both hold".to_string(),
                                pos,
                            ).with_help("A position cannot be simultaneously in check and not in check."));
                        }
                    }
                    check_constraint = Some(*chk);
                }
                if *is_checkmate == Some(true) {
                    is_mate = true;
                }
                if *is_stale == Some(true) {
                    is_stalemate = true;
                }
            }
            SearchQuery::Not(box_sub) => match &**box_sub {
                SearchQuery::Position(PositionPattern::BoardState { is_check, .. }) => {
                    if *is_check == Some(true) {
                        if let Some(existing_chk) = check_constraint {
                            if existing_chk {
                                return Err(ParseError::new(
                                    "Contradictory conditions: 'check' and 'not check' cannot both hold".to_string(),
                                    pos,
                                ).with_help("A position cannot be simultaneously in check and not in check."));
                            }
                        }
                        check_constraint = Some(false);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    if requires_opposite_bishops && requires_same_bishops {
        return Err(ParseError::new(
            "Contradictory conditions: 'opposite_bishops' and 'same_colored_bishops' are mutually exclusive".to_string(),
            pos,
        ).with_help("Bishops cannot simultaneously be on opposite colored squares and the same colored squares."));
    }

    // 2. Cross-validate board states
    if is_mate && is_stalemate {
        return Err(ParseError::new(
            "Contradictory conditions: 'mate' and 'stalemate' are mutually exclusive".to_string(),
            pos,
        )
        .with_help("A position cannot be simultaneously checkmate and stalemate."));
    }

    if is_mate && check_constraint == Some(false) {
        return Err(ParseError::new(
            "Contradictory conditions: 'mate' requires 'check', but 'not check' was specified"
                .to_string(),
            pos,
        )
        .with_help(
            "Checkmate by definition requires the king to be in check with no legal evasions.",
        ));
    }

    if is_stalemate && check_constraint == Some(true) {
        return Err(ParseError::new(
            "Contradictory conditions: 'stalemate' requires that the king is NOT in check, but 'check' was specified".to_string(),
            pos,
        ).with_help("Stalemate by definition requires that the side to move has no legal moves and is NOT in check."));
    }

    // Cross-validate Move Number, Ply, and Turn
    if let Some(ply) = ply_constraint {
        if ply == 0 {
            return Err(ParseError::new(
                "Impossible condition: ply number must be >= 1 (1. e4 is ply 1)".to_string(),
                pos,
            )
            .with_help("Chess plys are 1-indexed (ply 1 = 1. e4, ply 2 = 1... e5, etc.)."));
        }

        if let Some(turn) = turn_constraint {
            match turn {
                Color::White => {
                    if ply % 2 == 0 {
                        return Err(ParseError::new(
                            format!(
                                "Contradictory turn and ply: ply {} is Black to move (even ply), but 'wtm' was asserted",
                                ply
                            ),
                            pos,
                        ).with_help(format!(
                            "Ply {} corresponds to Black's turn (even). White to move is odd (e.g. ply {}).",
                            ply,
                            ply.saturating_sub(1)
                        )));
                    }
                }
                Color::Black => {
                    if ply % 2 == 1 {
                        return Err(ParseError::new(
                            format!(
                                "Contradictory turn and ply: ply {} is White to move (odd ply), but 'btm' was asserted",
                                ply
                            ),
                            pos,
                        ).with_help(format!(
                            "Ply {} corresponds to White's turn (odd). Black to move is even (e.g. ply {}).",
                            ply,
                            ply + 1
                        )));
                    }
                }
            }
        }

        if let Some(mov_num) = move_num_constraint {
            if mov_num == 0 {
                return Err(ParseError::new(
                    "Impossible condition: move number must be >= 1".to_string(),
                    pos,
                )
                .with_help("Chess move numbers are 1-indexed (move 1, move 2, ...)."));
            }

            let w_ply = 2 * mov_num - 1;
            let b_ply = 2 * mov_num;

            if let Some(turn) = turn_constraint {
                let expected_ply = match turn {
                    Color::White => w_ply,
                    Color::Black => b_ply,
                };
                if ply != expected_ply {
                    let turn_str = match turn {
                        Color::White => "wtm",
                        Color::Black => "btm",
                    };
                    return Err(ParseError::new(
                        format!(
                            "Contradictory move number and ply: Move {} ({}) is ply {}, but ply was asserted as {}",
                            mov_num, turn_str, expected_ply, ply
                        ),
                        pos,
                    ).with_help(format!(
                        "In chess, Move {} ({}) is always Ply {}.",
                        mov_num, turn_str, expected_ply
                    )));
                }
            } else if ply != w_ply && ply != b_ply {
                return Err(ParseError::new(
                    format!(
                        "Contradictory move number and ply: Move {} corresponds to ply {} (wtm) or ply {} (btm), but ply was asserted as {}",
                        mov_num, w_ply, b_ply, ply
                    ),
                    pos,
                ).with_help(format!(
                    "For move {}, valid ply numbers are only {} or {}.",
                    mov_num, w_ply, b_ply
                )));
            }
        }
    }

    // 3. Cross-validate moves against turn and check
    if check_constraint == Some(true) {
        for clause in clauses {
            if let SearchQuery::Move(ref m) = clause {
                if !m.is_previous && m.is_castle == Some(true) {
                    return Err(ParseError::new(
                        "Impossible move: castling is illegal while the King is currently in check".to_string(),
                        pos,
                    ).with_help("Under chess rules, a player cannot castle out of check. The check must be blocked, captured, or the King moved."));
                }
            }
        }
    }

    if let Some(turn) = turn_constraint {
        for clause in clauses {
            if let SearchQuery::Move(ref m) = clause {
                let move_color = get_move_color(m);
                if let Some(col) = move_color {
                    if m.is_previous {
                        // Previous move MUST have been played by the opponent
                        if col == turn {
                            let (turn_str, piece_col_str, expected_str) = match turn {
                                Color::White => {
                                    ("wtm (White to move)", "White", "Black (e.g. lowercase 'q')")
                                }
                                Color::Black => {
                                    ("btm (Black to move)", "Black", "White (e.g. uppercase 'Q')")
                                }
                            };
                            return Err(ParseError::new(
                                format!(
                                    "Impossible move: in a '{}' position, the previous move was played by {}, not {}",
                                    turn_str, expected_str, piece_col_str
                                ),
                                pos,
                            ).with_help(format!(
                                "Since it is now {}'s turn, the preceding move must have been made by {}. Check piece casing (uppercase = White, lowercase = Black).",
                                piece_col_str, expected_str
                            )));
                        }
                    } else {
                        // Current move MUST be played by the side to move
                        if col != turn {
                            let (turn_str, piece_col_str) = match turn {
                                Color::White => ("wtm (White to move)", "Black"),
                                Color::Black => ("btm (Black to move)", "White"),
                            };
                            return Err(ParseError::new(
                                format!(
                                    "Impossible move: {} cannot play a move when it is {}",
                                    piece_col_str, turn_str
                                ),
                                pos,
                            ).with_help("Only the side to move can make a move in the current position. Check piece casing."));
                        }
                    }
                }
            }
        }
    }

    // 4. Validate piece placements, square conflicts, pawn ranks, King-King adjacency, and direct-check turn contradictions
    let mut square_assignments: std::collections::HashMap<shakmaty::Square, SquareContent> =
        std::collections::HashMap::new();
    let mut white_king_sq: Option<shakmaty::Square> = None;
    let mut black_king_sq: Option<shakmaty::Square> = None;
    let mut pieces_list: Vec<(shakmaty::Square, shakmaty::Piece)> = Vec::new();
    let mut white_placed = PieceCountAccumulator::default();
    let mut black_placed = PieceCountAccumulator::default();

    for clause in clauses {
        if let SearchQuery::Position(PositionPattern::Squares(map)) = clause {
            for (&sq, content) in map {
                // Check conflicting assignment on the same square
                if let Some(existing) = square_assignments.get(&sq) {
                    if existing != content {
                        return Err(ParseError::new(
                            format!(
                                "Contradictory piece placement on square {}: cannot simultaneously require conflicting contents",
                                sq
                            ),
                            pos,
                        ).with_help("A single square cannot be occupied by two different pieces simultaneously."));
                    }
                }
                square_assignments.insert(sq, content.clone());

                if let SquareContent::Piece(piece) = content {
                    pieces_list.push((sq, *piece));

                    match piece.color {
                        Color::White => white_placed.add_piece_at_square(piece.role, sq),
                        Color::Black => black_placed.add_piece_at_square(piece.role, sq),
                    }

                    if piece.role == shakmaty::Role::Pawn {
                        if sq.rank() == shakmaty::Rank::First || sq.rank() == shakmaty::Rank::Eighth
                        {
                            let color_name = match piece.color {
                                Color::White => "White",
                                Color::Black => "Black",
                            };
                            return Err(ParseError::new(
                                format!(
                                    "Impossible position: {} pawn cannot exist on rank {}",
                                    color_name,
                                    (sq.rank() as u8) + 1
                                ),
                                pos,
                            ).with_help("Pawns can never occupy the 1st or 8th rank in standard chess (they promote upon reaching the 8th/1st rank)."));
                        }
                    }

                    if piece.role == shakmaty::Role::King {
                        match piece.color {
                            Color::White => {
                                if let Some(existing_w_king) = white_king_sq {
                                    if existing_w_king != sq {
                                        return Err(ParseError::new(
                                            format!(
                                                "Impossible position: multiple White Kings placed on {} and {}",
                                                existing_w_king, sq
                                            ),
                                            pos,
                                        ).with_help("A chess position can contain at most one King per side."));
                                    }
                                }
                                white_king_sq = Some(sq);
                            }
                            Color::Black => {
                                if let Some(existing_b_king) = black_king_sq {
                                    if existing_b_king != sq {
                                        return Err(ParseError::new(
                                            format!(
                                                "Impossible position: multiple Black Kings placed on {} and {}",
                                                existing_b_king, sq
                                            ),
                                            pos,
                                        ).with_help("A chess position can contain at most one King per side."));
                                    }
                                }
                                black_king_sq = Some(sq);
                            }
                        }
                    }
                }
            }
        }
    }

    // Castling rights vs piece placements
    for clause in clauses {
        if let SearchQuery::Position(PositionPattern::Castling {
            color,
            kingside,
            queenside,
        }) = clause
        {
            match color {
                Color::White => {
                    if *kingside == Some(true) || *queenside == Some(true) {
                        if let Some(w_k_sq) = white_king_sq {
                            if w_k_sq != shakmaty::Square::E1 {
                                return Err(ParseError::new(
                                    format!(
                                        "Impossible position: White castling rights require White King on e1, but King is placed on {}",
                                        w_k_sq
                                    ),
                                    pos,
                                ).with_help("Castling rights are permanently lost once the King moves from e1."));
                            }
                        }
                    }
                    if *kingside == Some(true) {
                        if let Some(content) = square_assignments.get(&shakmaty::Square::H1) {
                            if *content
                                != SquareContent::Piece(shakmaty::Piece {
                                    color: Color::White,
                                    role: shakmaty::Role::Rook,
                                })
                            {
                                return Err(ParseError::new(
                                    format!(
                                        "Impossible position: White kingside castling requires a White Rook on h1, but h1 has {:?}",
                                        content
                                    ),
                                    pos,
                                ).with_help("White kingside castling is impossible if the h1 rook has moved or square h1 is occupied by another piece."));
                            }
                        }
                    }
                    if *queenside == Some(true) {
                        if let Some(content) = square_assignments.get(&shakmaty::Square::A1) {
                            if *content
                                != SquareContent::Piece(shakmaty::Piece {
                                    color: Color::White,
                                    role: shakmaty::Role::Rook,
                                })
                            {
                                return Err(ParseError::new(
                                    format!(
                                        "Impossible position: White queenside castling requires a White Rook on a1, but a1 has {:?}",
                                        content
                                    ),
                                    pos,
                                ).with_help("White queenside castling is impossible if the a1 rook has moved or square a1 is occupied by another piece."));
                            }
                        }
                    }
                }
                Color::Black => {
                    if *kingside == Some(true) || *queenside == Some(true) {
                        if let Some(b_k_sq) = black_king_sq {
                            if b_k_sq != shakmaty::Square::E8 {
                                return Err(ParseError::new(
                                    format!(
                                        "Impossible position: Black castling rights require Black King on e8, but King is placed on {}",
                                        b_k_sq
                                    ),
                                    pos,
                                ).with_help("Castling rights are permanently lost once the King moves from e8."));
                            }
                        }
                    }
                    if *kingside == Some(true) {
                        if let Some(content) = square_assignments.get(&shakmaty::Square::H8) {
                            if *content
                                != SquareContent::Piece(shakmaty::Piece {
                                    color: Color::Black,
                                    role: shakmaty::Role::Rook,
                                })
                            {
                                return Err(ParseError::new(
                                    format!(
                                        "Impossible position: Black kingside castling requires a Black Rook on h8, but h8 has {:?}",
                                        content
                                    ),
                                    pos,
                                ).with_help("Black kingside castling is impossible if the h8 rook has moved or square h8 is occupied by another piece."));
                            }
                        }
                    }
                    if *queenside == Some(true) {
                        if let Some(content) = square_assignments.get(&shakmaty::Square::A8) {
                            if *content
                                != SquareContent::Piece(shakmaty::Piece {
                                    color: Color::Black,
                                    role: shakmaty::Role::Rook,
                                })
                            {
                                return Err(ParseError::new(
                                    format!(
                                        "Impossible position: Black queenside castling requires a Black Rook on a8, but a8 has {:?}",
                                        content
                                    ),
                                    pos,
                                ).with_help("Black queenside castling is impossible if the a8 rook has moved or square a8 is occupied by another piece."));
                            }
                        }
                    }
                }
            }
        }
    }

    // Validate placed piece limits and promotion counts
    white_placed.validate(Color::White, pos)?;
    black_placed.validate(Color::Black, pos)?;

    if white_placed.total_pieces() + black_placed.total_pieces() > 32 {
        return Err(ParseError::new(
            format!(
                "Impossible position: total pieces placed on board ({}) exceeds 32",
                white_placed.total_pieces() + black_placed.total_pieces()
            ),
            pos,
        ).with_help("A standard chess game begins with 32 pieces and pieces are only captured, never added."));
    }

    // Validate cross-clause PieceCount constraints (e.g. Q == 9 and P == 8)
    let mut white_counts = PieceCountAccumulator::default();
    let mut black_counts = PieceCountAccumulator::default();

    for clause in clauses {
        if let SearchQuery::SquareSet(crate::search::query::SetPredicate::CountComparison {
            expr: crate::search::query::SquareSetExpr::Piece(SquareContent::Piece(p)),
            op: crate::search::query::ComparisonOp::Equal,
            count,
        }) = clause
        {
            match p.color {
                Color::White => white_counts.set_count(p.role, *count),
                Color::Black => black_counts.set_count(p.role, *count),
            }
        }

        if let SearchQuery::Position(PositionPattern::PieceCount {
            content,
            squares,
            op: crate::search::query::ComparisonOp::Equal,
            count,
        }) = clause
        {
            if squares.is_none() {
                if let SquareContent::Piece(p) = content {
                    match p.color {
                        Color::White => white_counts.set_count(p.role, *count),
                        Color::Black => black_counts.set_count(p.role, *count),
                    }
                }
            } else if let Some(sq_list) = squares {
                // Check if squares specify light or dark squares
                let is_all_light = sq_list.iter().all(|s| s.is_light());
                let is_all_dark = sq_list.iter().all(|s| s.is_dark());

                if let SquareContent::Piece(p) = content {
                    if p.role == shakmaty::Role::Bishop {
                        if is_all_light {
                            match p.color {
                                Color::White => white_counts.light_bishops = *count,
                                Color::Black => black_counts.light_bishops = *count,
                            }
                        } else if is_all_dark {
                            match p.color {
                                Color::White => white_counts.dark_bishops = *count,
                                Color::Black => black_counts.dark_bishops = *count,
                            }
                        }
                    }
                }
            }
        }
    }

    white_counts.validate(Color::White, pos)?;
    black_counts.validate(Color::Black, pos)?;

    // Check White King and Black King proximity (cannot be adjacent)
    if let (Some(w_sq), Some(b_sq)) = (white_king_sq, black_king_sq) {
        let f_diff = (w_sq.file() as i32 - b_sq.file() as i32).abs();
        let r_diff = (w_sq.rank() as i32 - b_sq.rank() as i32).abs();
        let chebyshev_dist = f_diff.max(r_diff);
        if chebyshev_dist <= 1 {
            return Err(ParseError::new(
                format!(
                    "Impossible position: White King on {} and Black King on {} are adjacent (distance = {})",
                    w_sq, b_sq, chebyshev_dist
                ),
                pos,
            ).with_help("In chess, opposing kings can never occupy adjacent squares (distance must be >= 2) because a king cannot move into check."));
        }
    }

    // Check Triple Check (max 2 simultaneous direct attackers on King)
    let mut white_king_attackers = 0;
    let mut black_king_attackers = 0;

    for (atk_sq, piece) in &pieces_list {
        if let Some(w_sq) = white_king_sq {
            if piece.color == Color::Black && piece_directly_attacks(piece, *atk_sq, w_sq) {
                white_king_attackers += 1;
            }
        }
        if let Some(b_sq) = black_king_sq {
            if piece.color == Color::White && piece_directly_attacks(piece, *atk_sq, b_sq) {
                black_king_attackers += 1;
            }
        }
    }

    if white_king_attackers > 2 {
        return Err(ParseError::new(
            format!(
                "Impossible position: White King on {} is simultaneously attacked by {} pieces (Triple check is impossible)",
                white_king_sq.unwrap(),
                white_king_attackers
            ),
            pos,
        ).with_help("In chess, a king can be checked by at most 2 pieces simultaneously (Double Check via a direct move + discovered ray check)."));
    }

    if black_king_attackers > 2 {
        return Err(ParseError::new(
            format!(
                "Impossible position: Black King on {} is simultaneously attacked by {} pieces (Triple check is impossible)",
                black_king_sq.unwrap(),
                black_king_attackers
            ),
            pos,
        ).with_help("In chess, a king can be checked by at most 2 pieces simultaneously (Double Check via a direct move + discovered ray check)."));
    }

    // Check direct unblockable attacks on Kings against Turn and Check constraints
    for (atk_sq, piece) in &pieces_list {
        let target_king_sq = match piece.color {
            Color::White => black_king_sq,
            Color::Black => white_king_sq,
        };

        if let Some(king_sq) = target_king_sq {
            if piece_directly_attacks(piece, *atk_sq, king_sq) {
                let (def_king_str, atk_color_str) = match piece.color {
                    Color::White => ("Black King", "White"),
                    Color::Black => ("White King", "Black"),
                };

                // Turn contradiction: attacker cannot have the turn while defending king is in check
                if let Some(turn) = turn_constraint {
                    if turn == piece.color {
                        let turn_str = match turn {
                            Color::White => "wtm (White to move)",
                            Color::Black => "btm (Black to move)",
                        };
                        return Err(ParseError::new(
                            format!(
                                "Impossible position: {} on {} is directly attacked by {} {:?} on {}, but turn is '{}'",
                                def_king_str, king_sq, atk_color_str, piece.role, atk_sq, turn_str
                            ),
                            pos,
                        ).with_help(format!(
                            "When the {} is in check, it MUST be the defending side's turn to move. A position cannot have the defending king in check on the attacker's turn ({}). Check turn or piece positions.",
                            def_king_str, turn_str
                        )));
                    }
                }

                // Check constraint contradiction: cannot assert 'not check' when under direct unblockable check
                if check_constraint == Some(false) {
                    return Err(ParseError::new(
                        format!(
                            "Contradictory condition: {} on {} is in direct check from {} {:?} on {}, but 'not check' was asserted",
                            def_king_str, king_sq, atk_color_str, piece.role, atk_sq
                        ),
                        pos,
                    ).with_help("The piece is directly attacking the opposing King with no possible blocking squares, so the position is unavoidably in check."));
                }
            }
        }
    }

    Ok(())
}

#[derive(Default, Debug, Clone)]
struct PieceCountAccumulator {
    pawns: usize,
    knights: usize,
    bishops: usize,
    light_bishops: usize,
    dark_bishops: usize,
    rooks: usize,
    queens: usize,
    kings: usize,
}

impl PieceCountAccumulator {
    fn add_piece_at_square(&mut self, role: shakmaty::Role, sq: shakmaty::Square) {
        match role {
            shakmaty::Role::Pawn => self.pawns += 1,
            shakmaty::Role::Knight => self.knights += 1,
            shakmaty::Role::Bishop => {
                self.bishops += 1;
                if sq.is_light() {
                    self.light_bishops += 1;
                } else {
                    self.dark_bishops += 1;
                }
            }
            shakmaty::Role::Rook => self.rooks += 1,
            shakmaty::Role::Queen => self.queens += 1,
            shakmaty::Role::King => self.kings += 1,
        }
    }

    fn set_count(&mut self, role: shakmaty::Role, count: usize) {
        match role {
            shakmaty::Role::Pawn => self.pawns = count,
            shakmaty::Role::Knight => self.knights = count,
            shakmaty::Role::Bishop => self.bishops = count,
            shakmaty::Role::Rook => self.rooks = count,
            shakmaty::Role::Queen => self.queens = count,
            shakmaty::Role::King => self.kings = count,
        }
    }

    fn min_promotions_needed(&self) -> usize {
        let extra_queens = self.queens.saturating_sub(1);
        let extra_rooks = self.rooks.saturating_sub(2);
        let extra_knights = self.knights.saturating_sub(2);
        let extra_bishops = if self.light_bishops > 0 || self.dark_bishops > 0 {
            self.light_bishops.saturating_sub(1) + self.dark_bishops.saturating_sub(1)
        } else {
            self.bishops.saturating_sub(2)
        };
        extra_queens + extra_rooks + extra_bishops + extra_knights
    }

    fn total_pieces(&self) -> usize {
        self.pawns + self.knights + self.bishops + self.rooks + self.queens + self.kings
    }

    fn validate(&self, color: Color, pos: usize) -> Result<(), ParseError> {
        let color_name = match color {
            Color::White => "White",
            Color::Black => "Black",
        };

        if self.kings > 1 {
            return Err(ParseError::new(
                format!(
                    "Impossible position: {} side has {} Kings (maximum is 1)",
                    color_name, self.kings
                ),
                pos,
            )
            .with_help("A standard chess position must have exactly one King per side."));
        }

        if self.pawns > 8 {
            return Err(ParseError::new(
                format!(
                    "Impossible position: {} side has {} Pawns (maximum is 8)",
                    color_name, self.pawns
                ),
                pos,
            )
            .with_help("A side starts with 8 pawns and can never exceed 8 pawns on the board."));
        }

        if self.queens > 9 {
            return Err(ParseError::new(
                format!(
                    "Impossible position: {} side has {} Queens (maximum is 9: 1 original + 8 promotions)",
                    color_name, self.queens
                ),
                pos,
            ).with_help("Even with all 8 pawns promoted to Queens, a side can have at most 9 Queens on the board."));
        }

        if self.rooks > 10 {
            return Err(ParseError::new(
                format!(
                    "Impossible position: {} side has {} Rooks (maximum is 10: 2 original + 8 promotions)",
                    color_name, self.rooks
                ),
                pos,
            ).with_help("Even with all 8 pawns promoted to Rooks, a side can have at most 10 Rooks on the board."));
        }

        if self.bishops > 10 {
            return Err(ParseError::new(
                format!(
                    "Impossible position: {} side has {} Bishops (maximum is 10: 2 original + 8 promotions)",
                    color_name, self.bishops
                ),
                pos,
            ).with_help("Even with all 8 pawns promoted to Bishops, a side can have at most 10 Bishops on the board."));
        }

        if self.knights > 10 {
            return Err(ParseError::new(
                format!(
                    "Impossible position: {} side has {} Knights (maximum is 10: 2 original + 8 promotions)",
                    color_name, self.knights
                ),
                pos,
            ).with_help("Even with all 8 pawns promoted to Knights, a side can have at most 10 Knights on the board."));
        }

        if self.total_pieces() > 16 {
            return Err(ParseError::new(
                format!(
                    "Impossible position: {} side has {} total pieces (maximum is 16)",
                    color_name,
                    self.total_pieces()
                ),
                pos,
            )
            .with_help(
                "A side starts with 16 pieces and can never exceed 16 pieces on the board.",
            ));
        }

        let promos = self.min_promotions_needed();
        if self.pawns + promos > 8 {
            return Err(ParseError::new(
                format!(
                    "Impossible position: {} side requires {} promoted pieces but also has {} unpromoted pawns (total {} exceeds the 8 starting pawns)",
                    color_name,
                    promos,
                    self.pawns,
                    self.pawns + promos
                ),
                pos,
            ).with_help("Each extra promoted piece requires one pawn to have promoted. Total unpromoted pawns plus promoted pieces cannot exceed 8."));
        }

        Ok(())
    }
}

fn validate_piece_count(
    content: &SquareContent,
    squares: Option<&[shakmaty::Square]>,
    op: crate::search::query::ComparisonOp,
    count: usize,
    pos: usize,
) -> Result<(), ParseError> {
    use crate::search::query::ComparisonOp;

    // Only validate full-board piece counts (squares.is_none())
    if squares.is_none() {
        let (role_opt, color_opt) = match content {
            SquareContent::Piece(p) => (Some(p.role), Some(p.color)),
            SquareContent::Role(r) => (Some(*r), None),
            SquareContent::Color(c) => (None, Some(*c)),
            _ => (None, None),
        };

        if let Some(role) = role_opt {
            let role_name = format!("{:?}", role);
            match role {
                shakmaty::Role::King => {
                    if (op == ComparisonOp::Equal && count == 0)
                        || (op == ComparisonOp::LessThan && count <= 1)
                        || (op == ComparisonOp::LessThanOrEqual && count == 0)
                    {
                        return Err(ParseError::new(
                            "Impossible condition: King count cannot be 0 in a legal chess game"
                                .to_string(),
                            pos,
                        )
                        .with_help(
                            "A legal chess position always contains exactly 1 King per side.",
                        ));
                    }
                    if (op == ComparisonOp::Equal && count > 1)
                        || (op == ComparisonOp::GreaterThanOrEqual && count >= 2)
                        || (op == ComparisonOp::GreaterThan && count >= 1)
                    {
                        return Err(ParseError::new(
                            format!(
                                "Impossible condition: King count cannot be {} (maximum is 1 per side)",
                                count
                            ),
                            pos,
                        ).with_help("A legal chess position can never have more than 1 King per side."));
                    }
                }
                shakmaty::Role::Pawn => {
                    if (op == ComparisonOp::Equal && count > 8)
                        || (op == ComparisonOp::GreaterThanOrEqual && count > 8)
                        || (op == ComparisonOp::GreaterThan && count >= 8)
                    {
                        return Err(ParseError::new(
                            format!(
                                "Impossible condition: Pawn count cannot be {} (maximum is 8 per side)",
                                count
                            ),
                            pos,
                        ).with_help("A side starts with 8 pawns and can never have more than 8 pawns on the board."));
                    }
                }
                shakmaty::Role::Queen => {
                    if (op == ComparisonOp::Equal && count > 9)
                        || (op == ComparisonOp::GreaterThanOrEqual && count > 9)
                        || (op == ComparisonOp::GreaterThan && count >= 9)
                    {
                        return Err(ParseError::new(
                            format!(
                                "Impossible condition: Queen count cannot be {} (maximum is 9: 1 original + 8 promotions)",
                                count
                            ),
                            pos,
                        ).with_help("Even with all 8 pawns promoted to Queens, a side can have at most 9 Queens on the board."));
                    }
                }
                shakmaty::Role::Rook | shakmaty::Role::Bishop | shakmaty::Role::Knight => {
                    if (op == ComparisonOp::Equal && count > 10)
                        || (op == ComparisonOp::GreaterThanOrEqual && count > 10)
                        || (op == ComparisonOp::GreaterThan && count >= 10)
                    {
                        return Err(ParseError::new(
                            format!(
                                "Impossible condition: {} count cannot be {} (maximum is 10: 2 original + 8 promotions)",
                                role_name, count
                            ),
                            pos,
                        ).with_help(format!(
                            "Even with all 8 pawns promoted to {}s, a side can have at most 10 {}s on the board.",
                            role_name, role_name
                        )));
                    }
                }
            }
        }

        if let Some(color) = color_opt {
            if role_opt.is_none() {
                let color_name = match color {
                    Color::White => "White",
                    Color::Black => "Black",
                };
                if (op == ComparisonOp::Equal && count > 16)
                    || (op == ComparisonOp::GreaterThanOrEqual && count > 16)
                    || (op == ComparisonOp::GreaterThan && count >= 16)
                {
                    return Err(ParseError::new(
                        format!(
                            "Impossible condition: {} total pieces count cannot be {} (maximum is 16)",
                            color_name, count
                        ),
                        pos,
                    ).with_help("A side starts with 16 pieces and can never have more than 16 pieces on the board."));
                }
            }
        }
    }

    Ok(())
}

fn piece_directly_attacks(
    piece: &shakmaty::Piece,
    attacker_sq: shakmaty::Square,
    target_sq: shakmaty::Square,
) -> bool {
    let f1 = attacker_sq.file() as i32;
    let r1 = attacker_sq.rank() as i32;
    let f2 = target_sq.file() as i32;
    let r2 = target_sq.rank() as i32;
    let df = (f1 - f2).abs();
    let dr = (r1 - r2).abs();

    match piece.role {
        shakmaty::Role::Knight => (df == 1 && dr == 2) || (df == 2 && dr == 1),
        shakmaty::Role::Pawn => {
            df == 1
                && match piece.color {
                    Color::White => r2 - r1 == 1,
                    Color::Black => r1 - r2 == 1,
                }
        }
        shakmaty::Role::King => df <= 1 && dr <= 1 && (df + dr > 0),
        shakmaty::Role::Bishop => df == dr && df == 1,
        shakmaty::Role::Rook => (df == 1 && dr == 0) || (df == 0 && dr == 1),
        shakmaty::Role::Queen => (df <= 1 && dr <= 1) && (df + dr > 0),
    }
}

fn get_move_color(m: &MovePattern) -> Option<Color> {
    if let Some(c) = m.color {
        return Some(c);
    }
    if let Some(ref from_pcs) = m.from_pieces {
        if let Some(c) = get_piece_color_from_content(from_pcs) {
            return Some(c);
        }
    }
    None
}

fn get_piece_color_from_content(content: &[SquareContent]) -> Option<Color> {
    if content.is_empty() {
        return None;
    }
    let mut colors = Vec::new();
    for c in content {
        match c {
            SquareContent::Piece(p) => colors.push(p.color),
            SquareContent::Color(col) => colors.push(*col),
            _ => return None,
        }
    }
    let first = colors[0];
    if colors.iter().all(|&c| c == first) {
        Some(first)
    } else {
        None
    }
}
