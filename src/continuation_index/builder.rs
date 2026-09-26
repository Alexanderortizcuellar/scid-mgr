use anyhow::{Context, Result};
use memmap2::Mmap;
use pgn_reader::{BufferedReader, SanPlus, Skip, Visitor};
use rayon::prelude::*;
use shakmaty::fen::Fen;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{CastlingMode, Chess, EnPassantMode, Position};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use super::types::{
    HotEdge, HotGraphHeader, HotGraphMetadata, HotHashEntry, HotNode, NodeId, PackedMove,
    HEADER_SIZE, HOT_GRAPH_MAGIC, HOT_GRAPH_VERSION, NO_NODE, NUM_BUILDER_STRIPES,
};
use crate::db::ScidDatabaseWrapper;

#[derive(Debug, Clone, Default)]
struct BuilderEdge {
    packed_move: u16,
    target_hash: u64,
    total_games: u32,
    white_wins: u32,
    draws: u32,
    black_wins: u32,
}

#[derive(Debug, Clone, Default)]
struct BuilderNode {
    total_games: u32,
    white_wins: u32,
    draws: u32,
    black_wins: u32,
    edges: Vec<BuilderEdge>,
}

#[derive(Debug, Clone)]
pub struct HotGraphBuildConfig {
    pub max_ply: usize,
    pub min_games: usize,
}

impl Default for HotGraphBuildConfig {
    fn default() -> Self {
        Self {
            max_ply: 24,
            min_games: 1,
        }
    }
}

pub struct StripedHotGraphBuilder {
    config: HotGraphBuildConfig,
    stripes: Vec<Mutex<HashMap<u64, BuilderNode>>>,
    unique_counter: std::sync::atomic::AtomicUsize,
}

