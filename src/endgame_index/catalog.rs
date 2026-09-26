use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountSpec {
    Exact(u8),
    Range { min: u8, max: u8 },
}

impl CountSpec {
    #[inline]
    pub fn matches(&self, count: u8) -> bool {
        match *self {
            CountSpec::Exact(e) => count == e,
            CountSpec::Range { min, max } => count >= min && count <= max,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionDef {
    SameColoredBishops,
    OppositeColoredBishops,
    PawnDifference(u8),
    SideBMinorsEqual(u8),
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct SidePattern {
    pub queens: u8,
    pub rooks: u8,
    pub bishops: u8,
    pub knights: u8,
    pub pawns: CountSpec,
}

impl Default for SidePattern {
    fn default() -> Self {
        Self {
            queens: 0,
            rooks: 0,
            bishops: 0,
            knights: 0,
            pawns: CountSpec::Exact(0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EndgameFeatureDef {
    pub id: String,
    pub bit: u8,
    pub name: String,
    pub short_name: String,
    pub gbr_code: Option<String>,
    pub category_id: String,
    pub side_a: SidePattern,
    pub side_b: SidePattern,
    pub conditions: Vec<ConditionDef>,
}

#[derive(Debug, Clone)]
pub struct EndgameCatalog {
    pub version: u32,
    pub features: Vec<EndgameFeatureDef>,
    pub id_to_bit: HashMap<String, u8>,
    pub bit_to_feature: HashMap<u8, EndgameFeatureDef>,
}

impl EndgameCatalog {
    pub fn default_catalog() -> Self {
        // Embed the 47 standard v2.0 catalog definitions
        let yaml_str = include_str!("../../catalog/endgames.yaml");
        Self::parse_yaml(yaml_str).expect("Failed to parse embedded default endgame catalog")
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read catalog at {:?}", path.as_ref()))?;
        Self::parse_yaml(&content)
    }

    pub fn load_or_default(custom_path: Option<&Path>) -> Self {
        if let Some(path) = custom_path {
            if let Ok(cat) = Self::load_from_file(path) {
                return cat;
            }
        }
        Self::default_catalog()
    }

    pub fn parse_yaml(yaml_str: &str) -> Result<Self> {
        let mut features = Vec::new();
        let mut id_to_bit = HashMap::new();
        let mut bit_to_feature = HashMap::new();
        let mut version = 2;

        let mut current_cat_id = String::new();
        let mut current_feature_id = String::new();
        let mut current_bit = 0u8;
        let mut current_name = String::new();
        let mut current_short_name = String::new();
        let mut current_gbr = None;
        let mut current_side_a = SidePattern::default();
        let mut current_side_b = SidePattern::default();
        let mut current_conditions = Vec::new();
        let mut in_features_block = false;
        let mut in_feature = false;
        let mut in_side_a = false;
        let mut in_side_b = false;
        let mut in_conditions = false;

        let flush_current_feature = |features: &mut Vec<EndgameFeatureDef>,
                                     id_to_bit: &mut HashMap<String, u8>,
                                     bit_to_feature: &mut HashMap<u8, EndgameFeatureDef>,
                                     id: &str,
                                     bit: u8,
                                     name: &str,
                                     short_name: &str,
                                     gbr: &Option<String>,
                                     cat: &str,
                                     side_a: &SidePattern,
                                     side_b: &SidePattern,
                                     conds: &[ConditionDef]| {
            if !id.is_empty() {
                let feat = EndgameFeatureDef {
                    id: id.to_string(),
                    bit,
                    name: name.to_string(),
                    short_name: short_name.to_string(),
                    gbr_code: gbr.clone(),
                    category_id: cat.to_string(),
                    side_a: side_a.clone(),
                    side_b: side_b.clone(),
                    conditions: conds.to_vec(),
                };
                id_to_bit.insert(id.to_string(), bit);
                bit_to_feature.insert(bit, feat.clone());
                features.push(feat);
            }
        };

        for line in yaml_str.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with("schema_version:") {
                if let Some(v_str) = trimmed.split(':').nth(1) {
                    let clean = v_str.trim().trim_matches('"').trim_matches('\'');
                    if let Ok(v) = clean.parse::<f32>() {
                        version = v as u32;
                    }
                }
                continue;
            }

            if trimmed == "features:" {
                in_features_block = true;
                continue;
            }

            let indent = line.len() - line.trim_start().len();

            // Category detection: "- id: CATEGORY" at root level or before features block
            if (!in_features_block && (trimmed.starts_with("- id:") || trimmed.starts_with("id:")))
                || (indent == 0 && trimmed.starts_with("- id:"))
            {
                if in_feature {
                    flush_current_feature(
                        &mut features,
                        &mut id_to_bit,
                        &mut bit_to_feature,
                        &current_feature_id,
                        current_bit,
                        &current_name,
                        &current_short_name,
                        &current_gbr,
                        &current_cat_id,
                        &current_side_a,
                        &current_side_b,
                        &current_conditions,
                    );
                    in_feature = false;
                    current_feature_id.clear();
                }
                in_features_block = false;
                let val = trimmed
                    .strip_prefix("- id:")
                    .or_else(|| trimmed.strip_prefix("id:"))
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'');
                current_cat_id = val.to_string();
                continue;
            }

            // Feature start: inside features block
            if in_features_block && (trimmed.starts_with("- id:") || trimmed.starts_with("id:")) {
                if in_feature {
                    flush_current_feature(
                        &mut features,
                        &mut id_to_bit,
                        &mut bit_to_feature,
                        &current_feature_id,
                        current_bit,
                        &current_name,
                        &current_short_name,
                        &current_gbr,
                        &current_cat_id,
                        &current_side_a,
                        &current_side_b,
                        &current_conditions,
                    );
                }

                in_feature = true;
                in_side_a = false;
                in_side_b = false;
                in_conditions = false;
                current_side_a = SidePattern::default();
                current_side_b = SidePattern::default();
                current_conditions.clear();
                current_gbr = None;
                current_name.clear();
                current_short_name.clear();

                let val = trimmed
                    .strip_prefix("- id:")
                    .or_else(|| trimmed.strip_prefix("id:"))
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'');
                current_feature_id = val.to_string();
                continue;
            }

            if in_feature {
                if trimmed.starts_with("bit:") {
                    if let Some(val) = trimmed.split(':').nth(1) {
                        current_bit = val.trim().parse().unwrap_or(0);
                    }
                } else if trimmed.starts_with("name:") {
                    if let Some(val) = trimmed.split_once(':') {
                        current_name = val.1.trim().trim_matches('"').to_string();
                    }
                } else if trimmed.starts_with("short_name:") {
                    if let Some(val) = trimmed.split_once(':') {
                        current_short_name = val.1.trim().trim_matches('"').to_string();
                    }
                } else if trimmed.starts_with("gbr_code:") {
                    if let Some(val) = trimmed.split_once(':') {
                        current_gbr = Some(val.1.trim().trim_matches('"').to_string());
                    }
                } else if trimmed == "side_a:" {
                    in_side_a = true;
                    in_side_b = false;
                    in_conditions = false;
                } else if trimmed == "side_b:" {
                    in_side_a = false;
                    in_side_b = true;
                    in_conditions = false;
                } else if trimmed == "conditions:" {
                    in_side_a = false;
                    in_side_b = false;
                    in_conditions = true;
                } else if trimmed.starts_with("references:") || trimmed.starts_with("description:")
                {
                    in_side_a = false;
                    in_side_b = false;
                    in_conditions = false;
                } else if in_side_a || in_side_b {
                    let side = if in_side_a {
                        &mut current_side_a
                    } else {
                        &mut current_side_b
                    };
                    Self::parse_side_field(side, trimmed);
                } else if in_conditions && trimmed.starts_with('-') {
                    let cond_name = trimmed.trim_start_matches('-').trim();
                    let cond = match cond_name {
                        "same_colored_bishops" => ConditionDef::SameColoredBishops,
                        "opposite_colored_bishops" => ConditionDef::OppositeColoredBishops,
                        "pawn_difference_1" => ConditionDef::PawnDifference(1),
                        "pawn_difference_2" => ConditionDef::PawnDifference(2),
                        "side_b_minors_equal_1" => ConditionDef::SideBMinorsEqual(1),
                        "side_b_minors_equal_2" => ConditionDef::SideBMinorsEqual(2),
                        "side_b_minors_equal_3" => ConditionDef::SideBMinorsEqual(3),
                        other => ConditionDef::Unknown(other.to_string()),
                    };
                    current_conditions.push(cond);
                }
            }
        }

        if in_feature {
            flush_current_feature(
                &mut features,
                &mut id_to_bit,
                &mut bit_to_feature,
                &current_feature_id,
                current_bit,
                &current_name,
                &current_short_name,
                &current_gbr,
                &current_cat_id,
                &current_side_a,
                &current_side_b,
                &current_conditions,
            );
        }

        if features.is_empty() {
            bail!("No features parsed from catalog YAML");
        }

        Ok(Self {
            version,
            features,
            id_to_bit,
            bit_to_feature,
        })
    }

    fn parse_side_field(side: &mut SidePattern, line: &str) {
        let (key, val) = match line.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => return,
        };

        match key {
            "queens" => side.queens = val.parse().unwrap_or(0),
            "rooks" => side.rooks = val.parse().unwrap_or(0),
            "bishops" => side.bishops = val.parse().unwrap_or(0),
            "knights" => side.knights = val.parse().unwrap_or(0),
            "min" => {
                let min_val = val.parse().unwrap_or(0);
                match side.pawns {
                    CountSpec::Range { ref mut min, .. } => *min = min_val,
                    _ => {
                        side.pawns = CountSpec::Range {
                            min: min_val,
                            max: 8,
                        }
                    }
                }
            }
            "max" => {
                let max_val = val.parse().unwrap_or(8);
                match side.pawns {
                    CountSpec::Range { ref mut max, .. } => *max = max_val,
                    _ => {
                        side.pawns = CountSpec::Range {
                            min: 0,
                            max: max_val,
                        }
                    }
                }
            }
            "pawns" => {
                if val.is_empty() {
                    side.pawns = CountSpec::Range { min: 0, max: 8 };
                } else if val.starts_with('{') {
                    let min_part = val
                        .split("min:")
                        .nth(1)
                        .and_then(|s| s.split(',').next())
                        .unwrap_or("0")
                        .trim();
                    let max_part = val
                        .split("max:")
                        .nth(1)
                        .and_then(|s| s.split('}').next())
                        .unwrap_or("8")
                        .trim();
                    let min = min_part.parse().unwrap_or(0);
                    let max = max_part.parse().unwrap_or(8);
                    side.pawns = CountSpec::Range { min, max };
                } else {
                    let count = val.parse().unwrap_or(0);
                    side.pawns = CountSpec::Exact(count);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_catalog_loading() {
        let catalog = EndgameCatalog::default_catalog();
        assert_eq!(catalog.version, 2);
        assert_eq!(
            catalog.features.len(),
            47,
            "Default catalog should have exactly 47 features"
        );
        assert_eq!(catalog.id_to_bit.len(), 47);
        assert_eq!(catalog.bit_to_feature.len(), 47);

        // Check essential features
        assert!(catalog.id_to_bit.contains_key("END_PAWN_KP_K"));
        assert_eq!(*catalog.id_to_bit.get("END_PAWN_KP_K").unwrap(), 0);
        assert!(catalog.id_to_bit.contains_key("END_ROOK_R_R"));
        assert_eq!(*catalog.id_to_bit.get("END_ROOK_R_R").unwrap(), 5);
        assert!(catalog
            .id_to_bit
            .contains_key("END_QUEEN_ROOK_VS_QUEEN_ROOK"));
        assert_eq!(
            *catalog
                .id_to_bit
                .get("END_QUEEN_ROOK_VS_QUEEN_ROOK")
                .unwrap(),
            46
        );
    }
}
