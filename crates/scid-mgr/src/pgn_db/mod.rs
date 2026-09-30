pub mod builder;
pub mod core;
pub mod search_adapter;
pub mod sorting;
pub mod types;

// Re-export primary types and functions for seamless library usage
pub use core::{PgnDatabase, PgnDatabaseWrapper};
pub use sorting::sort_pgn_file;
pub use types::{
    pack_date, pack_eco, pack_result, unpack_date, unpack_eco, unpack_result, CompactPgnRecord,
    PgnIndexEntry, PgnNameTables, PGN_INDEX_MAGIC, PGN_INDEX_VERSION,
};
