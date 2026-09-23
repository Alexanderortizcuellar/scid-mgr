pub mod core;
pub mod mutations;
pub mod query;
pub mod search_adapter;
pub mod sorting;
pub mod types;

// Re-export primary types and wrapper for external API consumers
pub use core::{detect_format_from_path, ScidDatabaseWrapper};
pub use types::{result_code_to_str, DbStats, GameFilter, GameSummary, ScidFormat};
