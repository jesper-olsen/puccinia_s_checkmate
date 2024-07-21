pub mod bitmaps;
pub mod hashkeys;
pub mod hashkeys_generated;
use crate::hashkeys_generated::WHITE_HASH;
pub mod mgen;
pub mod misc;
pub mod openings;
pub mod val;
pub mod benchmark;
use crate::Piece::*;
use core::cmp::max;
use core::cmp::min;
use hashkeys::*;
use mgen::*;
use std::collections::hash_map::{Entry, HashMap};
use std::fmt;
use val::*;

pub const INFINITE: i16 = 10000;

#[derive(Debug, Copy, Clone, PartialEq)]
enum BType {
    Exact,
    Lower,
    Upper,
}

#[derive(Debug, Copy, Clone)]
pub struct TTable {
    pub depth: usize,
    pub score: i16,
    pub m: Move,
    bound: BType,
}

pub struct Game {
    pub board: [Piece; 64],
    pub colour: bool,
    pub n_searched: usize,
    material: i16,
    rep: HashMap<u64, usize>,
    pub ttable: HashMap<u64, TTable>,
    pub can_castle: Vec<[bool; 4]>, // white short, long, black short, long
    end_game: bool,
    pub hash: u64,
    bm_white: u64,
    bm_black: u64,
    bm_pawns: u64,
    bm_wking: u64,
    bm_bking: u64,
    log_bms: Vec<(u64, u64, u64, u64, u64, Piece, u64)>,
}

impl fmt::Debug for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        for y in (0..8).rev() {
            write!(f, "{} ", y + 1)?;
            for x in 0..8 {
                write!(f, "{}", self.board[(7 - x) * 8 + y])?;
            }
            writeln!(f)?;
        }
        write!(f, "  ABCDEFGH")
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ANSI escape codes for background and foreground colors
        let light_square_bg = "\x1b[48;5;229m"; // Light background
        let dark_square_bg = "\x1b[48;5;94m"; // Dark background
                                              //let light_square_bg = "\x1b[48;5;15m"; // White background
                                              //let dark_square_bg = "\x1b[48;5;8m";   // Gray background
        let black_fg = "\x1b[38;5;0m"; // Black foreground
        let white_fg = "\x1b[38;5;15m"; // White foreground
        let reset_colour = "\x1b[0m"; // Reset to default colour

        writeln!(f, "{}", self.to_fen())?;
        for y in (0..8).rev() {
            write!(f, "{} ", y + 1)?;
            for x in 0..8 {
                let i = (7 - x) * 8 + y;
                let ch = self.board[i].to_unicode();
                let fg = if self.board[i].is_white() {
                    white_fg
                } else {
                    black_fg
                };
                let is_light_square = (x + y) % 2 != 0;
                let background_color = if is_light_square {
                    light_square_bg
                } else {
                    dark_square_bg
                };
                let fg_colour = if is_light_square && fg == white_fg {
                    black_fg
                } else {
                    fg
                };
                write!(f, "{background_color}{fg_colour} {ch} {reset_colour}")?;
            }
            writeln!(f)?;
        }
        write!(f, "   A  B  C  D  E  F  G  H")
    }
}

fn move_to_head(moves: &mut Vec<Move>, k: &Move) {
    if let Some(q) = moves
        .iter()
        .position(|m| (m.frm(), m.to()) == (k.frm(), k.to()))
    {
        if q != 0 {
            let m = moves.remove(q);
            //let m = moves.swap_remove(q);
            //println!("Move {} to head {}->0", m, q);
            moves.insert(0, m);
        }
    }
}

impl Game {
    pub fn new(board: [Piece; 64]) -> Self {
        let key = board2hash(&board, WHITE);
        let (bm_white, bm_black) = board2bm(&board);
        Game {
            board,
            colour: WHITE,
            n_searched: 0,
            material: material(&board),
            rep: HashMap::from([(key, 1)]),
            ttable: HashMap::new(),
            can_castle: vec![[true; 4]],
            end_game: false,
            hash: key,
            bm_white,
            bm_black,
            bm_pawns: board2bm_pawns(&board),
            log_bms: vec![],
            bm_wking: 0,
            bm_bking: 0,
        }
    }

