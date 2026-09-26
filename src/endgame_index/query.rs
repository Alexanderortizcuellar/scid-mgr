use anyhow::Result;
use std::collections::HashMap;

use crate::endgame_index::catalog::EndgameCatalog;
use crate::endgame_index::serializer::MmapFeatureIndex;
use crate::endgame_index::types::{
    CategoryPopularity, EndgamePopularityReport, FeaturePopularity, FeatureQueryReport,
};

pub struct EndgameQueryEngine;

impl EndgameQueryEngine {
    pub fn calculate_popularity(
        index: &MmapFeatureIndex,
        catalog: &EndgameCatalog,
        db_path: &str,
        candidate_games: Option<&[u32]>,
        fen: Option<&str>,
        get_game_result: Option<&dyn Fn(u32) -> u8>, // 1: 1-0 (White win), 2: 0-1 (Black win), 3: 1/2-1/2 (Draw)
    ) -> Result<EndgamePopularityReport> {
        let total_db_games = index.game_count() as usize;
        let bit_count = index.header().endgame_bit_count.min(64) as usize;

        let mut feature_counts = vec![0usize; bit_count];
        let mut feature_w_wins = vec![0usize; bit_count];
        let mut feature_draws = vec![0usize; bit_count];
        let mut feature_b_wins = vec![0usize; bit_count];

        let reaching_games_count = if let Some(ids) = candidate_games {
            for &gid in ids {
                if let Some(rec) = index.get_record(gid) {
                    let res = get_game_result.map(|f| f(gid)).unwrap_or(0);
                    for bit in 0..bit_count {
                        if rec.has_endgame_bit(bit as u8) {
                            feature_counts[bit] += 1;
                            match res {
                                1 => feature_w_wins[bit] += 1,
                                2 => feature_b_wins[bit] += 1,
                                3 => feature_draws[bit] += 1,
                                _ => {}
                            }
                        }
                    }
                }
            }
            ids.len()
        } else {
            for gid in 0..total_db_games as u32 {
                if let Some(rec) = index.get_record(gid) {
                    let res = get_game_result.map(|f| f(gid)).unwrap_or(0);
                    for bit in 0..bit_count {
                        if rec.has_endgame_bit(bit as u8) {
                            feature_counts[bit] += 1;
                            match res {
                                1 => feature_w_wins[bit] += 1,
                                2 => feature_b_wins[bit] += 1,
                                3 => feature_draws[bit] += 1,
                                _ => {}
                            }
                        }
                    }
                }
            }
            total_db_games
        };

        let denom = reaching_games_count.max(1) as f64;

        let mut features_pop = Vec::new();
        let mut cat_map: HashMap<String, Vec<FeaturePopularity>> = HashMap::new();

        for feat in &catalog.features {
            let bit = feat.bit as usize;
            let count = if bit < feature_counts.len() {
                feature_counts[bit]
            } else {
                0
            };
            let w_win = if bit < feature_w_wins.len() {
                feature_w_wins[bit]
            } else {
                0
            };
            let drw = if bit < feature_draws.len() {
                feature_draws[bit]
            } else {
                0
            };
            let b_win = if bit < feature_b_wins.len() {
                feature_b_wins[bit]
            } else {
                0
            };

            let pct = (count as f64 / denom) * 100.0;

            let pop = FeaturePopularity {
                id: feat.id.clone(),
                bit: feat.bit,
                name: feat.name.clone(),
                short_name: feat.short_name.clone(),
                gbr_code: feat.gbr_code.clone(),
                category_id: feat.category_id.clone(),
                game_count: count,
                percentage: pct,
                white_wins: w_win,
                draws: drw,
                black_wins: b_win,
            };

            cat_map
                .entry(feat.category_id.clone())
                .or_default()
                .push(pop.clone());
            features_pop.push(pop);
        }

        // Build category aggregations in defined sequence
        let cat_names: &[(&str, &str)] = &[
            ("PAWN", "Pawn Endgames"),
            ("ROOK", "Rook Endgames"),
            ("BISHOP", "Bishop Endgames"),
            ("KNIGHT", "Knight Endgames"),
            ("MINOR_MIXED", "Bishop vs Knight Endgames"),
            ("QUEEN", "Queen Endgames"),
            ("ROOK_VS_MINOR", "Rook vs Minor / Major Imbalances"),
            ("QUEEN_VS_PIECES", "Queen vs Multiple Pieces"),
        ];

        let mut categories = Vec::new();
        for &(cat_id, cat_name) in cat_names {
            if let Some(feats) = cat_map.remove(cat_id) {
                // Category total = sum of games matching any feature in category
                let cat_games: usize = feats.iter().map(|f| f.game_count).sum();
                let cat_pct = (cat_games as f64 / denom) * 100.0;

                categories.push(CategoryPopularity {
                    category_id: cat_id.to_string(),
                    name: cat_name.to_string(),
                    total_games: cat_games,
                    percentage: cat_pct,
                    features: feats,
                });
            }
        }

        // Remaining uncategorized
        for (cat_id, feats) in cat_map {
            let cat_games: usize = feats.iter().map(|f| f.game_count).sum();
            let cat_pct = (cat_games as f64 / denom) * 100.0;
            categories.push(CategoryPopularity {
                category_id: cat_id.clone(),
                name: cat_id,
                total_games: cat_games,
                percentage: cat_pct,
                features: feats,
            });
        }

        Ok(EndgamePopularityReport {
            db_path: db_path.to_string(),
            total_db_games,
            games_reaching_position: reaching_games_count,
            position_filtered: candidate_games.is_some(),
            fen: fen.map(|s| s.to_string()),
            categories,
            features: features_pop,
        })
    }