impl StripedHotGraphBuilder {
    pub fn new(config: HotGraphBuildConfig) -> Self {
        let mut stripes = Vec::with_capacity(NUM_BUILDER_STRIPES);
        for _ in 0..NUM_BUILDER_STRIPES {
            stripes.push(Mutex::new(HashMap::with_capacity(2048)));
        }
        Self {
            config,
            stripes,
            unique_counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    #[inline]
    fn stripe_index(hash: u64) -> usize {
        let mut x = hash;
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51afd7ed558ccd);
        x ^= x >> 33;
        (x as usize) & (NUM_BUILDER_STRIPES - 1)
    }

    pub fn total_nodes(&self) -> usize {
        self.unique_counter.load(Ordering::Relaxed)
    }

    pub fn record_game_transition(
        &self,
        from_hash: u64,
        packed_move: u16,
        to_hash: u64,
        w_win: u32,
        draw: u32,
        b_win: u32,
    ) {
        let idx = Self::stripe_index(from_hash);
        let mut map = self.stripes[idx].lock().unwrap();

        use std::collections::hash_map::Entry;
        let node = match map.entry(from_hash) {
            Entry::Occupied(occ) => occ.into_mut(),
            Entry::Vacant(vac) => {
                self.unique_counter.fetch_add(1, Ordering::Relaxed);
                vac.insert(BuilderNode::default())
            }
        };

        node.total_games += 1;
        node.white_wins += w_win;
        node.draws += draw;
        node.black_wins += b_win;

        if let Some(pos) = node.edges.iter().position(|e| e.packed_move == packed_move) {
            let edge = &mut node.edges[pos];
            edge.total_games += 1;
            edge.white_wins += w_win;
            edge.draws += draw;
            edge.black_wins += b_win;
            edge.target_hash = to_hash;
        } else {
            node.edges.push(BuilderEdge {
                packed_move,
                target_hash: to_hash,
                total_games: 1,
                white_wins: w_win,
                draws: draw,
                black_wins: b_win,
            });
        }
    }

    pub fn record_leaf_position(&self, hash: u64, w_win: u32, draw: u32, b_win: u32) {
        let idx = Self::stripe_index(hash);
        let mut map = self.stripes[idx].lock().unwrap();

        use std::collections::hash_map::Entry;
        let node = match map.entry(hash) {
            Entry::Occupied(occ) => occ.into_mut(),
            Entry::Vacant(vac) => {
                self.unique_counter.fetch_add(1, Ordering::Relaxed);
                vac.insert(BuilderNode::default())
            }
        };

        node.total_games += 1;
        node.white_wins += w_win;
        node.draws += draw;
        node.black_wins += b_win;
    }

    pub fn write_to_file<P: AsRef<Path>>(
        self,
        dest_path: P,
        db_game_count: u64,
        db_mtime_secs: u64,
        db_file_size: u64,
    ) -> Result<HotGraphMetadata> {
        let dest_p = dest_path.as_ref();
        let temp_path = dest_p.with_file_name(format!(
            "{}.tmp",
            dest_p.file_name().unwrap_or_default().to_string_lossy()
        ));
        let min_games_cutoff = self.config.min_games as u32;

        let mut combined_nodes: HashMap<u64, BuilderNode> = HashMap::new();
        for stripe in self.stripes {
            let map = stripe.into_inner().unwrap();
            for (hash, node) in map {
                if node.total_games >= min_games_cutoff {
                    combined_nodes.insert(hash, node);
                }
            }
        }

        let mut sorted_hashes: Vec<u64> = combined_nodes.keys().copied().collect();
        sorted_hashes.sort_unstable();

        let mut hash_to_node_id: HashMap<u64, NodeId> = HashMap::with_capacity(sorted_hashes.len());
        for (idx, &hash) in sorted_hashes.iter().enumerate() {
            hash_to_node_id.insert(hash, idx as NodeId);
        }

        let mut final_nodes: Vec<HotNode> = Vec::with_capacity(sorted_hashes.len());
        let mut final_edges: Vec<HotEdge> = Vec::new();
        let mut final_hash_entries: Vec<HotHashEntry> = Vec::with_capacity(sorted_hashes.len());

        for &hash in &sorted_hashes {
            let node_id = hash_to_node_id[&hash];
            let bnode = &combined_nodes[&hash];

            let first_edge = final_edges.len() as u32;
            let mut valid_edges: Vec<&BuilderEdge> = bnode
                .edges
                .iter()
                .filter(|e| e.total_games >= min_games_cutoff)
                .collect();

            valid_edges.sort_by_key(|a| std::cmp::Reverse(a.total_games));

            for e in &valid_edges {
                let target_node = hash_to_node_id
                    .get(&e.target_hash)
                    .copied()
                    .unwrap_or(NO_NODE);
                final_edges.push(HotEdge::new(
                    PackedMove(e.packed_move),
                    target_node,
                    e.total_games,
                    e.white_wins,
                    e.draws,
                    e.black_wins,
                ));
            }

            final_nodes.push(HotNode::new(
                first_edge,
                valid_edges.len() as u16,
                bnode.total_games,
                bnode.white_wins,
                bnode.draws,
                bnode.black_wins,
            ));

            final_hash_entries.push(HotHashEntry { hash, node_id });
        }

        let created_timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        {
            let file = File::create(&temp_path)
                .with_context(|| format!("Failed to create hot graph at {:?}", temp_path))?;
            let mut writer = BufWriter::new(file);

            let nodes_offset = HEADER_SIZE as u64;
            let edges_offset =
                nodes_offset + (final_nodes.len() * std::mem::size_of::<HotNode>()) as u64;
            let hashes_offset =
                edges_offset + (final_edges.len() * std::mem::size_of::<HotEdge>()) as u64;

            let header = HotGraphHeader {
                magic: *HOT_GRAPH_MAGIC,
                version: HOT_GRAPH_VERSION,
                flags: 0,
                db_mtime_secs,
                db_file_size,
                db_game_count,
                max_ply: self.config.max_ply as u32,
                min_games: self.config.min_games as u32,
                node_count: final_nodes.len() as u32,
                edge_count: final_edges.len() as u32,
                hash_count: final_hash_entries.len() as u32,
                _reserved: 0,
                nodes_offset,
                edges_offset,
                hashes_offset,
                created_timestamp,
            };

            header.write_to(&mut writer)?;

            for node in &final_nodes {
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        node as *const HotNode as *const u8,
                        std::mem::size_of::<HotNode>(),
                    )
                };
                writer.write_all(bytes)?;
            }

            for edge in &final_edges {
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        edge as *const HotEdge as *const u8,
                        std::mem::size_of::<HotEdge>(),
                    )
                };
                writer.write_all(bytes)?;
            }

            for hash_entry in &final_hash_entries {
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        hash_entry as *const HotHashEntry as *const u8,
                        std::mem::size_of::<HotHashEntry>(),
                    )
                };
                writer.write_all(bytes)?;
            }

            writer.flush()?;
        }

        if dest_p.exists() {
            let _ = std::fs::remove_file(dest_p);
        }
        std::fs::rename(&temp_path, dest_p).with_context(|| {
            format!(
                "Failed to rename {} to {}",
                temp_path.display(),
                dest_p.display()
            )
        })?;

        Ok(HotGraphMetadata {
            max_ply: self.config.max_ply as u32,
            min_games: self.config.min_games as u32,
            db_game_count,
            db_mtime_secs,
            db_file_size,
            created_timestamp,
            node_count: final_nodes.len() as u32,
            edge_count: final_edges.len() as u32,
            hash_count: final_hash_entries.len() as u32,
        })
    }
}

