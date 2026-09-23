pub mod decoder;
pub mod parser;
pub mod scid_search;
pub mod types;

// Re-export decoder symbols
pub use decoder::{
    decode_extra_tags, decode_raw_move, parse_start_position, skip_extra_tags,
    standard_piece_slots, update_slots_on_move, ENCODE_END_GAME,
};

// Re-export parser symbols
pub use parser::{matches_piece_placements, parse_piece_placements, parse_position_matcher};

// Re-export search symbols
pub use scid_search::{
    matches_material, search_material_mmap, search_material_mmap_with_progress,
    search_piece_placements_mmap, search_piece_placements_mmap_with_progress,
    search_position_matcher_mmap_with_progress, search_position_mmap,
    search_position_mmap_with_progress,
};

// Re-export types
pub use types::{MaterialFilter, PositionMatch, PositionSearchResult, PositionTargetMatcher};
