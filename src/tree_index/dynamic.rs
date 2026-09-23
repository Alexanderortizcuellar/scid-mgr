use rayon::prelude::*;
use shakmaty::san::SanPlus;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{Chess, EnPassantMode, Position};
use std::fs::File;
use std::path::Path;

use super::codec::{generate_tree_report, parse_target_position};
use super::types::{OpeningTreeReport, PackedMove, TreePositionNode};

// ---------------------------------------------------------------------------
// Dynamic On-the-Fly Tree Calculations
// ---------------------------------------------------------------------------

/// On-the-fly dynamic calculation of opening tree statistics for SCID databases
pub fn calculate_tree_for_scid<P: AsRef<Path>>(
    entries: &[chess_scid_rw::entry::IndexEntry],
    games_path: P,
    fen_str: &str,
    target_game_ids: Option<&[usize]>,
    _max_depth: Option<usize>,
) -> Option<OpeningTreeReport> {
    let (target_pos, target_hash) = parse_target_position(fen_str)?;
    let file = File::open(games_path.as_ref()).ok()?;
    let mmap = unsafe { memmap2::Mmap::map(&file).ok()? };

    let max_ply = 50;
    let mut node = TreePositionNode::new(target_hash);

    let process_game = |game_id: usize, node: &mut TreePositionNode| {
        if game_id >= entries.len() {
            return;
        }
        let entry = &entries[game_id];
        if entry.deleted {
            return;
        }

        let start = entry.offset as usize;
        let end = start + entry.length as usize;
        if end > mmap.len() || start >= end {
            return;
        }

        let blob = &mmap[start..end];
        let mut cursor = 0;

        let mut pos = match crate::position_search::parse_start_position(blob, &mut cursor) {
            Some(p) => p,
            None => return,
        };

        let (w_win, draw, b_win) = match entry.result {
            1 => (1, 0, 0),
            2 => (0, 0, 1),
            3 => (0, 1, 0),
            _ => (0, 0, 0),
        };

        let w_elo = entry.white_elo;
        let b_elo = entry.black_elo;

        let mut slots = crate::position_search::standard_piece_slots();
        let mut counts = [16usize, 16usize];
        let mut ply = 0;

        while cursor < blob.len() && ply < max_ply {
            let curr_hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
            let is_match = curr_hash.0 == target_hash;

            let b = blob[cursor];
            cursor += 1;

            if b == 15 {
                if is_match {
                    node.record_game(None, w_win, draw, b_win, w_elo, b_elo);
                }
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
                    None => break,
                };

            if !pos.is_legal(&mv) {
                break;
            }

            if is_match {
                let packed = PackedMove::from(&mv).0;
                node.record_game(Some(packed), w_win, draw, b_win, w_elo, b_elo);
                break;
            }

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

            pos.play_unchecked(&mv);
            ply += 1;
        }
    };

    if let Some(ids) = target_game_ids {
        for &gid in ids {
            process_game(gid, &mut node);
        }
    } else {
        for gid in 0..entries.len() {
            process_game(gid, &mut node);
        }
    }

    Some(generate_tree_report(&node, &target_pos, target_hash))
}

