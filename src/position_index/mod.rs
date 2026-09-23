pub mod builder;
pub mod codec;
pub mod core;
pub mod types;

// Re-export core types and functions for seamless library usage
pub use builder::{build_for_pgn, build_for_scid};
pub use codec::{
    decode_position_game_ids, encode_posting_payload, parse_target_position, read_varint,
    write_varint,
};
pub use core::PositionIndex;
pub use types::{
    IndexDiagnostics, IndexStatus, PositionIndexHeader, PositionPostingList, SortedIndexEntry,
    DEFAULT_MAX_SEARCH_PLY, HEADER_SIZE, INLINE_FLAG, NUM_STRIPES, POS_INDEX_MAGIC,
    POS_INDEX_VERSION,
};

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_posting_payload_roundtrip() {
        let mut posting = PositionPostingList::new(123456789);
        posting.add(10);
        posting.add(25);
        posting.add(100);
        posting.add(5000);

        let encoded = encode_posting_payload(&posting);
        let decoded_ids = decode_position_game_ids(&encoded).unwrap();
        assert_eq!(decoded_ids, vec![10, 25, 100, 5000]);
    }
}