pub fn build_for_pgn<P: AsRef<Path>>(
    pgn_path: P,
    dest_path: P,
    config: HotGraphBuildConfig,
) -> Result<HotGraphMetadata> {
    build_for_pgn_direct(pgn_path, dest_path, config, None, |_, _, _| {})
}

pub fn build_for_pgn_direct<P1: AsRef<Path>, P2: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
    pgn_path: P1,
    dest_path: P2,
    config: HotGraphBuildConfig,
    threads: Option<usize>,
    progress: F,
) -> Result<HotGraphMetadata> {
    let pgn_p = pgn_path.as_ref();
    let dest_p = dest_path.as_ref();
    let pgn_file =
        File::open(pgn_p).with_context(|| format!("Failed to open PGN at {:?}", pgn_p))?;
    let metadata = pgn_file.metadata()?;
    let db_mtime_secs = metadata
        .modified()
        .map(|t| {
            t.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        })
        .unwrap_or(0);
    let db_file_size = metadata.len();

    let mmap = unsafe { Mmap::map(&pgn_file)? };
    let builder = StripedHotGraphBuilder::new(config.clone());

    let num_threads = threads.unwrap_or_else(rayon::current_num_threads).max(1);
    let chunk_size = (mmap.len() / num_threads).max(64 * 1024);
    let mut chunk_offsets = Vec::new();
    let mut curr = 0;

    while curr < mmap.len() {
        let mut next = (curr + chunk_size).min(mmap.len());
        if next < mmap.len() {
            while next < mmap.len() && mmap[next] != b'\n' {
                next += 1;
            }
            if next < mmap.len() {
                next += 1;
            }
        }
        chunk_offsets.push((curr, next));
        curr = next;
    }

    let game_counter = AtomicU64::new(0);

    let run_index = || {
        chunk_offsets.par_iter().for_each(|&(start, end)| {
            let chunk = &mmap[start..end];
            let mut reader = BufferedReader::new_cursor(chunk);
            let mut visitor = HotGraphPgnVisitor::new(&builder, config.max_ply);

            while let Ok(Some(())) = reader.read_game(&mut visitor) {
                let scanned = game_counter.fetch_add(1, Ordering::Relaxed) + 1;
                if scanned.is_multiple_of(1000) {
                    progress(scanned as usize, 0, builder.total_nodes());
                }
                visitor.reset();
            }
        });
    };

    if let Some(t) = threads {
        if t > 0 {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(t).build()?;
            pool.install(run_index);
        } else {
            run_index();
        }
    } else {
        run_index();
    }

    let db_game_count = game_counter.load(Ordering::Relaxed);
    progress(
        db_game_count as usize,
        db_game_count as usize,
        builder.total_nodes(),
    );
    builder.write_to_file(dest_p, db_game_count, db_mtime_secs, db_file_size)
}

