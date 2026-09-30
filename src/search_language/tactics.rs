use super::query::{ComparisonOp, PieceMatcher, SquareOrPiece, TacticalPredicate};
use shakmaty::{Bitboard, Chess, Color, File, Piece, Position, Rank, Role, Square};

/// Tactical and geometric motif evaluator
pub struct TacticsEvaluator;

impl TacticsEvaluator {
    /// Evaluate a TacticalPredicate against the current board state with optional variable environment
    pub fn matches_with_env(
        pred: &TacticalPredicate,
        pos: &Chess,
        env: &std::collections::HashMap<String, Square>,
    ) -> bool {
        match pred {
            TacticalPredicate::Pin {
                pinners,
                pinneds,
                targets,
            } => Self::has_pin(pos, pinners, pinneds, targets),
            TacticalPredicate::Fork {
                attackers,
                target_slots,
                targets_pool,
                min_targets,
            } => Self::has_fork(pos, attackers, target_slots, targets_pool, *min_targets),
            TacticalPredicate::DiscoveredAttack { color, is_check } => {
                Self::has_discovered_attack(pos, *color, *is_check)
            }
            TacticalPredicate::Skewer {
                attackers,
                fronts,
                rears,
            } => Self::has_skewer(pos, attackers, fronts, rears),
            TacticalPredicate::TrappedPiece { piece } => Self::has_trapped_piece(pos, piece),
            TacticalPredicate::Outpost { piece, square } => {
                Self::has_outpost(pos, piece, square.as_ref())
            }
            TacticalPredicate::RookOnSeventh { color } => Self::has_rook_on_seventh(pos, *color),
            TacticalPredicate::OpenFile {
                file,
                semi_open_for,
            } => Self::has_open_file(pos, file.as_ref(), semi_open_for.as_ref()),
            TacticalPredicate::Distance {
                sq1,
                sq2,
                op,
                distance,
            } => Self::matches_distance_with_env(pos, sq1, sq2, *distance, *op, env),
            TacticalPredicate::Attacks { attacker, target } => {
                Self::matches_attacks_with_env(pos, attacker, target, env)
            }
        }
    }

    /// Evaluate a TacticalPredicate against the current board state
    pub fn matches(pred: &TacticalPredicate, pos: &Chess) -> bool {
        Self::matches_with_env(pred, pos, &std::collections::HashMap::new())
    }