    pub fn query_feature(
        index: &MmapFeatureIndex,
        catalog: &EndgameCatalog,
        feature_id_or_bit: &str,
        max_samples: usize,
    ) -> Result<FeatureQueryReport> {
        let feat = if let Ok(bit) = feature_id_or_bit.parse::<u8>() {
            catalog
                .bit_to_feature
                .get(&bit)
                .ok_or_else(|| anyhow::anyhow!("Unknown endgame bit {}", bit))?
        } else {
            let bit = catalog.id_to_bit.get(feature_id_or_bit).ok_or_else(|| {
                anyhow::anyhow!("Unknown endgame feature ID '{}'", feature_id_or_bit)
            })?;
            catalog.bit_to_feature.get(bit).unwrap()
        };

        let target_bit = feat.bit;
        let mask = 1u64 << target_bit;
        let matching_ids = index.find_games_with_endgame_mask(mask);
        let total_db_games = index.game_count() as usize;
        let count = matching_ids.len();
        let pct = (count as f64 / total_db_games.max(1) as f64) * 100.0;

        let samples = if max_samples > 0 {
            matching_ids.into_iter().take(max_samples).collect()
        } else {
            Vec::new()
        };

        Ok(FeatureQueryReport {
            feature_id: feat.id.clone(),
            bit: feat.bit,
            name: feat.name.clone(),
            short_name: feat.short_name.clone(),
            gbr_code: feat.gbr_code.clone(),
            category_id: feat.category_id.clone(),
            total_db_games,
            matching_games_count: count,
            percentage: pct,
            sample_game_ids: samples,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endgame_index::model::GameFeatureRecord;
    use crate::endgame_index::serializer::FeatureIndexWriter;
    use tempfile::NamedTempFile;

    #[test]
    fn test_popularity_report_and_query_feature() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        let catalog = EndgameCatalog::default_catalog();

        let mut writer = FeatureIndexWriter::create(&path, catalog.version, 47, 0, 0).unwrap();

        // Game 0: KP_K (bit 0) - 1-0 (White win, code 1)
        let mut rec0 = GameFeatureRecord::new();
        rec0.set_endgame_bit(0);

        // Game 1: KP_K (bit 0) & R_R (bit 5) - 0-1 (Black win, code 2)
        let mut rec1 = GameFeatureRecord::new();
        rec1.set_endgame_bit(0);
        rec1.set_endgame_bit(5);

        // Game 2: R_R (bit 5) - 1/2-1/2 (Draw, code 3)
        let mut rec2 = GameFeatureRecord::new();
        rec2.set_endgame_bit(5);

        writer.write_record(&rec0).unwrap();
        writer.write_record(&rec1).unwrap();
        writer.write_record(&rec2).unwrap();
        writer.finish().unwrap();

        let mmap_idx = MmapFeatureIndex::open(&path).unwrap();

        let results_fn = |gid: u32| match gid {
            0 => 1, // 1-0
            1 => 2, // 0-1
            2 => 3, // 1/2-1/2
            _ => 0,
        };

        let report = EndgameQueryEngine::calculate_popularity(
            &mmap_idx,
            &catalog,
            "test_db",
            None,
            None,
            Some(&results_fn),
        )
        .unwrap();

        assert_eq!(report.total_db_games, 3);
        assert_eq!(report.games_reaching_position, 3);

        // Feature 0: KP_K -> 2 games, 1 W win, 1 B win, 0 Draw
        let f0 = report.features.iter().find(|f| f.bit == 0).unwrap();
        assert_eq!(f0.game_count, 2);
        assert_eq!(f0.white_wins, 1);
        assert_eq!(f0.black_wins, 1);
        assert_eq!(f0.draws, 0);

        // Feature 5: R_R -> 2 games, 0 W win, 1 B win, 1 Draw
        let f5 = report.features.iter().find(|f| f.bit == 5).unwrap();
        assert_eq!(f5.game_count, 2);
        assert_eq!(f5.white_wins, 0);
        assert_eq!(f5.black_wins, 1);
        assert_eq!(f5.draws, 1);

        // Query single feature report
        let q_rep =
            EndgameQueryEngine::query_feature(&mmap_idx, &catalog, "END_PAWN_KP_K", 10).unwrap();
        assert_eq!(q_rep.matching_games_count, 2);
        assert_eq!(q_rep.sample_game_ids, vec![0, 1]);
    }
}
