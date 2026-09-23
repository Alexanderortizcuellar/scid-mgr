pub mod builder;
pub mod codec;
pub mod core;
pub mod dynamic;
pub mod types;

// Re-export core types and functions for seamless library usage
pub use builder::{build_for_pgn, build_for_scid};
pub use codec::{
    decode_tree_position_payload, encode_tree_position_payload, generate_tree_report,
    parse_target_position, read_varint, write_varint,
};
pub use core::TreeIndex;
pub use dynamic::{calculate_tree_for_pgn, calculate_tree_for_scid};
pub use types::{
    IndexStatus, OpeningTreeMoveView, OpeningTreeReport, PackedMove, SortedTreeIndexEntry,
    TreeIndexDiagnostics, TreeIndexHeader, TreeMoveStats, TreePositionNode, DEFAULT_MAX_TREE_PLY,
    HEADER_SIZE, TREE_INDEX_MAGIC, TREE_INDEX_VERSION,
};
