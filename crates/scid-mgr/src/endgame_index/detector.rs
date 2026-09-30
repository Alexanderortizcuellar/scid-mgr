use crate::endgame_index::catalog::{ConditionDef, EndgameCatalog, EndgameFeatureDef, SidePattern};
use crate::endgame_index::model::GameFeatureRecord;
use shakmaty::{Board, Chess, Position, Square};

pub trait FeatureDetector {
    fn process_position(&mut self, pos: &Chess);
    fn finish_game(&mut self) -> GameFeatureRecord;
    fn reset(&mut self);
}

pub struct EndgameDetector {
    catalog: EndgameCatalog,
    accumulated_record: GameFeatureRecord,
}

impl EndgameDetector {
    pub fn new() -> Self {
        Self::with_catalog(EndgameCatalog::default_catalog())
    }

    pub fn with_catalog(catalog: EndgameCatalog) -> Self {
        Self {
            catalog,
            accumulated_record: GameFeatureRecord::new(),
        }
    }

    pub fn catalog(&self) -> &EndgameCatalog {
        &self.catalog
    }

    #[inline]
    pub fn evaluate_position(&self, pos: &Chess) -> u64 {
        let board = pos.board();

        let w_q = board.white().intersect(board.queens()).count() as u8;
        let w_r = board.white().intersect(board.rooks()).count() as u8;
        let w_b = board.white().intersect(board.bishops()).count() as u8;
        let w_n = board.white().intersect(board.knights()).count() as u8;
        let w_p = board.white().intersect(board.pawns()).count() as u8;

        let b_q = board.black().intersect(board.queens()).count() as u8;
        let b_r = board.black().intersect(board.rooks()).count() as u8;
        let b_b = board.black().intersect(board.bishops()).count() as u8;
        let b_n = board.black().intersect(board.knights()).count() as u8;
        let b_p = board.black().intersect(board.pawns()).count() as u8;

        // Quick bail-out: if total piece count is clearly middlegame (> 4 major/minor pieces per side)
        let w_minors_majors = w_q + w_r + w_b + w_n;
        let b_minors_majors = b_q + b_r + b_b + b_n;
        if w_minors_majors > 4 || b_minors_majors > 4 {
            return 0;
        }

        let mut mask = 0u64;

        for feat in &self.catalog.features {
            if Self::matches_feature(
                feat, board, w_q, w_r, w_b, w_n, w_p, b_q, b_r, b_b, b_n, b_p,
            ) {
                mask |= 1u64 << feat.bit;
            }
        }

        mask
    }

    #[inline]
    fn matches_side(pattern: &SidePattern, q: u8, r: u8, b: u8, n: u8, p: u8) -> bool {
        q == pattern.queens
            && r == pattern.rooks
            && b == pattern.bishops
            && n == pattern.knights
            && pattern.pawns.matches(p)
    }