    /// Check if attacker piece(s) attack target piece(s) or square(s) with variable environment
    pub fn matches_attacks_with_env(
        pos: &Chess,
        attacker: &SquareOrPiece,
        target: &SquareOrPiece,
        env: &std::collections::HashMap<String, Square>,
    ) -> bool {
        let attacker_squares = resolve_squares(pos, attacker, env);
        let target_squares = resolve_squares(pos, target, env);

        for &atk_sq in &attacker_squares {
            let attacks = pos.board().attacks_from(atk_sq);
            for &tgt_sq in &target_squares {
                if attacks.contains(tgt_sq) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if attacker piece(s) attack target piece(s) or square(s)
    pub fn matches_attacks(pos: &Chess, attacker: &SquareOrPiece, target: &SquareOrPiece) -> bool {
        Self::matches_attacks_with_env(pos, attacker, target, &std::collections::HashMap::new())
    }

    /// Check if there is any pin matching the criteria
    pub fn has_pin(
        pos: &Chess,
        pinner_specs: &[PieceMatcher],
        pinned_specs: &[PieceMatcher],
        target_specs: &[PieceMatcher],
    ) -> bool {
        let board = pos.board();

        for slider_sq in Square::ALL {
            if let Some(slider_piece) = board.piece_at(slider_sq) {
                // Must be a sliding piece (Bishop, Rook, Queen)
                if !matches!(slider_piece.role, Role::Bishop | Role::Rook | Role::Queen) {
                    continue;
                }
                if !pinner_specs.is_empty()
                    && !pinner_specs.iter().any(|spec| spec.matches(&slider_piece))
                {
                    continue;
                }

                let directions: &[(i32, i32)] = match slider_piece.role {
                    Role::Bishop => &[(-1, -1), (-1, 1), (1, -1), (1, 1)],
                    Role::Rook => &[(-1, 0), (1, 0), (0, -1), (0, 1)],
                    Role::Queen => &[
                        (-1, -1),
                        (-1, 1),
                        (1, -1),
                        (1, 1),
                        (-1, 0),
                        (1, 0),
                        (0, -1),
                        (0, 1),
                    ],
                    _ => &[],
                };

                for &(df, dr) in directions {
                    let mut cur_f = slider_sq.file() as i32 + df;
                    let mut cur_r = slider_sq.rank() as i32 + dr;
                    let mut first_piece: Option<(Square, Piece)> = None;

                    while (0..8).contains(&cur_f) && (0..8).contains(&cur_r) {
                        let sq = match square_from_coords(cur_f, cur_r) {
                            Some(s) => s,
                            None => break,
                        };

                        if let Some(piece) = board.piece_at(sq) {
                            if let Some((_pinned_sq, pinned_piece)) = first_piece {
                                // Second piece along ray
                                if piece.color == pinned_piece.color {
                                    // Pinned against this piece (e.g. King, Queen, Rook)
                                    let pinned_match = pinned_specs.is_empty()
                                        || pinned_specs.iter().any(|s| s.matches(&pinned_piece));
                                    let target_match = target_specs.is_empty()
                                        || target_specs.iter().any(|s| s.matches(&piece));

                                    // Default target should be more valuable or King if no target specified
                                    let is_valid_target = if !target_specs.is_empty() {
                                        target_match
                                    } else {
                                        piece.role == Role::King
                                            || piece_value(piece.role)
                                                > piece_value(pinned_piece.role)
                                    };

                                    if pinned_match && is_valid_target {
                                        return true;
                                    }
                                }
                                break;
                            } else {
                                // First piece along ray
                                if piece.color != slider_piece.color {
                                    first_piece = Some((sq, piece));
                                } else {
                                    // Blocked by friendly piece
                                    break;
                                }
                            }
                        }

                        cur_f += df;
                        cur_r += dr;
                    }
                }
            }
        }

        false
    }

    /// Check if there is any fork matching the criteria
    pub fn has_fork(
        pos: &Chess,
        attacker_specs: &[PieceMatcher],
        target_slots: &[Vec<PieceMatcher>],
        targets_pool: &[PieceMatcher],
        min_targets: usize,
    ) -> bool {
        let board = pos.board();

        for attacker_sq in Square::ALL {
            if let Some(attacker_piece) = board.piece_at(attacker_sq) {
                if !attacker_specs.is_empty()
                    && !attacker_specs
                        .iter()
                        .any(|spec| spec.matches(&attacker_piece))
                {
                    continue;
                }

                let attacks = board.attacks_from(attacker_sq);
                let enemy_color = attacker_piece.color.other();
                let enemy_targets_bb = board.by_color(enemy_color) & attacks;
                let enemy_pieces: Vec<Piece> = enemy_targets_bb
                    .into_iter()
                    .filter_map(|sq| board.piece_at(sq))
                    .collect();

                if enemy_pieces.len() < 2 {
                    continue;
                }

                // 1. If distinct target slots are specified (e.g. fork(P, q, r) or fork([P, N], [q, k], [r, b])),
                // perform distinct 1-to-1 matching across slots.
                if !target_slots.is_empty() {
                    if enemy_pieces.len() < target_slots.len() {
                        continue;
                    }
                    if !Self::match_distinct_target_slots(0, target_slots, &enemy_pieces, 0) {
                        continue;
                    }
                }

                // 2. If a targets pool is specified (e.g. fork(attacker in [N, B, P], targets in [q, r])),
                // count how many attacked pieces match any piece in the pool.
                if !targets_pool.is_empty() {
                    let matched_pool = enemy_pieces
                        .iter()
                        .filter(|p| targets_pool.iter().any(|spec| spec.matches(p)))
                        .count();
                    if matched_pool < min_targets.max(2) {
                        continue;
                    }
                } else if target_slots.is_empty() {
                    // Default fork: targets are valuable (Queen, Rook, Bishop, Knight, King)
                    let matched_valuable = enemy_pieces
                        .iter()
                        .filter(|p| {
                            matches!(
                                p.role,
                                Role::King | Role::Queen | Role::Rook | Role::Bishop | Role::Knight
                            )
                        })
                        .count();
                    if matched_valuable < min_targets.max(2) {
                        continue;
                    }
                }

                return true;
            }
        }

        false
    }

    fn match_distinct_target_slots(
        slot_idx: usize,
        target_slots: &[Vec<PieceMatcher>],
        enemy_pieces: &[Piece],
        used_mask: u32,
    ) -> bool {
        if slot_idx >= target_slots.len() {
            return true;
        }
        let slot_specs = &target_slots[slot_idx];
        for (i, piece) in enemy_pieces.iter().enumerate() {
            if (used_mask & (1 << i)) == 0
                && (slot_specs.is_empty() || slot_specs.iter().any(|spec| spec.matches(piece)))
                && Self::match_distinct_target_slots(
                    slot_idx + 1,
                    target_slots,
                    enemy_pieces,
                    used_mask | (1 << i),
                )
            {
                return true;
            }
        }
        false
    }

    /// Check for discovered attacks or checks
    pub fn has_discovered_attack(pos: &Chess, color: Color, is_check: bool) -> bool {
        if is_check && !pos.is_check() {
            return false;
        }

        let board = pos.board();
        let enemy_color = color.other();
        let enemy_king = match board.king_of(enemy_color) {
            Some(k) => k,
            None => return false,
        };

        let enemy_targets = if is_check {
            Bitboard::from_square(enemy_king)
        } else {
            board.by_color(enemy_color)
        };

        // Scan if any slider has an attack ray aligned with enemy pieces
        for slider_sq in board.by_color(color) {
            if let Some(piece) = board.piece_at(slider_sq) {
                if matches!(piece.role, Role::Bishop | Role::Rook | Role::Queen) {
                    let attacks = board.attacks_from(slider_sq);
                    if !(attacks & enemy_targets).is_empty() {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check for a skewer
    pub fn has_skewer(
        pos: &Chess,
        attacker_specs: &[PieceMatcher],
        front_specs: &[PieceMatcher],
        rear_specs: &[PieceMatcher],
    ) -> bool {
        let board = pos.board();

        for slider_sq in Square::ALL {
            if let Some(slider_piece) = board.piece_at(slider_sq) {
                if !matches!(slider_piece.role, Role::Bishop | Role::Rook | Role::Queen) {
                    continue;
                }
                if !attacker_specs.is_empty()
                    && !attacker_specs
                        .iter()
                        .any(|spec| spec.matches(&slider_piece))
                {
                    continue;
                }

                let directions: &[(i32, i32)] = match slider_piece.role {
                    Role::Bishop => &[(-1, -1), (-1, 1), (1, -1), (1, 1)],
                    Role::Rook => &[(-1, 0), (1, 0), (0, -1), (0, 1)],
                    Role::Queen => &[
                        (-1, -1),
                        (-1, 1),
                        (1, -1),
                        (1, 1),
                        (-1, 0),
                        (1, 0),
                        (0, -1),
                        (0, 1),
                    ],
                    _ => &[],
                };

                for &(df, dr) in directions {
                    let mut cur_f = slider_sq.file() as i32 + df;
                    let mut cur_r = slider_sq.rank() as i32 + dr;
                    let mut first_piece: Option<(Square, Piece)> = None;

                    while (0..8).contains(&cur_f) && (0..8).contains(&cur_r) {
                        let sq = match square_from_coords(cur_f, cur_r) {
                            Some(s) => s,
                            None => break,
                        };

                        if let Some(piece) = board.piece_at(sq) {
                            if let Some((_, front_piece)) = first_piece {
                                if piece.color == front_piece.color {
                                    let front_match = front_specs.is_empty()
                                        || front_specs.iter().any(|s| s.matches(&front_piece));
                                    let rear_match = rear_specs.is_empty()
                                        || rear_specs.iter().any(|s| s.matches(&piece));

                                    // Front piece is more valuable than rear piece (e.g. King in front of Queen)
                                    let is_skewer_hierarchy = piece_value(front_piece.role)
                                        >= piece_value(piece.role)
                                        || front_piece.role == Role::King;

                                    if front_match && rear_match && is_skewer_hierarchy {
                                        return true;
                                    }
                                }
                                break;
                            } else {
                                if piece.color != slider_piece.color {
                                    first_piece = Some((sq, piece));
                                } else {
                                    break;
                                }
                            }
                        }

                        cur_f += df;
                        cur_r += dr;
                    }
                }
            }
        }

        false
    }

    /// Check if a piece is trapped (0 legal moves or all moves captured by lesser pieces)
    pub fn has_trapped_piece(pos: &Chess, spec: &PieceMatcher) -> bool {
        let board = pos.board();

        for sq in Square::ALL {
            if let Some(piece) = board.piece_at(sq) {
                if spec.matches(&piece) && piece.role != Role::King && piece.role != Role::Pawn {
                    let attacks = board.attacks_from(sq);
                    let non_friendly = attacks & !board.by_color(piece.color);
                    let enemy_pawns = board.by_piece(Piece {
                        color: piece.color.other(),
                        role: Role::Pawn,
                    });

                    // If non-friendly destinations are all attacked by enemy pawns or occupied by guarded pieces
                    let mut safe_destinations = 0;
                    for dest_sq in non_friendly {
                        let is_attacked_by_pawn = !(pawn_defenders(dest_sq, piece.color.other())
                            & enemy_pawns)
                            .is_empty();
                        if !is_attacked_by_pawn {
                            safe_destinations += 1;
                        }
                    }

                    if safe_destinations == 0 {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check for an outpost piece on 4th/5th/6th rank
    pub fn has_outpost(pos: &Chess, spec: &PieceMatcher, square_filter: Option<&Square>) -> bool {
        let board = pos.board();

        for sq in Square::ALL {
            if let Some(piece) = board.piece_at(sq) {
                if !spec.matches(&piece) {
                    continue;
                }
                if let Some(target_sq) = square_filter {
                    if sq != *target_sq {
                        continue;
                    }
                }

                let rank = sq.rank();
                let is_outpost_rank = match piece.color {
                    Color::White => matches!(rank, Rank::Fourth | Rank::Fifth | Rank::Sixth),
                    Color::Black => matches!(rank, Rank::Third | Rank::Fourth | Rank::Fifth),
                };

                if !is_outpost_rank {
                    continue;
                }

                // Must be protected by a friendly pawn
                let friendly_pawns = board.by_piece(Piece {
                    color: piece.color,
                    role: Role::Pawn,
                });
                let pawn_guard = pawn_defenders(sq, piece.color);
                if (friendly_pawns & pawn_guard).is_empty() {
                    continue;
                }

                // Cannot be evicted by enemy pawns on adjacent files
                let enemy_pawns = board.by_piece(Piece {
                    color: piece.color.other(),
                    role: Role::Pawn,
                });
                let file = sq.file();
                let mut can_be_evicted = false;

                if let Some(left) = file.offset(-1) {
                    let left_mask = Bitboard::from(left);
                    let enemy_left = enemy_pawns & left_mask;
                    if !enemy_left.is_empty() {
                        // Check if any enemy pawn is behind or on this rank
                        for p_sq in enemy_left {
                            let p_rank = p_sq.rank();
                            let can_advance = match piece.color {
                                Color::White => p_rank as usize > rank as usize,
                                Color::Black => (p_rank as usize) < rank as usize,
                            };
                            if can_advance {
                                can_be_evicted = true;
                                break;
                            }
                        }
                    }
                }

                if !can_be_evicted {
                    if let Some(right) = file.offset(1) {
                        let right_mask = Bitboard::from(right);
                        let enemy_right = enemy_pawns & right_mask;
                        if !enemy_right.is_empty() {
                            for p_sq in enemy_right {
                                let p_rank = p_sq.rank();
                                let can_advance = match piece.color {
                                    Color::White => p_rank as usize > rank as usize,
                                    Color::Black => (p_rank as usize) < rank as usize,
                                };
                                if can_advance {
                                    can_be_evicted = true;
                                    break;
                                }
                            }
                        }
                    }
                }

                if !can_be_evicted {
                    return true;
                }
            }
        }

        false
    }

    /// Check for Rook on 7th rank
    pub fn has_rook_on_seventh(pos: &Chess, color: Color) -> bool {
        let board = pos.board();
        let target_rank = match color {
            Color::White => Rank::Seventh,
            Color::Black => Rank::Second,
        };
        let rooks = board.by_piece(Piece {
            color,
            role: Role::Rook,
        });
        let rank_mask = Bitboard::from(target_rank);

        !(rooks & rank_mask).is_empty()
    }

    /// Check for open or semi-open file
    pub fn has_open_file(
        pos: &Chess,
        file_filter: Option<&File>,
        semi_open_for: Option<&Color>,
    ) -> bool {
        let board = pos.board();
        let white_pawns = board.by_piece(Piece {
            color: Color::White,
            role: Role::Pawn,
        });
        let black_pawns = board.by_piece(Piece {
            color: Color::Black,
            role: Role::Pawn,
        });

        let files: &[File] = match file_filter {
            Some(f) => std::slice::from_ref(f),
            None => &File::ALL,
        };

        for &file in files {
            let file_mask = Bitboard::from(file);
            let has_w = !(white_pawns & file_mask).is_empty();
            let has_b = !(black_pawns & file_mask).is_empty();

            match semi_open_for {
                None => {
                    // Fully open file (no pawns of either color)
                    if !has_w && !has_b {
                        return true;
                    }
                }
                Some(Color::White) => {
                    // Semi-open for White (no White pawn, at least 1 Black pawn)
                    if !has_w && has_b {
                        return true;
                    }
                }
                Some(Color::Black) => {
                    // Semi-open for Black (no Black pawn, at least 1 White pawn)
                    if has_w && !has_b {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Calculate distance between two squares or pieces with optional variable environment
    pub fn matches_distance_with_env(
        pos: &Chess,
        sq1: &SquareOrPiece,
        sq2: &SquareOrPiece,
        expected: usize,
        op: ComparisonOp,
        env: &std::collections::HashMap<String, Square>,
    ) -> bool {
        let squares1 = resolve_squares(pos, sq1, env);
        let squares2 = resolve_squares(pos, sq2, env);

        for &s1 in &squares1 {
            for &s2 in &squares2 {
                let dist = chebyshev_distance(s1, s2);
                if compare_distance(dist, expected, op) {
                    return true;
                }
            }
        }

        false
    }

    /// Calculate distance between two squares or pieces
    pub fn matches_distance(
        pos: &Chess,
        sq1: &SquareOrPiece,
        sq2: &SquareOrPiece,
        expected: usize,
        op: ComparisonOp,
    ) -> bool {
        Self::matches_distance_with_env(
            pos,
            sq1,
            sq2,
            expected,
            op,
            &std::collections::HashMap::new(),
        )
    }
}

fn resolve_squares(
    pos: &Chess,
    target: &SquareOrPiece,
    env: &std::collections::HashMap<String, Square>,
) -> Vec<Square> {
    match target {
        SquareOrPiece::Square(sq) => vec![*sq],
        SquareOrPiece::Variable(var) => {
            if let Some(&sq) = env.get(var) {
                vec![sq]
            } else {
                Vec::new()
            }
        }
        SquareOrPiece::Empty => {
            let mut list = Vec::new();
            for sq in Square::ALL {
                if pos.board().piece_at(sq).is_none() {
                    list.push(sq);
                }
            }
            list
        }
        SquareOrPiece::Piece(spec) => {
            let mut list = Vec::new();
            for sq in Square::ALL {
                if let Some(piece) = pos.board().piece_at(sq) {
                    if spec.matches(&piece) {
                        list.push(sq);
                    }
                }
            }
            list
        }
    }
}

fn chebyshev_distance(sq1: Square, sq2: Square) -> usize {
    let f1 = sq1.file() as i32;
    let r1 = sq1.rank() as i32;
    let f2 = sq2.file() as i32;
    let r2 = sq2.rank() as i32;

    (f1 - f2).abs().max((r1 - r2).abs()) as usize
}

fn square_from_coords(file: i32, rank: i32) -> Option<Square> {
    if (0..8).contains(&file) && (0..8).contains(&rank) {
        Some(Square::new((rank * 8 + file) as u32))
    } else {
        None
    }
}

fn pawn_defenders(sq: Square, friendly_color: Color) -> Bitboard {
    let mut bb = Bitboard::EMPTY;
    match friendly_color {
        Color::White => {
            if sq.file() != File::A {
                if let Some(l) = sq.offset(-9) {
                    bb |= Bitboard::from_square(l);
                }
            }
            if sq.file() != File::H {
                if let Some(r) = sq.offset(-7) {
                    bb |= Bitboard::from_square(r);
                }
            }
        }
        Color::Black => {
            if sq.file() != File::A {
                if let Some(l) = sq.offset(7) {
                    bb |= Bitboard::from_square(l);
                }
            }
            if sq.file() != File::H {
                if let Some(r) = sq.offset(9) {
                    bb |= Bitboard::from_square(r);
                }
            }
        }
    }
    bb
}

fn piece_value(role: Role) -> u32 {
    match role {
        Role::Pawn => 1,
        Role::Knight => 3,
        Role::Bishop => 3,
        Role::Rook => 5,
        Role::Queen => 9,
        Role::King => 100,
    }
}

fn compare_distance(actual: usize, expected: usize, op: ComparisonOp) -> bool {
    match op {
        ComparisonOp::Equal => actual == expected,
        ComparisonOp::NotEqual => actual != expected,
        ComparisonOp::GreaterThan => actual > expected,
        ComparisonOp::GreaterThanOrEqual => actual >= expected,
        ComparisonOp::LessThan => actual < expected,
        ComparisonOp::LessThanOrEqual => actual <= expected,
        _ => false,
    }
}
