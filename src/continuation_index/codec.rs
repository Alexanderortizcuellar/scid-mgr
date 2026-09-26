use anyhow::{bail, Context, Result};
use memmap2::Mmap;
use shakmaty::{Chess, Color, Position};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::types::{HotEdge, HotGraphHeader, HotHashEntry, HotNode, NodeId, HEADER_SIZE};

/// Format continuation moves into standard chess notation (e.g. `1. e4 e5 2. Nf3 Nc6` or `3... a6 4. Ba4 Nf6`)
pub fn format_continuation_moves(
    start_pos: &Chess,
    start_fullmove: u32,
    moves_san: &[String],
) -> String {
    let mut result = String::new();
    let mut turn = start_pos.turn();
    let mut move_number = start_fullmove;

    for (i, m) in moves_san.iter().enumerate() {
        if i == 0 {
            if turn == Color::Black {
                result.push_str(&format!("{}... {}", move_number, m));
            } else {
                result.push_str(&format!("{}. {}", move_number, m));
            }
        } else {
            result.push(' ');
            if turn == Color::White {
                move_number += 1;
                result.push_str(&format!("{}. {}", move_number, m));
            } else {
                result.push_str(m);
            }
        }
        turn = !turn;
    }

    result
}

pub fn parse_fen_fullmove(fen_str: &str) -> u32 {
    let parts: Vec<&str> = fen_str.split_whitespace().collect();
    if parts.len() >= 6 {
        parts[5].parse::<u32>().unwrap_or(1).max(1)
    } else {
        1
    }
}

pub fn resolve_companion_hot_path<P: AsRef<Path>>(db_path: P) -> PathBuf {
    let p = db_path.as_ref();
    let path_str = p.to_string_lossy();
    let lower = path_str.to_lowercase();
    if lower.ends_with(".pgn") {
        let direct = PathBuf::from(format!("{}.hot.idx", path_str));
        if direct.exists() {
            return direct;
        }
        let stem = p.with_extension("hot.idx");
        if stem.exists() {
            return stem;
        }
        direct
    } else {
        let direct = PathBuf::from(format!("{}.hot.idx", path_str));
        if direct.exists() {
            return direct;
        }
        p.with_extension("hot.idx")
    }
}

/// Zero-copy memory-mapped hot position graph
pub struct MmapHotGraph {
    _mmap: Arc<Mmap>,
    pub header: HotGraphHeader,
    nodes_ptr: *const HotNode,
    edges_ptr: *const HotEdge,
    hashes_ptr: *const HotHashEntry,
    node_count: usize,
    edge_count: usize,
    hash_count: usize,
}

unsafe impl Send for MmapHotGraph {}
unsafe impl Sync for MmapHotGraph {}

impl MmapHotGraph {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open hot graph at {:?}", path.as_ref()))?;
        let mmap = unsafe { Mmap::map(&file)? };

        if mmap.len() < HEADER_SIZE {
            bail!("Hot graph file too small: {} bytes", mmap.len());
        }

        let header = HotGraphHeader::read_from_slice(&mmap[0..HEADER_SIZE])?;

        let nodes_offset = header.nodes_offset as usize;
        let edges_offset = header.edges_offset as usize;
        let hashes_offset = header.hashes_offset as usize;

        let node_count = header.node_count as usize;
        let edge_count = header.edge_count as usize;
        let hash_count = header.hash_count as usize;

        let nodes_end = nodes_offset + node_count * std::mem::size_of::<HotNode>();
        let edges_end = edges_offset + edge_count * std::mem::size_of::<HotEdge>();
        let hashes_end = hashes_offset + hash_count * std::mem::size_of::<HotHashEntry>();

        if nodes_end > mmap.len() || edges_end > mmap.len() || hashes_end > mmap.len() {
            bail!("Corrupt hot graph file: offsets exceed file length");
        }

        let nodes_ptr = mmap.as_ptr().wrapping_add(nodes_offset) as *const HotNode;
        let edges_ptr = mmap.as_ptr().wrapping_add(edges_offset) as *const HotEdge;
        let hashes_ptr = mmap.as_ptr().wrapping_add(hashes_offset) as *const HotHashEntry;

        Ok(Self {
            _mmap: Arc::new(mmap),
            header,
            nodes_ptr,
            edges_ptr,
            hashes_ptr,
            node_count,
            edge_count,
            hash_count,
        })
    }

    #[inline]
    pub fn get_node_id(&self, hash: u64) -> Option<NodeId> {
        if self.hash_count == 0 {
            return None;
        }
        let hashes = unsafe { std::slice::from_raw_parts(self.hashes_ptr, self.hash_count) };
        let idx = hashes.binary_search_by_key(&hash, |e| e.hash).ok()?;
        Some(hashes[idx].node_id)
    }

    #[inline]
    pub fn get_node(&self, id: NodeId) -> Option<&HotNode> {
        if (id as usize) < self.node_count {
            unsafe { Some(&*self.nodes_ptr.add(id as usize)) }
        } else {
            None
        }
    }

    #[inline]
    pub fn get_edges(&self, node: &HotNode) -> &[HotEdge] {
        let start = node.first_edge as usize;
        let count = node.edge_count as usize;
        if start + count <= self.edge_count {
            unsafe { std::slice::from_raw_parts(self.edges_ptr.add(start), count) }
        } else {
            &[]
        }
    }

    #[inline]
    pub fn total_database_games(&self) -> u64 {
        self.header.db_game_count
    }
}
