pub mod builder;
pub mod catalog;
pub mod detector;
pub mod model;
pub mod query;
pub mod serializer;
pub mod types;

pub use builder::{EndgameIndexBuilder, EndgameProgressCallback};
pub use catalog::{ConditionDef, CountSpec, EndgameCatalog, EndgameFeatureDef, SidePattern};
pub use detector::{EndgameDetector, FeatureDetector};
pub use model::{
    FeatureIndexHeader, GameFeatureRecord, FEATURE_INDEX_MAGIC, FEATURE_INDEX_VERSION, HEADER_SIZE,
    RECORD_SIZE,
};
pub use query::EndgameQueryEngine;
pub use serializer::{resolve_companion_feat_path, FeatureIndexWriter, MmapFeatureIndex};
pub use types::{
    CategoryPopularity, EndgamePopularityReport, FeaturePopularity, FeatureQueryReport,
};