pub fn build_for_scid<P: AsRef<Path>>(
    db_stem: P,
    dest_path: P,
    config: HotGraphBuildConfig,
) -> Result<HotGraphMetadata> {
    let db = ScidDatabaseWrapper::open(db_stem.as_ref())?;
    build_for_scid_direct(
        db.index_path(),
        db.entries(),
        db.games_path(),
        dest_path.as_ref(),
        config,
        None,
        |_, _, _| {},
    )
}

pub fn build_for_scid_direct<
    P1: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
    F: Fn(usize, usize, usize) + Sync,
>(
    db_path: P1,
    entries: &[chess_scid_rw::entry::IndexEntry],
    games_path: P2,
    dest_path: P3,
    config: HotGraphBuildConfig,
    threads: Option<usize>,
    progress: F,
) -> Result<HotGraphMetadata> {
    let db_p = db_path.as_ref();
    let dest_p = dest_path.as_ref();
    let file = File::open(games_path.as_ref()).with_context(|| {
        format!(
            "Failed to open games file: {}",
            games_path.as_ref().display()
        )
    })?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };

    let total_games = entries.len();
    let chunk_size = 5000;
    let scanned_counter = std::sync::atomic::AtomicUsize::new(0);
    let builder = StripedHotGraphBuilder::new(config.clone());
    let max_ply = config.max_ply;

    let run_index = || {
        (0..total_games)
            .into_par_iter()
            .step_by(chunk_size)
            .for_each(|start_idx| {
                let end_idx = (start_idx + chunk_size).min(total_games);

                #[allow(clippy::needless_range_loop)]
                for game_id in start_idx..end_idx {
                    let entry = &entries[game_id];
                    if entry.deleted {
                        continue;
                    }

                    let start = entry.offset as usize;
                    let end = start + entry.length as usize;
                    if end > mmap.len() || start >= end {
                        continue;
                    }

                    let blob = &mmap[start..end];
                    let mut cursor = 0;

                    let mut pos =
                        match crate::position_search::parse_start_position(blob, &mut cursor) {
                            Some(p) => p,
                            None => continue,
                        };

                    let (w_win, draw, b_win) = match entry.result {
                        1 => (1, 0, 0),
                        2 => (0, 0, 1),
                        3 => (0, 1, 0),
                        _ => (0, 0, 0),
                    };

                    let mut slots = crate::position_search::standard_piece_slots();
                    let mut counts = [16usize, 16usize];
                    let mut ply = 0;

                    while cursor < blob.len() && ply < max_ply {
                        let from_hash_val: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
                        let from_hash = from_hash_val.0;

                        let b = blob[cursor];
                        cursor += 1;

                        if b == 15 {
                            builder.record_leaf_position(from_hash, w_win, draw, b_win);
                            break;
                        }
                        if b == 11 {
                            cursor += 1;
                            continue;
                        }
                        if b == 12 || b == 13 || b == 14 {
                            continue;
                        }

                        let (mv, piece_idx, to_sq, is_k, is_q, cap_sq) =
                            match crate::position_search::decode_raw_move(
                                b,
                                &mut cursor,
                                blob,
                                &pos,
                                &slots,
                                &counts,
                            ) {
                                Some(res) => res,
                                None => {
                                    builder.record_leaf_position(from_hash, w_win, draw, b_win);
                                    break;
                                }
                            };

                        let packed = PackedMove::from(&mv);
                        let mut next_pos = pos.clone();
                        next_pos.play_unchecked(&mv);
                        let to_hash_val: Zobrist64 = next_pos.zobrist_hash(EnPassantMode::Legal);

                        builder.record_game_transition(
                            from_hash,
                            packed.0,
                            to_hash_val.0,
                            w_win,
                            draw,
                            b_win,
                        );

                        let side_idx = usize::from(pos.turn() == shakmaty::Color::Black);
                        crate::position_search::update_slots_on_move(
                            &mut slots,
                            &mut counts,
                            side_idx,
                            piece_idx,
                            to_sq,
                            is_k,
                            is_q,
                            cap_sq,
                        );

                        pos = next_pos;
                        ply += 1;
                    }

                    if ply == max_ply {
                        let final_hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
                        builder.record_leaf_position(final_hash.0, w_win, draw, b_win);
                    }
                }

                let current_scanned = scanned_counter
                    .fetch_add(end_idx - start_idx, Ordering::Relaxed)
                    + (end_idx - start_idx);
                progress(current_scanned, total_games, builder.total_nodes());
            });
    };

    if let Some(t) = threads {
        if t > 0 {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(t).build()?;
            pool.install(run_index);
        } else {
            run_index();
        }
    } else {
        run_index();
    }

    let db_metadata = std::fs::metadata(db_p)?;
    let db_mtime_secs = db_metadata
        .modified()
        .map(|t| {
            t.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        })
        .unwrap_or(0);
    let db_file_size = db_metadata.len();
    let db_game_count = entries.iter().filter(|e| !e.deleted).count() as u64;

    builder.write_to_file(dest_p, db_game_count, db_mtime_secs, db_file_size)
}