    pub fn to_fen(&self) -> String {
        let mut s = String::new();
        for y in (0..=7).rev() {
            let mut n = 0;
            for x in (0..=7).rev() {
                let idx = x * 8 + y;
                if self.board[idx] == Nil {
                    n += 1;
                } else {
                    if n > 0 {
                        s.push_str(format!("{}", n).as_str());
                        n = 0;
                    }
                    s.push(self.board[idx].to_ascii())
                }
            }
            if n > 0 {
                s.push_str(format!("{}", n).as_str())
            }

            if y != 0 {
                s.push('/')
            }
        }
        s.push_str(if self.turn() == WHITE { " w" } else { " b" });
        s
    }

    pub fn from_fen(s: &str) -> Self {
        let mut a = [Nil; 64];
        let mut offset = 0i16;
        let parts = s.split(' ').collect::<Vec<&str>>();
        for (i, c) in parts[0].chars().enumerate() {
            if let Some(d) = c.to_digit(10) {
                offset += d as i16 - 1;
            } else if c == '/' {
                offset -= 1;
            } else {
                let k: usize = (i as i16 + offset).try_into().unwrap();
                let x = 7 - k % 8;
                let y = 7 - k / 8;
                let q = x * 8 + y;
                a[q] = Piece::from_ascii(c);
            }
        }

        let mut game = Game::new(a);
        if parts.len() > 1 {
            game.colour = matches!(parts[1].chars().nth(0), Some('w') | Some('W'));
        }
        game
    }

    pub fn rep_len(&self) -> usize {
        self.rep.len()
    }

    pub fn ttable_len(&self) -> usize {
        self.ttable.len()
    }

    fn is_quiescent(&self, last: &Move) -> bool {
        // quiescent unless last move was pawn near promotion
        // !self.in_check(self.colour) &&
        match self.board[last.to()] {
            Pawn(WHITE) => last.to() % 8 != 6,
            Pawn(BLACK) => last.to() % 8 != 1,
            _ => true,
        }
    }

    fn ttable_clear(&mut self) {
        let key = self.hash;
        if self.ttable.contains_key(&key) {
            self.ttable = HashMap::from([(key, self.ttable[&key])]);
        } else {
            self.ttable = HashMap::new();
        }
    }

    fn rep_clear(&mut self) {
        let key = board2hash(&self.board, self.colour);
        self.rep = HashMap::from([(key, 1)]);
    }

    fn rep_inc(&mut self) {
        //*self.rep.entry(self.hash).or_default() += 1;
        self.rep
            .entry(self.hash)
            .and_modify(|x| *x += 1)
            .or_insert(1);
    }

    fn rep_dec(&mut self) {
        if let Entry::Occupied(entry) = self
            .rep
            .entry(self.hash)
            .and_modify(|x| *x = x.saturating_sub(1))
        {
            if *entry.get() == 0 {
                self.rep.remove(&self.hash);
            }
        }

        // self.rep
        //     .entry(self.hash)
        //     .and_modify(|x| *x = x.saturating_sub(1));
        // if let Some(0) = self.rep.get(&self.hash) {
        //     self.rep.remove(&self.hash);
        // }

        // self.rep
        //     .entry(self.hash)
        //     .and_modify(|x| *x = if *x == 0 { 0 } else { *x - 1 });
        // if let Some(count) = self.rep.get(&self.hash) {
        //     if *count == 0 {
        //         self.rep.remove(&self.hash);
        //     }
        // }
    }

    pub fn rep_count(&self) -> usize {
        if let Some(count) = self.rep.get(&self.hash) {
            *count
        } else {
            0
        }
    }