    #[inline]
    fn matches_side_b(
        pattern: &SidePattern,
        minors_cond: Option<u8>,
        q: u8,
        r: u8,
        b: u8,
        n: u8,
        p: u8,
    ) -> bool {
        if q != pattern.queens || r != pattern.rooks || !pattern.pawns.matches(p) {
            return false;
        }
        if let Some(target_minors) = minors_cond {
            (b + n) == target_minors
        } else {
            b == pattern.bishops && n == pattern.knights
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn matches_feature(
        feat: &EndgameFeatureDef,
        board: &Board,
        w_q: u8,
        w_r: u8,
        w_b: u8,
        w_n: u8,
        w_p: u8,
        b_q: u8,
        b_r: u8,
        b_b: u8,
        b_n: u8,
        b_p: u8,
    ) -> bool {
        let minors_cond = feat.conditions.iter().find_map(|c| match c {
            ConditionDef::SideBMinorsEqual(k) => Some(*k),
            _ => None,
        });

        // Universal side normalization check:
        // Branch 1: White is Side A, Black is Side B
        let normal_match = Self::matches_side(&feat.side_a, w_q, w_r, w_b, w_n, w_p)
            && Self::matches_side_b(&feat.side_b, minors_cond, b_q, b_r, b_b, b_n, b_p);

        // Branch 2: White is Side B, Black is Side A
        let inverted_match = Self::matches_side(&feat.side_a, b_q, b_r, b_b, b_n, b_p)
            && Self::matches_side_b(&feat.side_b, minors_cond, w_q, w_r, w_b, w_n, w_p);

        if !normal_match && !inverted_match {
            return false;
        }

        // Evaluate conditions if any
        if !feat.conditions.is_empty() {
            for cond in &feat.conditions {
                match cond {
                    ConditionDef::SameColoredBishops => {
                        if !Self::check_same_colored_bishops(board) {
                            return false;
                        }
                    }
                    ConditionDef::OppositeColoredBishops => {
                        if !Self::check_opposite_colored_bishops(board) {
                            return false;
                        }
                    }
                    ConditionDef::PawnDifference(diff) => {
                        let actual_diff = w_p.abs_diff(b_p);
                        if actual_diff != *diff {
                            return false;
                        }
                    }
                    ConditionDef::SideBMinorsEqual(_) => {
                        // Already evaluated in matches_side_b
                    }
                    ConditionDef::Unknown(_) => {}
                }
            }
        }

        true
    }

    #[inline]
    fn check_same_colored_bishops(board: &Board) -> bool {
        let mut w_bishops = board.white().intersect(board.bishops()).into_iter();
        let mut b_bishops = board.black().intersect(board.bishops()).into_iter();

        if let (Some(w_sq), Some(b_sq)) = (w_bishops.next(), b_bishops.next()) {
            Self::square_is_light(w_sq) == Self::square_is_light(b_sq)
        } else {
            false
        }
    }

    #[inline]
    fn check_opposite_colored_bishops(board: &Board) -> bool {
        let mut w_bishops = board.white().intersect(board.bishops()).into_iter();
        let mut b_bishops = board.black().intersect(board.bishops()).into_iter();

        if let (Some(w_sq), Some(b_sq)) = (w_bishops.next(), b_bishops.next()) {
            Self::square_is_light(w_sq) != Self::square_is_light(b_sq)
        } else {
            false
        }
    }

    #[inline]
    fn square_is_light(sq: Square) -> bool {
        (sq.file() as u8 + sq.rank() as u8) % 2 == 1
    }
}

impl Default for EndgameDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureDetector for EndgameDetector {
    fn process_position(&mut self, pos: &Chess) {
        let mask = self.evaluate_position(pos);
        if mask != 0 {
            self.accumulated_record.endgame_bits |= mask;
        }
    }

    fn finish_game(&mut self) -> GameFeatureRecord {
        let rec = self.accumulated_record;
        self.reset();
        rec
    }

    fn reset(&mut self) {
        self.accumulated_record = GameFeatureRecord::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::fen::Fen;
    use shakmaty::CastlingMode;

    fn parse_fen(fen_str: &str) -> Chess {
        let fen: Fen = fen_str.parse().unwrap();
        fen.into_position(CastlingMode::Chess960).unwrap()
    }

    #[test]
    fn test_kp_k_detection() {
        let detector = EndgameDetector::new();
        // White K+P vs Black K
        let pos = parse_fen("8/8/8/8/4k3/8/4P3/4K3 w - - 0 1");
        let mask = detector.evaluate_position(&pos);
        assert_ne!(mask, 0);
        assert_eq!(mask & (1u64 << 0), 1u64 << 0, "Should match END_PAWN_KP_K");

        // Inverted: Black K+P vs White K
        let pos_inv = parse_fen("8/8/8/8/4K3/8/4p3/4k3 b - - 0 1");
        let mask_inv = detector.evaluate_position(&pos_inv);
        assert_eq!(
            mask_inv & (1u64 << 0),
            1u64 << 0,
            "Should match END_PAWN_KP_K inverted"
        );
    }

    #[test]
    fn test_rook_endgame_detection() {
        let detector = EndgameDetector::new();
        // Bare Rooks: R vs R (bit 5)
        let pos_rr = parse_fen("8/8/8/4k3/4r3/8/4R3/4K3 w - - 0 1");
        let mask_rr = detector.evaluate_position(&pos_rr);
        assert_eq!(
            mask_rr & (1u64 << 5),
            1u64 << 5,
            "Should match END_ROOK_R_R"
        );

        // Lucena / Philidor: R+P vs R (bit 6)
        let pos_rpr = parse_fen("8/8/8/4k3/4r3/8/4P3/4K2R w - - 0 1");
        let mask_rpr = detector.evaluate_position(&pos_rpr);
        assert_eq!(
            mask_rpr & (1u64 << 6),
            1u64 << 6,
            "Should match END_ROOK_RP_R"
        );
    }

    #[test]
    fn test_bishop_endgame_colored_squares() {
        let detector = EndgameDetector::new();
        let cat = detector.catalog();
        let scb_bit = *cat.id_to_bit.get("END_BISHOP_SCB").unwrap();
        let ocb_bit = *cat.id_to_bit.get("END_BISHOP_OCB").unwrap();

        // Same-colored bishops (e4 is dark, e2 is dark) -> END_BISHOP_SCB
        let pos_scb = parse_fen("8/8/8/4k3/4b3/8/4B3/4K3 w - - 0 1");
        let mask_scb = detector.evaluate_position(&pos_scb);
        assert_eq!(
            mask_scb & (1u64 << scb_bit),
            1u64 << scb_bit,
            "Should match END_BISHOP_SCB"
        );

        // Opposite-colored bishops (e4 is dark, f2 is light) -> END_BISHOP_OCB
        let pos_ocb = parse_fen("8/8/8/4k3/4b3/8/5B2/4K3 w - - 0 1");
        let mask_ocb = detector.evaluate_position(&pos_ocb);
        assert_eq!(
            mask_ocb & (1u64 << ocb_bit),
            1u64 << ocb_bit,
            "Should match END_BISHOP_OCB"
        );
    }

    #[test]
    fn test_rook_vs_two_minors_condition() {
        let detector = EndgameDetector::new();
        // Rook vs Bishop + Knight (bit 37)
        let pos = parse_fen("8/8/8/4k3/4bn2/8/4R3/4K3 w - - 0 1");
        let mask = detector.evaluate_position(&pos);
        assert_eq!(
            mask & (1u64 << 37),
            1u64 << 37,
            "Should match END_ROOK_VS_2MINORS"
        );
    }

    #[test]
    fn test_queen_vs_rook_minor_condition() {
        let detector = EndgameDetector::new();
        // Queen vs Rook + Knight (bit 43)
        let pos = parse_fen("8/8/8/4k3/4rn2/8/4Q3/4K3 w - - 0 1");
        let mask = detector.evaluate_position(&pos);
        assert_eq!(
            mask & (1u64 << 43),
            1u64 << 43,
            "Should match END_QUEEN_VS_ROOK_MINOR"
        );
    }
}
