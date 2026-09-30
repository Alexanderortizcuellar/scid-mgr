use chess_scid_rw::entry::IndexEntry;
use chess_scid_rw::names::NameTables;
use cql_lang::evaluator::{GameSearchEvaluator, QueryMatchResult};
use cql_lang::header::{compare_date, compare_numeric, compare_string};
use cql_lang::query::{HeaderPredicate, SearchQuery};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::db::result_code_to_str;
use crate::pgn_db::{CompactPgnRecord, PgnNameTables};
use crate::position_search::decode_extra_tags;

/// Result record for a matched game in a database
#[derive(Debug, Clone, PartialEq)]
pub struct ScidMatchResult {
    pub game_id: usize,
    pub match_details: QueryMatchResult,
}

/// Native adapter for executing search queries on SCID binary databases
pub struct ScidSearchAdapter;

impl ScidSearchAdapter {
    /// Evaluate a SearchQuery on a SCID binary game record
    pub fn evaluate_scid_game(
        query: &SearchQuery,
        entry: &IndexEntry,
        names: &NameTables,
        blob: &[u8],
    ) -> QueryMatchResult {
        // Fast in-memory check without header_map allocation
        if let Some(false) = quick_check_entry_headers(query, entry, names) {
            return QueryMatchResult::default();
        }

        let has_headers = query.has_header_predicates();
        let mut header_map = HashMap::new();

        if has_headers {
            if let SearchQuery::Header(ref pred) = query {
                if let Some(is_match) = matches_scid_entry(pred, entry, names) {
                    return QueryMatchResult {
                        is_match,
                        matching_plies: if is_match { vec![0] } else { Vec::new() },
                        match_count: if is_match { 1 } else { 0 },
                    };
                }
            }

            header_map.insert(
                "White".to_string(),
                names.player(entry.white_id).to_string(),
            );
            header_map.insert(
                "Black".to_string(),
                names.player(entry.black_id).to_string(),
            );
            header_map.insert("Event".to_string(), names.event(entry.event_id).to_string());
            header_map.insert("Site".to_string(), names.site(entry.site_id).to_string());
            header_map.insert(
                "Date".to_string(),
                chess_scid_rw::dates::date_to_pgn(entry.date),
            );
            header_map.insert(
                "Result".to_string(),
                result_code_to_str(entry.result).to_string(),
            );
            if let Some(eco) = chess_scid_rw::eco::eco_to_string(entry.eco_code) {
                header_map.insert("ECO".to_string(), eco);
            }
            if entry.white_elo > 0 {
                header_map.insert("WhiteElo".to_string(), entry.white_elo.to_string());
            }
            if entry.black_elo > 0 {
                header_map.insert("BlackElo".to_string(), entry.black_elo.to_string());
            }

            // Decode extra tags from blob
            let mut tag_cursor = 0;
            decode_extra_tags(blob, &mut tag_cursor, &mut header_map);
        }

        // Evaluate pure header queries directly
        if let SearchQuery::Header(ref pred) = query {
            let is_match = cql_lang::header::HeaderMatcher::matches(pred, &header_map);
            return QueryMatchResult {
                is_match,
                matching_plies: if is_match { vec![0] } else { Vec::new() },
                match_count: if is_match { 1 } else { 0 },
            };
        }

        if let Ok(pgn) = chess_scid_rw::pgn_build::build_pgn(entry, names, blob) {
            GameSearchEvaluator::evaluate_pgn(query, &pgn)
        } else {
            QueryMatchResult::default()
        }
    }