    pub fn make_move(&mut self, m: Move) {
        if m.en_passant()
            || self.board[m.to()] != Nil
            || [Pawn(WHITE), Pawn(BLACK)].contains(&self.board[m.frm()])
        {
            self.rep_clear(); // ireversible move
        }
        self.ttable_clear();
        self.update(&m);

        //adjust king value in end game
        self.end_game = abs_material(&self.board) < END_GAME_MATERIAL / 3;

        //update castling permissions
        let cc = self.can_castle.last_mut().unwrap();
        match (*cc, self.board[m.to()], m.frm()) {
            ([true, _, _, _], King(WHITE), 24) => (cc[0], cc[1]) = (false, false),
            ([_, true, _, _], King(WHITE), 24) => (cc[0], cc[1]) = (false, false),
            ([_, _, true, _], King(BLACK), 31) => (cc[2], cc[3]) = (false, false),
            ([_, _, _, true], King(BLACK), 31) => (cc[2], cc[3]) = (false, false),
            ([true, _, _, _], Rook(WHITE), 0) => cc[0] = false,
            ([_, true, _, _], Rook(WHITE), 56) => cc[1] = false,
            ([_, _, true, _], Rook(BLACK), 7) => cc[2] = false,
            ([_, _, _, true], Rook(BLACK), 63) => cc[3] = false,
            _ => (),
        }
    }

    pub fn check_50_move_rule(&self) -> bool {
        self.rep.iter().map(|(_, &v)| v).sum::<usize>() >= 100
    }

    pub fn in_check(&self, colour: bool) -> bool {
        // true if other side can capture king

        mgen::in_check(
            &self.board,
            colour,
            self.bm_wking,
            self.bm_bking,
            self.bm_white | self.bm_black,
        )
    }

    fn legal_move(&mut self, m: &Move) -> bool {
        // verify move does not expose own king
        self.update(m);
        let flag = match self.board[m.to()] {
            Rook(c) | Knight(c) | Bishop(c) | Queen(c) | King(c) | Pawn(c) => self.in_check(c),
            _ => false,
        };
        self.backdate(m);
        !flag
    }

    pub fn legal_moves(&mut self, last: Option<&Move>) -> Vec<Move> {
        let mut moves = self.moves(self.colour, last);
        moves.retain(|m| self.legal_move(m));
        moves
    }

    fn moves(&mut self, colour: bool, last: Option<&Move>) -> Vec<Move> {
        let mut l = moves(
            &self.board,
            colour,
            self.end_game,
            self.can_castle.last().unwrap(),
            last,
            self.bm_white,
            self.bm_black,
        );
        if colour {
            //l.sort_by(|b, a| a.val.cmp(&b.val)); // decreasing
            l.sort_unstable_by(|b, a| a.val.cmp(&b.val)); // decreasing
        } else {
            //l.sort_by(|a, b| a.val.cmp(&b.val)); // increasing
            l.sort_unstable_by(|a, b| a.val.cmp(&b.val)); // increasing
        }
        self.n_searched += l.len();
        l
    }

    pub fn turn(&self) -> bool {
        self.colour
    }

