use shakmaty::fen::Fen;
use shakmaty::san::SanPlus;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{Chess, EnPassantMode};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use super::codec::{format_continuation_moves, parse_fen_fullmove, MmapHotGraph};
use super::types::{
    ContinuationLine, ContinuationResult, HotEdge, HotGraphMetadata, HotHashEntry, HotNode, NodeId,
    PackedMove, NO_NODE,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchPath {
    node_id: NodeId,
    moves: Vec<PackedMove>,
    games: u64,
    white_wins: u64,
    draws: u64,
    black_wins: u64,
}

impl Ord for SearchPath {
    fn cmp(&self, other: &Self) -> Ordering {
        self.games
            .cmp(&other.games)
            .then_with(|| other.moves.len().cmp(&self.moves.len()))
    }
}

impl PartialOrd for SearchPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Default)]
pub struct HotGraph {
    pub nodes: Vec<HotNode>,
    pub edges: Vec<HotEdge>,
    pub hash_entries: Vec<HotHashEntry>,
    pub hash_to_node: HashMap<u64, NodeId>,
    pub metadata: HotGraphMetadata,
}

impl HotGraph {
    #[inline]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    #[inline]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    #[inline]
    pub fn get_node_id(&self, hash: u64) -> Option<NodeId> {
        if !self.hash_to_node.is_empty() {
            self.hash_to_node.get(&hash).copied()
        } else {
            let idx = self
                .hash_entries
                .binary_search_by_key(&hash, |e| e.hash)
                .ok()?;
            Some(self.hash_entries[idx].node_id)
        }
    }

    #[inline]
    pub fn get_node(&self, id: NodeId) -> Option<&HotNode> {
        self.nodes.get(id as usize)
    }

    #[inline]
    pub fn get_edges(&self, node: &HotNode) -> &[HotEdge] {
        let start = node.first_edge as usize;
        let end = start + (node.edge_count as usize);
        if end <= self.edges.len() {
            &self.edges[start..end]
        } else {
            &[]
        }
    }

    #[inline]
    pub fn total_database_games(&self) -> u64 {
        self.metadata.db_game_count
    }
}

pub trait HotGraphQueryable {
    fn get_node_id(&self, hash: u64) -> Option<NodeId>;
    fn get_node(&self, id: NodeId) -> Option<&HotNode>;
    fn get_edges(&self, node: &HotNode) -> &[HotEdge];
    fn total_database_games(&self) -> u64;

    fn query_continuations(
        &self,
        start_pos: &Chess,
        fen_str: &str,
        max_depth: usize,
        max_lines: usize,
        min_games: u64,
        min_percentage: f64,
    ) -> ContinuationResult {
        let start_hash_val: Zobrist64 = start_pos.zobrist_hash(EnPassantMode::Legal);
        let start_hash = start_hash_val.0;

        let start_node_id = match self.get_node_id(start_hash) {
            Some(id) => id,
            None => {
                return ContinuationResult {
                    starting_fen: Fen::from_position(start_pos.clone(), EnPassantMode::Legal)
                        .to_string(),
                    total_games_processed: self.total_database_games(),
                    games_reaching_position: 0,
                    lines: Vec::new(),
                    tree: None,
                };
            }
        };

        let start_node = match self.get_node(start_node_id) {
            Some(n) => n,
            None => {
                return ContinuationResult {
                    starting_fen: Fen::from_position(start_pos.clone(), EnPassantMode::Legal)
                        .to_string(),
                    total_games_processed: self.total_database_games(),
                    games_reaching_position: 0,
                    lines: Vec::new(),
                    tree: None,
                };
            }
        };

        let games_reaching = start_node.total_games as u64;
        if games_reaching == 0 {
            return ContinuationResult {
                starting_fen: Fen::from_position(start_pos.clone(), EnPassantMode::Legal)
                    .to_string(),
                total_games_processed: self.total_database_games(),
                games_reaching_position: 0,
                lines: Vec::new(),
                tree: None,
            };
        }

        let raw_lines =
            self.extract_top_lines(start_node_id, start_node, max_depth, max_lines, min_games);
        let start_fullmove = parse_fen_fullmove(fen_str);
        let mut final_lines = Vec::with_capacity(raw_lines.len());

        for line in raw_lines {
            let percentage = (line.games as f64 / games_reaching as f64) * 100.0;
            if line.games >= min_games && percentage >= min_percentage {
                let mut sim_pos = start_pos.clone();
                let mut san_moves = Vec::with_capacity(line.moves.len());

                for pm in line.moves {
                    if let Some(m) = pm.to_shakmaty_move(&sim_pos) {
                        let san_plus = SanPlus::from_move_and_play_unchecked(&mut sim_pos, &m);
                        san_moves.push(san_plus.to_string());
                    } else {
                        san_moves.push(pm.to_uci_string());
                    }
                }

                let formatted = format_continuation_moves(start_pos, start_fullmove, &san_moves);
                final_lines.push(ContinuationLine {
                    moves: san_moves,
                    formatted,
                    games: line.games,
                    percentage,
                    white_wins: line.white_wins,
                    draws: line.draws,
                    black_wins: line.black_wins,
                });
            }
        }

        final_lines.sort_by(|a, b| {
            b.games
                .cmp(&a.games)
                .then_with(|| b.moves.len().cmp(&a.moves.len()))
                .then_with(|| a.moves.cmp(&b.moves))
        });

        if final_lines.len() > max_lines {
            final_lines.truncate(max_lines);
        }

        ContinuationResult {
            starting_fen: Fen::from_position(start_pos.clone(), EnPassantMode::Legal).to_string(),
            total_games_processed: self.total_database_games(),
            games_reaching_position: games_reaching,
            lines: final_lines,
            tree: None,
        }
    }

