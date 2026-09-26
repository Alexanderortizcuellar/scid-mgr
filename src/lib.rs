//! # SCID Manager (`scid-mgr`)
//!
//! A high-performance Rust library and CLI toolkit for managing, searching,
//! indexing, and querying SCID (`.si4` / `.si5`) and PGN chess databases.
//!
//! ## Overview
//!
//! `scid-mgr` provides:
//! - **Database Access & Manipulation**: Full read/write/edit/sort support for SCID (`si4`/`si5`) and PGN databases via [`ScidDatabaseWrapper`] and [`PgnDatabaseWrapper`].
//! - **Fast Position Indexing**: Companion inverted index (`.pos.idx`) support for instantaneous candidate matching using Delta-Varint encoded posting lists.
//! - **Instant Opening Tree Statistics**: High-speed opening book / tree index (`.tree.idx`) generation and querying.
//! - **Advanced Search Engine**: Exact FEN position searches, material count filters, and player/event metadata filtering.
//! - **High-Speed PGN Ingestion & Export**: Parallel chunk parsing, zero-copy ingesting, and fast serial exporting.
//!
//! ## Quick Start Example
//!
//! ```no_run
//! use scid_mgr::db::{ScidDatabaseWrapper, GameFilter};
//! use scid_mgr::position_search::MaterialFilter;
//! use std::path::Path;
//!
//! fn main() -> anyhow::Result<()> {
//!     // Open an existing SCID database (.si5 or .si4)
//!     let db = ScidDatabaseWrapper::open(Path::new("games/sample.si5"))?;
//!     println!("Total games in database: {}", db.game_count());
//!
//!     // Query games matching metadata criteria
//!     let filter = GameFilter {
//!         player: Some("Kasparov".to_string()),
//!         ..Default::default()
//!     };
//!     let (games, total_matching) = db.query_games(&filter, 0, 20);
//!     println!("Found {} games for player", total_matching);
//!
//!     // Query game PGN text
//!     if let Some(first_game) = games.first() {
//!         let pgn = db.game_pgn(first_game.id)?;
//!         println!("PGN:\n{}", pgn);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Opening Tree Example
//!
//! ```no_run
//! use scid_mgr::tree_index::TreeIndex;
//! use std::path::Path;
//!
//! fn main() -> anyhow::Result<()> {
//!     // Query tree statistics for starting position
//!     let db_path = Path::new("games/sample.si5");
//!     if let Ok(tree_index) = TreeIndex::load(db_path) {
//!         if let Some(tree) = tree_index.query_tree("") {
//!             println!("Total games at root: {}", tree.total_games);
//!             for m in tree.moves {
//!                 println!("Move: {:<6} Games: {:<8} White Win: {:.1}%", m.san, m.total_games, m.white_pct);
//!             }
//!         }
//!     }
//!     Ok(())
//! }
//! ```

pub mod benchmark;
pub mod cli;
pub mod continuation_index;
pub mod db;
pub mod endgame_index;
pub mod pgn;
pub mod pgn_db;
pub mod pgn_io;
pub mod position_index;
pub mod position_search;
pub mod search;
pub mod server;
pub mod tree_index;

// Backward-compatibility module aliases
pub mod pgn_utils {
    pub use crate::pgn_io::*;
}
pub mod zero_copy_ingest {
    pub use crate::pgn_io::*;
}

// Public re-exports for ergonomic library consumption
pub use benchmark::{BenchmarkItem, BenchmarkReport};
pub use continuation_index::{
    ContinuationLine, ContinuationNode, ContinuationQuery, ContinuationResult, HotGraph,
    HotGraphBuildConfig, HotGraphHeader, HotGraphMetadata, HotGraphQueryable, MmapHotGraph,
};
pub use db::{DbStats, GameFilter, GameSummary, ScidDatabaseWrapper, ScidFormat};
pub use endgame_index::{
    CategoryPopularity, EndgameCatalog, EndgameDetector, EndgameFeatureDef, EndgameIndexBuilder,
    EndgamePopularityReport, EndgameQueryEngine, FeatureIndexHeader, FeaturePopularity,
    FeatureQueryReport, GameFeatureRecord, MmapFeatureIndex,
};
pub use pgn_db::{CompactPgnRecord, PgnDatabaseWrapper, PgnNameTables};
pub use position_index::{
    IndexDiagnostics as PositionIndexDiagnostics, IndexStatus, PositionIndex, PositionIndexHeader,
    PositionPostingList,
};
pub use position_search::{MaterialFilter, PositionMatch, PositionSearchResult};
pub use search::{
    ComparisonOp, GameSearchEvaluator, HeaderMatcher, HeaderPredicate, MaterialPredicate,
    MovePattern, MoveRecord, PathMatcher, PathPattern, PositionMatcher, PositionPattern,
    QueryMatchResult, SearchQuery, SquareContent,
};
pub use tree_index::{
    OpeningTreeMoveView, OpeningTreeReport, TreeIndex, TreeIndexDiagnostics, TreeIndexHeader,
    TreeMoveStats, TreePositionNode,
};