    pub fn update(&mut self, m: &Move) {
        self.log_bms.push((
            self.bm_pawns,
            self.bm_white,
            self.bm_black,
            self.bm_wking,
            self.bm_bking,
            self.board[m.to()],
            self.hash,
        ));
        self.colour = !self.colour;
        let hash;
        self.board[m.to()] = if m.castle() {
            let cc = self.can_castle.last().unwrap();
            match self.board[m.frm()] {
                King(WHITE) => self.can_castle.push([false, false, cc[2], cc[3]]),
                King(BLACK) => self.can_castle.push([cc[0], cc[1], false, false]),
                _ => (),
            }

            let (x, y) = if m.to() <= 15 {
                (m.frm() - 24, m.frm() - 8) // short
            } else {
                (m.frm() + 32, m.frm() + 8) // long
            };
            hash = self.board[m.frm()].hashkey(m.to())
                ^ self.board[m.frm()].hashkey(m.frm())
                ^ self.board[x].hashkey(y)
                ^ self.board[x].hashkey(x);
            self.board[y] = self.board[x]; // move rook
            self.board[x] = Nil;
            self.board[m.frm()]
        } else if m.en_passant() {
            // +9  +1 -7
            // +8   0 -8
            // +7  -1 -9
            let x = match m.to() > m.frm() {
                true => m.frm() + 8,  // west
                false => m.frm() - 8, // east
            };
            hash = self.board[m.frm()].hashkey(m.to())
                ^ self.board[m.frm()].hashkey(m.frm())
                ^ self.board[x].hashkey(x);
            self.board[x] = Nil;
            self.board[m.frm()]
        } else if m.transform() {
            hash = self.board[m.frm()].transform(m.to()).hashkey(m.to())
                ^ self.board[m.frm()].hashkey(m.frm())
                ^ self.board[m.to()].hashkey(m.to());
            self.board[m.frm()].transform(m.to())
        } else {
            hash = self.board[m.frm()].hashkey(m.to())
                ^ self.board[m.frm()].hashkey(m.frm())
                ^ self.board[m.to()].hashkey(m.to());
            self.board[m.frm()]
        };
        self.board[m.frm()] = Nil;
        self.material += m.val;
        self.rep_inc();
        //self.hash ^= m.hash ^ WHITE_HASH;
        self.hash ^= hash ^ WHITE_HASH;

        // update bitmaps - TODO calculate incrementally; ~6% faster?
        self.bm_pawns = 0;
        self.bm_white = 0;
        self.bm_black = 0;
        for i in 0..64 {
            match self.board[i] {
                Pawn(WHITE) => {
                    self.bm_pawns |= 1 << i;
                    self.bm_white |= 1 << i;
                }
                Pawn(BLACK) => {
                    self.bm_pawns |= 1 << i;
                    self.bm_black |= 1 << i;
                }
                Rook(WHITE) | Knight(WHITE) | Bishop(WHITE) | Queen(WHITE) => {
                    self.bm_white |= 1 << i
                }
                Rook(BLACK) | Knight(BLACK) | Bishop(BLACK) | Queen(BLACK) => {
                    self.bm_black |= 1 << i
                }
                King(WHITE) => {
                    self.bm_white |= 1 << i;
                    self.bm_wking = 1 << i
                }
                King(BLACK) => {
                    self.bm_black |= 1 << i;
                    self.bm_bking = 1 << i
                }
                _ => (),
            }
        }
    }

    pub fn backdate(&mut self, m: &Move) {
        let bms = self.log_bms.pop().unwrap();
        let capture;
        (
            self.bm_pawns,
            self.bm_white,
            self.bm_black,
            self.bm_wking,
            self.bm_bking,
            capture,
            self.hash,
        ) = bms;
        self.colour = !self.colour;
        //self.hash ^= m.hash ^ WHITE_HASH;
        self.rep_dec();
        if m.castle() {
            self.can_castle.pop();
            let (frm, to) = if m.to() <= 15 {
                (m.frm() - 24, m.frm() - 8) // short
            } else {
                (m.frm() + 32, m.frm() + 8) // long
            };
            self.board[frm] = self.board[to]; // move rook
            self.board[to] = Nil;
        }
        self.board[m.frm()] = if m.transform() {
            match m.to() % 8 {
                7 => Pawn(WHITE),
                0 => Pawn(BLACK),
                _ => panic!(),
            }
        } else {
            self.board[m.to()]
        };
        self.board[m.to()] = capture;

        if m.en_passant() {
            let x = match m.to() > m.frm() {
                true => m.frm() + 8,  // west
                false => m.frm() - 8, // w east
            };
            self.board[x] = match self.board[m.frm()] {
                Pawn(WHITE) => Pawn(BLACK),
                Pawn(BLACK) => Pawn(WHITE),
                _ => unreachable!(),
            }
        }

        self.material -= m.val;
    }