    /// Parallel batch evaluation across SCID database records
    pub fn evaluate_scid_batch(
        query: &SearchQuery,
        entries: &[IndexEntry],
        names: &NameTables,
        blobs: &[Vec<u8>],
    ) -> Vec<ScidMatchResult> {
        entries
            .par_iter()
            .zip(blobs.par_iter())
            .enumerate()
            .filter_map(|(idx, (entry, blob))| {
                let match_details = Self::evaluate_scid_game(query, entry, names, blob);
                if match_details.is_match {
                    Some(ScidMatchResult {
                        game_id: idx,
                        match_details,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Execute a search across all entries with progress reporting
    pub fn search_parallel_with_progress<F, B, BRef>(
        query: &SearchQuery,
        entries: &[IndexEntry],
        names: &NameTables,
        get_blob: B,
        progress: F,
    ) -> Vec<ScidMatchResult>
    where
        F: Fn(usize, usize, usize) + Sync,
        B: Fn(&IndexEntry) -> Option<BRef> + Sync,
        BRef: AsRef<[u8]>,
    {
        Self::search_parallel_range_with_progress(
            query,
            entries,
            names,
            0,
            entries.len(),
            get_blob,
            progress,
        )
    }

    /// Execute a search across a sub-range of entries with progress reporting
    pub fn search_parallel_range_with_progress<F, B, BRef>(
        query: &SearchQuery,
        entries: &[IndexEntry],
        names: &NameTables,
        start_game: usize,
        end_game: usize,
        get_blob: B,
        progress: F,
    ) -> Vec<ScidMatchResult>
    where
        F: Fn(usize, usize, usize) + Sync,
        B: Fn(&IndexEntry) -> Option<BRef> + Sync,
        BRef: AsRef<[u8]>,
    {
        let end_game = end_game.min(entries.len());
        if start_game >= end_game {
            return Vec::new();
        }

        let slice = &entries[start_game..end_game];
        let scanned = AtomicUsize::new(0);
        let matched = AtomicUsize::new(0);
        let total = slice.len();

        let results: Vec<ScidMatchResult> = slice
            .par_iter()
            .enumerate()
            .filter_map(|(relative_idx, entry)| {
                let s = scanned.fetch_add(1, Ordering::Relaxed) + 1;
                if s.is_multiple_of(5000) || s == total {
                    progress(s, total, matched.load(Ordering::Relaxed));
                }

                if entry.deleted {
                    return None;
                }

                let blob_opt = get_blob(entry);
                let blob = blob_opt.as_ref().map(|b| b.as_ref()).unwrap_or(&[]);
                let match_details = Self::evaluate_scid_game(query, entry, names, blob);

                if match_details.is_match {
                    matched.fetch_add(1, Ordering::Relaxed);
                    Some(ScidMatchResult {
                        game_id: start_game + relative_idx,
                        match_details,
                    })
                } else {
                    None
                }
            })
            .collect();

        progress(total, total, results.len());
        results
    }
}

pub fn matches_scid_entry(
    predicate: &HeaderPredicate,
    entry: &IndexEntry,
    names: &NameTables,
) -> Option<bool> {
    match predicate {
        HeaderPredicate::White {
            name,
            op,
            case_sensitive,
        } => {
            let white = names.player(entry.white_id);
            Some(compare_string(white, name, *op, *case_sensitive))
        }
        HeaderPredicate::Black {
            name,
            op,
            case_sensitive,
        } => {
            let black = names.player(entry.black_id);
            Some(compare_string(black, name, *op, *case_sensitive))
        }
        HeaderPredicate::Player {
            name,
            op,
            case_sensitive,
        } => {
            let white = names.player(entry.white_id);
            let black = names.player(entry.black_id);
            Some(
                compare_string(white, name, *op, *case_sensitive)
                    || compare_string(black, name, *op, *case_sensitive),
            )
        }
        HeaderPredicate::WhiteElo { op, value } => {
            let elo = if entry.white_elo > 0 {
                Some(entry.white_elo)
            } else {
                None
            };
            Some(compare_numeric(elo, Some(*value), *op))
        }
        HeaderPredicate::BlackElo { op, value } => {
            let elo = if entry.black_elo > 0 {
                Some(entry.black_elo)
            } else {
                None
            };
            Some(compare_numeric(elo, Some(*value), *op))
        }
        HeaderPredicate::AnyElo { op, value } => {
            let w_elo = if entry.white_elo > 0 {
                Some(entry.white_elo)
            } else {
                None
            };
            let b_elo = if entry.black_elo > 0 {
                Some(entry.black_elo)
            } else {
                None
            };
            Some(
                compare_numeric(w_elo, Some(*value), *op)
                    || compare_numeric(b_elo, Some(*value), *op),
            )
        }
        HeaderPredicate::AvgElo { op, value } => {
            if entry.white_elo > 0 && entry.black_elo > 0 {
                let avg = (entry.white_elo + entry.black_elo) / 2;
                Some(compare_numeric(Some(avg), Some(*value), *op))
            } else {
                Some(false)
            }
        }
        HeaderPredicate::EloDiff {
            op,
            value,
            absolute,
        } => {
            if entry.white_elo > 0 && entry.black_elo > 0 {
                let diff = if *absolute {
                    (entry.white_elo as i32 - entry.black_elo as i32).abs()
                } else {
                    entry.white_elo as i32 - entry.black_elo as i32
                };
                Some(compare_numeric(Some(diff), Some(*value), *op))
            } else {
                Some(false)
            }
        }
        HeaderPredicate::Result { expected } => {
            let actual = result_code_to_str(entry.result);
            if expected == "All" || (expected == "*" && actual.is_empty()) {
                Some(true)
            } else {
                Some(actual == expected)
            }
        }
        HeaderPredicate::Eco { code, op } => {
            let actual = chess_scid_rw::eco::eco_to_string(entry.eco_code).unwrap_or_default();
            Some(compare_string(&actual, code, *op, false))
        }
        HeaderPredicate::Date { op, value } => {
            let actual = chess_scid_rw::dates::date_to_pgn(entry.date);
            Some(compare_date(&actual, value, *op))
        }
        HeaderPredicate::Event {
            name,
            op,
            case_sensitive,
        } => {
            let event = names.event(entry.event_id);
            Some(compare_string(event, name, *op, *case_sensitive))
        }
        HeaderPredicate::Site {
            name,
            op,
            case_sensitive,
        } => {
            let site = names.site(entry.site_id);
            Some(compare_string(site, name, *op, *case_sensitive))
        }
        HeaderPredicate::Round { value } => {
            let actual = names.round(entry.round_id);
            Some(actual.trim() == value.trim())
        }
        _ => None,
    }
}

pub fn matches_pgn_entry(
    predicate: &HeaderPredicate,
    entry: &CompactPgnRecord,
    names: &PgnNameTables,
) -> Option<bool> {
    match predicate {
        HeaderPredicate::White {
            name,
            op,
            case_sensitive,
        } => {
            let white = names.player(entry.white_id);
            Some(compare_string(white, name, *op, *case_sensitive))
        }
        HeaderPredicate::Black {
            name,
            op,
            case_sensitive,
        } => {
            let black = names.player(entry.black_id);
            Some(compare_string(black, name, *op, *case_sensitive))
        }
        HeaderPredicate::Player {
            name,
            op,
            case_sensitive,
        } => {
            let white = names.player(entry.white_id);
            let black = names.player(entry.black_id);
            Some(
                compare_string(white, name, *op, *case_sensitive)
                    || compare_string(black, name, *op, *case_sensitive),
            )
        }
        HeaderPredicate::WhiteElo { op, value } => {
            let elo = if entry.white_elo > 0 {
                Some(entry.white_elo)
            } else {
                None
            };
            Some(compare_numeric(elo, Some(*value), *op))
        }
        HeaderPredicate::BlackElo { op, value } => {
            let elo = if entry.black_elo > 0 {
                Some(entry.black_elo)
            } else {
                None
            };
            Some(compare_numeric(elo, Some(*value), *op))
        }
        HeaderPredicate::AnyElo { op, value } => {
            let w_elo = if entry.white_elo > 0 {
                Some(entry.white_elo)
            } else {
                None
            };
            let b_elo = if entry.black_elo > 0 {
                Some(entry.black_elo)
            } else {
                None
            };
            Some(
                compare_numeric(w_elo, Some(*value), *op)
                    || compare_numeric(b_elo, Some(*value), *op),
            )
        }
        HeaderPredicate::Result { expected } => {
            let actual = entry.result_str();
            if expected == "All" || (expected == "*" && actual.is_empty()) {
                Some(true)
            } else {
                Some(actual == expected)
            }
        }
        HeaderPredicate::Eco { code, op } => {
            let actual = entry.eco_str();
            Some(compare_string(&actual, code, *op, false))
        }
        HeaderPredicate::Date { op, value } => {
            let actual = entry.date_str();
            Some(compare_date(&actual, value, *op))
        }
        HeaderPredicate::Event {
            name,
            op,
            case_sensitive,
        } => {
            let event = names.event(entry.event_id);
            Some(compare_string(event, name, *op, *case_sensitive))
        }
        HeaderPredicate::Site {
            name,
            op,
            case_sensitive,
        } => {
            let site = names.site(entry.site_id);
            Some(compare_string(site, name, *op, *case_sensitive))
        }
        _ => None,
    }
}

pub fn quick_check_entry_headers(
    query: &SearchQuery,
    entry: &IndexEntry,
    names: &NameTables,
) -> Option<bool> {
    match query {
        SearchQuery::Header(pred) => matches_scid_entry(pred, entry, names),
        SearchQuery::And(sub_queries) => {
            let mut all_true = true;
            for q in sub_queries {
                match quick_check_entry_headers(q, entry, names) {
                    Some(false) => return Some(false),
                    Some(true) => {}
                    None => all_true = false,
                }
            }
            if all_true && !sub_queries.is_empty() {
                Some(true)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn quick_check_pgn_entry_headers(
    query: &SearchQuery,
    entry: &CompactPgnRecord,
    names: &PgnNameTables,
) -> Option<bool> {
    match query {
        SearchQuery::Header(pred) => matches_pgn_entry(pred, entry, names),
        SearchQuery::And(sub_queries) => {
            let mut all_true = true;
            for q in sub_queries {
                match quick_check_pgn_entry_headers(q, entry, names) {
                    Some(false) => return Some(false),
                    Some(true) => {}
                    None => all_true = false,
                }
            }
            if all_true && !sub_queries.is_empty() {
                Some(true)
            } else {
                None
            }
        }
        _ => None,
    }
}
