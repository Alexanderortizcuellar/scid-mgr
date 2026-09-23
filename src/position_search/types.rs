use serde::{Deserialize, Serialize};
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{Chess, Color, EnPassantMode, Position, Role, Square};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionMatch {
    pub game_id: usize,
    pub ply: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSearchResult {
    pub target_fen: String,
    pub target_hash: u64,
    pub matches: Vec<PositionMatch>,
    pub total_games_searched: usize,
    pub elapsed_ms: f64,
}

#[derive(Debug, Clone)]
pub enum PositionTargetMatcher {
    /// Exact Zobrist hash (matching board, turn, castling)
    ExactHash(u64),
    /// Complete board match with optional turn filter (None = any turn)
    BoardWithTurn {
        board: shakmaty::Board,
        turn: Option<Color>,
    },
    /// Partial piece placement on specific squares
    PartialPieces(Vec<(Square, Role, Color)>),
}

impl PositionTargetMatcher {
    #[inline]
    pub fn matches(&self, pos: &Chess) -> bool {
        match self {
            Self::ExactHash(target_hash) => {
                let h: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
                h.0 == *target_hash
            }
            Self::BoardWithTurn { board, turn } => {
                if let Some(t) = turn {
                    if pos.turn() != *t {
                        return false;
                    }
                }
                pos.board() == board
            }
            Self::PartialPieces(pieces) => {
                let board = pos.board();
                pieces.iter().all(|&(sq, role, color)| {
                    board.piece_at(sq) == Some(shakmaty::Piece { color, role })
                })
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MaterialFilter {
    pub white_queens: Option<u8>,
    pub white_rooks: Option<u8>,
    pub white_bishops: Option<u8>,
    pub white_knights: Option<u8>,
    pub white_pawns: Option<u8>,

    pub black_queens: Option<u8>,
    pub black_rooks: Option<u8>,
    pub black_bishops: Option<u8>,
    pub black_knights: Option<u8>,
    pub black_pawns: Option<u8>,

    pub opposite_bishops: Option<bool>,
    pub same_bishops: Option<bool>,

    pub match_any_ply: bool,
    pub max_ply: Option<usize>,
}