    fn ttstore(&mut self, depth: usize, score: i16, alpha: i16, beta: i16, m: &Move) {
        // TODO - implement more efficient hashing function
        let key = self.hash;
        let e = TTable {
            depth,
            score,
            bound: if score <= alpha {
                BType::Upper
            } else if score >= beta {
                BType::Lower
            } else {
                BType::Exact
            },
            m: *m,
        };
        self.ttable
            .entry(key)
            .and_modify(|x| {
                if x.depth < e.depth {
                    *x = e;
                }
            })
            .or_insert(e);
    }

    pub fn mobility(&self) -> i16 {
        count_moves(&self.board, WHITE, self.bm_white, self.bm_black) as i16
            - count_moves(&self.board, BLACK, self.bm_white, self.bm_black) as i16
    }

    pub fn eval(&self, colour: bool) -> i16 {
        let s = self.material + self.score_pawn_structure() + self.mobility();
        if colour {
            s
        } else {
            -s
        }
        //s * (2 * (colour as i16) - 1)
    }

    pub fn score_pawn_structure(&self) -> i16 {
        let mut pen: i16 = 0;
        let bm: [u64; 2] = [self.bm_pawns & self.bm_white, self.bm_pawns & self.bm_black];
        for (i, &p) in [Pawn(WHITE), Pawn(BLACK)].iter().enumerate() {
            let nfiles = (0..8)
                .filter(|&q| 0b11111111 << (q * 8) & bm[i] > 0)
                .count() as i16;
            let npawns = bm[i].count_ones() as i16;
            let double_pawns = npawns - nfiles;

            let l = (0..8)
                .map(|q| (0b11111111 << (q * 8)) & bm[i] > 0)
                .collect::<Vec<bool>>();
            let isolated_pawns = (0..8)
                .filter(|&q| {
                    (q == 0 && l[0] && !l[1])
                        || (q > 0 && q < 7 && l[q] && !l[q - 1] && !l[q + 1])
                        || (q == 7 && l[7] && !l[6])
                })
                .count() as i16;

            let x = 20 * double_pawns + 4 * isolated_pawns;
            pen += if p == Pawn(WHITE) { -x } else { x };
        }

        // passed pawn bonus
        for i in 0..8 {
            let file: u64 = 0b11111111 << (i * 8);
            let w = file & bm[0];
            let b = file & bm[1];
            if w > 0 && w > b {
                let k = 63 - w.leading_zeros();
                let q = (k % 8) as i16;
                pen += 2 * q * q;
            }
            if b > 0 && (w == 0 || b < w) {
                let k = b.trailing_zeros();
                let q = (7 - k % 8) as i16;
                pen -= 2 * q * q;
            }
        }

        pen
    }

    fn quiescence_fab(&mut self, _ply: usize, alp: i16, beta: i16, last: &Move, rfab: bool) -> i16 {
        let colour = self.colour;

        let mut bscore = None;
        let mut alpha = alp;
        let mut moves = self.moves(colour, Some(last));
        moves.retain(|m|
            //let ic = self.in_check(colour);
            if rfab {
                m.to() == last.to()
            } else {
                m.en_passant() || self.board[m.to()] != Nil
            }
        );
        for m in moves {
            self.update(&m);
            if !self.in_check(colour) {
                // legal move
                let score = -self.quiescence_fab(_ply + 1, -beta, -alpha, &m, true);
                match bscore {
                    Some(bs) if score <= bs => (),
                    _ => {
                        if score >= beta {
                            self.backdate(&m);
                            return score;
                        }
                        bscore = Some(score);
                        alpha = max(alpha, score);
                    }
                }
            }
            self.backdate(&m);
        }
        if let Some(bs) = bscore {
            bs
        } else {
            self.eval(colour)
        }
    } // fn quiescence fab

