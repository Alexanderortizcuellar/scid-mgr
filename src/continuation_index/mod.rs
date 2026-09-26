pub mod builder;
pub mod codec;
pub mod core;
pub mod dynamic;
pub mod types;

// Re-export core types and functions for seamless library usage
pub use builder::{
    build_for_pgn, build_for_pgn_direct, build_for_scid, build_for_scid_direct,
    HotGraphBuildConfig, StripedHotGraphBuilder,
};
pub use codec::{
    format_continuation_moves, parse_fen_fullmove, resolve_companion_hot_path, MmapHotGraph,
};
pub use core::{HotGraph, HotGraphQueryable};
pub use dynamic::{calculate_continuations_for_pgn, calculate_continuations_for_scid};
pub use types::{
    ContinuationLine, ContinuationNode, ContinuationQuery, ContinuationResult, HotEdge,
    HotGraphHeader, HotGraphMetadata, HotHashEntry, HotNode, NodeId, PackedMove, HEADER_SIZE,
    HOT_GRAPH_MAGIC, HOT_GRAPH_VERSION, NO_NODE,
};
