//! # Chess Search & Query Engine (`scid_mgr::search`)
//!
//! A modular, CQL-inspired chess search engine for pattern matching,
//! move-sequence paths, board configurations, and metadata filtering.

pub mod annotation;
pub mod evaluator;
pub mod explain;
pub mod header;
pub mod parser;
pub mod path;
pub mod pattern;
pub mod pawn;
pub mod query;
pub mod scid_adapter;
pub mod squares;
pub mod tactics;
pub mod transform;

#[cfg(test)]
pub mod tests;

// Public re-exports
pub use annotation::{AnnotationManager, AnnotationPredicate, CommentPredicate, NagPredicate};
pub use evaluator::{GameSearchEvaluator, QueryMatchResult};
pub use explain::{explain_query, QueryBranch, QueryExplanation, ToDsl};
pub use header::HeaderMatcher;
pub use parser::{ParseError, QueryParser};
pub use path::{MoveRecord, PathMatcher};
pub use pattern::PositionMatcher;
pub use pawn::PawnEvaluator;
pub use query::{
    ComparisonOp, CqlLinePattern, CqlPathConstituent, CqlPathPattern, Direction, HeaderPredicate,
    LineDirection, MaterialPredicate, MovePattern, PathPattern, PawnPredicate, PieceMatcher,
    PositionPattern, PowerPredicate, SearchQuery, SetPredicate, SquareContent, SquareOrPiece,
    SquareSetExpr, TacticalPredicate,
};
pub use scid_adapter::{ScidMatchResult, ScidSearchAdapter};
pub use tactics::TacticsEvaluator;
pub use transform::{BoardSymmetry, TransformMatcher};