    pub fn pvs(&mut self, dpt: usize, ply: usize, alp: i16, bet: i16, last: &Move) -> i16 {
        if self.rep_count() >= 2 {
            return 0;
        }

        let mut alpha = alp;
        let mut beta = bet;
        let mut bscore = -INFINITE + ply as i16;
        let mut bmove = None;
        let colour = self.colour;

        let in_check = self.in_check(colour);
        let mut depth = if in_check { dpt + 1 } else { dpt };

        let mut kmove = None;
        let key = self.hash;
        if let Some(e) = self.ttable.get(&key) {
            if e.depth >= depth {
                match e.bound {
                    BType::Exact => return e.score,
                    BType::Lower => alpha = max(alpha, e.score),
                    BType::Upper => beta = min(beta, e.score),
                }
                if alpha >= beta {
                    return e.score;
                }
            }
            kmove = Some(e.m);
        }

        match depth {
            0 if self.is_quiescent(last) => {
                return self.quiescence_fab(ply, alpha, beta, last, false)
            }
            0 => depth = 1,
            _ => (),
        }

        let mut moves = self.moves(colour, Some(last));
        if let Some(k) = kmove {
            move_to_head(&mut moves, &k);
        }
        for m in moves.iter() {
            self.update(m);
            if !self.in_check(colour) {
                // legal move
                if bmove.is_none() {
                    bscore = -self.pvs(depth - 1, ply + 1, -beta, -alpha, m); // full beam
                    alpha = max(bscore, alpha);
                    bmove = Some(m);
                } else {
                    let mut score =
                        -self.pvs(depth - 1, ply + 1, -alpha - 1, -max(bscore, alpha), m);
                    if score > bscore {
                        if score > max(bscore, alpha) && score < beta && depth > 2 {
                            score = -self.pvs(depth - 1, ply + 1, -beta, -score, m);
                        }
                        bscore = score;
                        alpha = max(bscore, alpha);
                        bmove = Some(m);
                    }
                }
            }
            self.backdate(m);
            if let Some(m) = bmove {
                if bscore >= beta {
                    self.ttstore(depth, bscore, alp, beta, m);
                    return bscore;
                }
            }
        }
        if let Some(m) = bmove {
            self.ttstore(depth, bscore, alp, beta, m);
        }

        match (bmove, in_check) {
            (None, false) => 0,
            _ => bscore,
        }
    }

    pub fn score_moves(
        &mut self,
        moves: &[Move],
        max_searched: usize,
        max_depth: usize,
        verbose: bool,
    ) -> Vec<(Move, i16)> {
        // top level pvs - does iterative deepening, sorts moves
        // note that only the best move has exact scoring...

        if moves.is_empty() {
            return vec![];
        }

        self.n_searched = 0;
        let mut pq0: Vec<(Move, i16)> = moves.iter().map(|m| (*m, 0)).collect();
        for depth in (2..=max_depth).step_by(1) {
            if depth > 1 && self.n_searched > max_searched {
                break;
            }
            let mut pq: Vec<(Move, i16)> = Vec::new();
            let mut alpha = -INFINITE;
            let beta = INFINITE;
            let mut bscore = alpha;

            for (i, (m, _v)) in pq0.iter().enumerate() {
                self.update(m);
                alpha = max(bscore, alpha);
                let mut score = if i == 0 {
                    // full beam
                    -self.pvs(depth - 1, 1, -beta, -alpha, m)
                } else {
                    -self.pvs(depth - 1, 1, -alpha - 1, -alpha, m)
                };

                if score > bscore {
                    if score > alpha && score < beta && depth > 2 {
                        score = -self.pvs(depth - 1, 1, -beta, -score, m);
                    }
                    bscore = score;
                }
                self.backdate(m);
                pq.push((*m, score));
            }
            pq.sort_by(|b, a| a.1.cmp(&b.1)); // decreasing
            pq0 = pq;
            if verbose {
                println!(
                    "Depth {:>2} #searched {:>8} bmove: {} bscore: {}",
                    depth, self.n_searched, pq0[0].0, bscore
                );
            }
            if !pq0.is_empty() && pq0[0].1.abs() >= INFINITE - depth as i16 {
                break;
            }
        }
        pq0
    } // fn score_moves
}