struct HotGraphPgnVisitor<'a> {
    builder: &'a StripedHotGraphBuilder,
    max_ply: usize,
    pos: Chess,
    ply: usize,
    w_win: u32,
    draw: u32,
    b_win: u32,
}

impl<'a> HotGraphPgnVisitor<'a> {
    fn new(builder: &'a StripedHotGraphBuilder, max_ply: usize) -> Self {
        Self {
            builder,
            max_ply,
            pos: Chess::default(),
            ply: 0,
            w_win: 0,
            draw: 0,
            b_win: 0,
        }
    }

    fn reset(&mut self) {
        self.pos = Chess::default();
        self.ply = 0;
        self.w_win = 0;
        self.draw = 0;
        self.b_win = 0;
    }
}

impl<'a> Visitor for HotGraphPgnVisitor<'a> {
    type Result = ();

    fn header(&mut self, key: &[u8], value: pgn_reader::RawHeader<'_>) {
        if key == b"Result" {
            let val = value.as_bytes();
            if val == b"1-0" {
                self.w_win = 1;
            } else if val == b"0-1" {
                self.b_win = 1;
            } else if val == b"1/2-1/2" {
                self.draw = 1;
            }
        } else if key == b"FEN" {
            if let Ok(fen_str) = std::str::from_utf8(value.as_bytes()) {
                if let Ok(fen) = fen_str.parse::<Fen>() {
                    if let Ok(custom_pos) = fen.into_position(CastlingMode::Standard) {
                        self.pos = custom_pos;
                    }
                }
            }
        }
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // Mainline only
    }

    fn san(&mut self, san: SanPlus) {
        if self.ply >= self.max_ply {
            return;
        }

        let from_hash: Zobrist64 = self.pos.zobrist_hash(EnPassantMode::Legal);

        if let Ok(m) = san.san.to_move(&self.pos) {
            let packed = PackedMove::from(&m);
            let mut next_pos = self.pos.clone();
            next_pos.play_unchecked(&m);
            let to_hash: Zobrist64 = next_pos.zobrist_hash(EnPassantMode::Legal);

            self.builder.record_game_transition(
                from_hash.0,
                packed.0,
                to_hash.0,
                self.w_win,
                self.draw,
                self.b_win,
            );

            self.pos = next_pos;
            self.ply += 1;
        }
    }

    fn end_game(&mut self) -> Self::Result {
        let final_hash: Zobrist64 = self.pos.zobrist_hash(EnPassantMode::Legal);
        self.builder
            .record_leaf_position(final_hash.0, self.w_win, self.draw, self.b_win);
    }
}