/// On-the-fly dynamic calculation of opening tree statistics for PGN databases
pub fn calculate_tree_for_pgn(
    entries: &[crate::pgn_db::PgnIndexEntry],
    mmap: &memmap2::Mmap,
    fen_str: &str,
    target_game_ids: Option<&[usize]>,
    _max_depth: Option<usize>,
) -> Option<OpeningTreeReport> {
    let (target_pos, target_hash) = parse_target_position(fen_str)?;
    let max_ply = 50;

    let process_single_game = |game_id: usize, node: &mut TreePositionNode| {
        if game_id >= entries.len() {
            return;
        }
        let entry = &entries[game_id];
        let (w_win, draw, b_win) = match entry.result {
            1 => (1, 0, 0),
            2 => (0, 0, 1),
            3 => (0, 1, 0),
            _ => (0, 0, 0),
        };
        let w_elo = entry.white_elo;
        let b_elo = entry.black_elo;

        let start = entry.offset as usize;
        let end = start + entry.length as usize;
        if end > mmap.len() || start >= end {
            return;
        }

        let slice = &mmap[start..end];
        let mut reader = pgn_reader::BufferedReader::new_cursor(slice);
        let mut visitor = PgnSinglePositionVisitor::new(
            target_hash,
            max_ply,
            w_win,
            draw,
            b_win,
            w_elo,
            b_elo,
            node,
        );
        let _ = reader.read_game(&mut visitor);
    };

    let node = if let Some(ids) = target_game_ids {
        if ids.len() > 500 {
            ids.par_chunks(250)
                .map(|chunk| {
                    let mut local_node = TreePositionNode::new(target_hash);
                    for &gid in chunk {
                        process_single_game(gid, &mut local_node);
                    }
                    local_node
                })
                .reduce(
                    || TreePositionNode::new(target_hash),
                    |mut acc, n| {
                        acc.merge(n);
                        acc
                    },
                )
        } else {
            let mut local_node = TreePositionNode::new(target_hash);
            for &gid in ids {
                process_single_game(gid, &mut local_node);
            }
            local_node
        }
    } else {
        let total = entries.len();
        if total > 500 {
            (0..total)
                .into_par_iter()
                .chunks(250)
                .map(|chunk| {
                    let mut local_node = TreePositionNode::new(target_hash);
                    for gid in chunk {
                        process_single_game(gid, &mut local_node);
                    }
                    local_node
                })
                .reduce(
                    || TreePositionNode::new(target_hash),
                    |mut acc, n| {
                        acc.merge(n);
                        acc
                    },
                )
        } else {
            let mut local_node = TreePositionNode::new(target_hash);
            for gid in 0..total {
                process_single_game(gid, &mut local_node);
            }
            local_node
        }
    };

    Some(generate_tree_report(&node, &target_pos, target_hash))
}

pub(crate) struct PgnSinglePositionVisitor<'a> {
    target_hash: u64,
    max_ply: usize,
    w_win: u32,
    draw: u32,
    b_win: u32,
    w_elo: u16,
    b_elo: u16,
    node: &'a mut TreePositionNode,
    pos: Chess,
    ply: usize,
    matched: bool,
}

impl<'a> PgnSinglePositionVisitor<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        target_hash: u64,
        max_ply: usize,
        w_win: u32,
        draw: u32,
        b_win: u32,
        w_elo: u16,
        b_elo: u16,
        node: &'a mut TreePositionNode,
    ) -> Self {
        Self {
            target_hash,
            max_ply,
            w_win,
            draw,
            b_win,
            w_elo,
            b_elo,
            node,
            pos: Chess::default(),
            ply: 0,
            matched: false,
        }
    }
}

impl<'a> pgn_reader::Visitor for PgnSinglePositionVisitor<'a> {
    type Result = ();

    fn begin_game(&mut self) {
        self.pos = Chess::default();
        self.ply = 0;
        self.matched = false;
    }

    fn begin_variation(&mut self) -> pgn_reader::Skip {
        pgn_reader::Skip(true)
    }

    fn san(&mut self, san_plus: SanPlus) {
        if self.matched || self.ply >= self.max_ply {
            return;
        }

        let pre_hash: Zobrist64 = self.pos.zobrist_hash(EnPassantMode::Legal);
        let is_match = pre_hash.0 == self.target_hash;

        if let Ok(m) = san_plus.san.to_move(&self.pos) {
            if is_match {
                let packed = PackedMove::from(&m).0;
                self.node.record_game(
                    Some(packed),
                    self.w_win,
                    self.draw,
                    self.b_win,
                    self.w_elo,
                    self.b_elo,
                );
                self.matched = true;
                return;
            }

            self.pos.play_unchecked(&m);
            self.ply += 1;
        }
    }

    fn end_game(&mut self) {
        if !self.matched && self.ply < self.max_ply {
            let curr_hash: Zobrist64 = self.pos.zobrist_hash(EnPassantMode::Legal);
            if curr_hash.0 == self.target_hash {
                self.node.record_game(
                    None, self.w_win, self.draw, self.b_win, self.w_elo, self.b_elo,
                );
            }
        }
    }
}