    fn extract_top_lines(
        &self,
        _start_node_id: NodeId,
        start_node: &HotNode,
        max_depth: usize,
        max_lines: usize,
        min_games: u64,
    ) -> Vec<SearchPath> {
        let start_edges = self.get_edges(start_node);
        if start_edges.is_empty() || max_depth == 0 {
            return Vec::new();
        }

        let mut completed_lines: Vec<SearchPath> = Vec::new();
        let mut heap: BinaryHeap<SearchPath> = BinaryHeap::new();

        for e in start_edges {
            if (e.total_games as u64) >= min_games {
                heap.push(SearchPath {
                    node_id: e.target_node,
                    moves: vec![e.packed_move()],
                    games: e.total_games as u64,
                    white_wins: e.white_wins as u64,
                    draws: e.draws as u64,
                    black_wins: e.black_wins as u64,
                });
            }
        }

        while let Some(path) = heap.pop() {
            if path.moves.len() >= max_depth || path.node_id == NO_NODE {
                completed_lines.push(path);
                if completed_lines.len() >= max_lines * 4 {
                    break;
                }
                continue;
            }

            let next_node = match self.get_node(path.node_id) {
                Some(n) => n,
                None => {
                    completed_lines.push(path);
                    continue;
                }
            };

            let next_edges = self.get_edges(next_node);
            let mut branched = false;

            for e in next_edges {
                if (e.total_games as u64) >= min_games {
                    let mut new_moves = path.moves.clone();
                    new_moves.push(e.packed_move());
                    heap.push(SearchPath {
                        node_id: e.target_node,
                        moves: new_moves,
                        games: e.total_games as u64,
                        white_wins: e.white_wins as u64,
                        draws: e.draws as u64,
                        black_wins: e.black_wins as u64,
                    });
                    branched = true;
                }
            }

            if !branched {
                completed_lines.push(path);
            }
        }

        completed_lines.sort_by(|a, b| {
            b.games
                .cmp(&a.games)
                .then_with(|| b.moves.len().cmp(&a.moves.len()))
        });

        completed_lines.dedup_by(|a, b| a.moves == b.moves);
        completed_lines.truncate(max_lines * 2);
        completed_lines
    }
}

impl HotGraphQueryable for HotGraph {
    fn get_node_id(&self, hash: u64) -> Option<NodeId> {
        HotGraph::get_node_id(self, hash)
    }

    fn get_node(&self, id: NodeId) -> Option<&HotNode> {
        HotGraph::get_node(self, id)
    }

    fn get_edges(&self, node: &HotNode) -> &[HotEdge] {
        HotGraph::get_edges(self, node)
    }

    fn total_database_games(&self) -> u64 {
        HotGraph::total_database_games(self)
    }
}

impl HotGraphQueryable for MmapHotGraph {
    fn get_node_id(&self, hash: u64) -> Option<NodeId> {
        MmapHotGraph::get_node_id(self, hash)
    }

    fn get_node(&self, id: NodeId) -> Option<&HotNode> {
        MmapHotGraph::get_node(self, id)
    }

    fn get_edges(&self, node: &HotNode) -> &[HotEdge] {
        MmapHotGraph::get_edges(self, node)
    }

    fn total_database_games(&self) -> u64 {
        MmapHotGraph::total_database_games(self)
    }
}
