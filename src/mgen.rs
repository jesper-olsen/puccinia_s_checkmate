use crate::Piece;
use crate::bitmaps::*;
use crate::hashkeys_generated::WHITE_HASH;
use crate::misc;
use crate::val::*;
use crate::val::{Colour::*, Piece::*};
use std::collections::hash_map::{Entry, HashMap};
use std::fmt;
use std::ops::{Index, IndexMut};
use std::slice::Iter;

#[derive(Debug, Copy, Clone)]
pub struct Bitmaps {
    pub pieces: [u64; 2],
    pub pawns: u64,
    pub kings: u64,
}

// bitpacking - 1st 12 bits (6+6) for from/to, remaining 4 bits for castling and
// pawn transforms & enpassant. Castling, en passant & transform are mutually exclusive.
const CASTLE_BIT: u16 = 1 << 12;
const EN_PASSANT_BIT: u16 = 1 << 13;
const TRANSFORM_BIT: u16 = 1 << 14;
const TO_SHIFT: u16 = 6;
pub const CASTLE_W_SHORT: u8 = 0b0001;
pub const CASTLE_W_LONG: u8 = 0b0010;
pub const CASTLE_B_SHORT: u8 = 0b0100;
pub const CASTLE_B_LONG: u8 = 0b1000;
pub const FRM_MASK: u16 = 0b111111;
pub const TO_MASK: u16 = FRM_MASK << TO_SHIFT;

const fn pack_data(
    castle: bool,
    en_passant: bool,
    ptransform: Piece,
    frm: usize,
    to: usize,
) -> u16 {
    let (transform, tbits) = match ptransform {
        Rook(_) => (true, 1 << 15),
        Knight(_) => (true, 1 << 12),
        Bishop(_) => (true, 1 << 13),
        Queen(_) => (true, 0),
        _ => (false, 0),
    };
    ((castle as u16) << 12)
        | ((en_passant as u16) << 13)
        | ((transform as u16) << 14)
        | ((to << 6) | frm) as u16
        | tbits
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Move {
    pub data: u16,
    pub val: i16,
}

impl Move {
    pub fn new(castle: bool, en_passant: bool, frm: usize, to: usize) -> Self {
        // incomplete - needed by from_fen
        let data = pack_data(castle, en_passant, Nil, frm, to);
        Move { data, val: 0 }
    }
    #[inline]
    pub fn castle(&self) -> bool {
        self.data & CASTLE_BIT != 0 && !self.transform()
    }
    #[inline]
    pub fn en_passant(&self) -> bool {
        (self.data & EN_PASSANT_BIT) != 0 && !self.transform()
    }
    #[inline]
    pub fn ptransform(&self, colour: Colour) -> Piece {
        const MASK: u16 = 1 << 15 | 1 << 13 | 1 << 12;
        match self.data & MASK {
            0b10000000_00000000 => Rook(colour),
            0b00100000_00000000 => Bishop(colour),
            0b00010000_00000000 => Knight(colour),
            _ => Queen(colour),
        }
    }
    #[inline]
    pub fn transform(&self) -> bool {
        self.data & TRANSFORM_BIT != 0
    }
    #[inline]
    pub fn frm(&self) -> usize {
        (self.data & FRM_MASK) as usize
    }
    #[inline]
    pub fn to(&self) -> usize {
        ((self.data & TO_MASK) >> TO_SHIFT) as usize
    }
}

pub fn ext_frm(data: u16) -> u8 {
    (data & FRM_MASK) as u8
}

pub fn ext_to(data: u16) -> u8 {
    ((data & TO_MASK) >> TO_SHIFT) as u8
}

pub const NULL_MOVE: Move = Move {
    data: pack_data(false, false, Piece::Nil, 0, 0),
    val: 0,
};

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (frm, to) = (self.frm(), self.to());
        let t = if self.transform() {
            match self.ptransform(White) {
                Rook(_) => "=R",
                Knight(_) => "=N",
                Bishop(_) => "=B",
                _ => "=Q",
            }
        } else {
            ""
        };
        write!(f, "{}{}{t}", I2SQ[frm], I2SQ[to])
    }
}

pub struct Board {
    squares: [Piece; 64],
    pub colour: Colour,
    pub can_castle: u8, // white short, long, black short, long
    pub move_log: Vec<Move>,
    pub material: i16,
    pub hash: u64,
    pub half_move_clock: usize, // since last irreversible move
    pub full_move_count: usize,
    pub rep: HashMap<u64, usize>,
    bitmaps: Bitmaps,
    end_game_material: i16,
    log_bms: Vec<(Bitmaps, Piece, u64, u8)>,
}

impl Default for Board {
    fn default() -> Self {
        Board::from_fen(ROOT_FEN)
    }
}

impl Index<usize> for Board {
    type Output = Piece;
    #[inline(always)]
    fn index(&self, idx: usize) -> &Self::Output {
        &self.squares[idx]
    }
}

impl IndexMut<usize> for Board {
    #[inline(always)]
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.squares[idx]
    }
}

impl<'a> IntoIterator for &'a Board {
    type Item = &'a Piece;
    type IntoIter = Iter<'a, Piece>;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.squares.iter()
    }
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        for y in (0..8).rev() {
            write!(f, "{} ", y + 1)?;
            for x in 0..8 {
                write!(f, "{}", self.squares[(7 - x) * 8 + y])?;
            }
            writeln!(f)?;
        }
        write!(f, "  ABCDEFGH")
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ANSI escape codes for background and foreground colors
        let light_square_bg = "\x1b[48;5;229m"; // Light background
        let dark_square_bg = "\x1b[48;5;94m"; // Dark background
        //let light_square_bg = "\x1b[48;5;15m"; // White background
        //let dark_square_bg = "\x1b[48;5;8m";   // Gray background
        let black_fg = "\x1b[38;5;0m"; // Black foreground
        let white_fg = "\x1b[38;5;15m"; // White foreground
        let reset_colour = "\x1b[0m"; // Reset to default colour

        for y in (0..8).rev() {
            write!(f, "{} ", y + 1)?;
            for x in 0..8 {
                let i = (7 - x) * 8 + y;
                let ch = self.squares[i].to_unicode();
                let fg = if self.squares[i].is_white() {
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

impl Board {
    pub fn rep_count(&self) -> usize {
        if let Some(count) = self.rep.get(&self.hash) {
            if *count >= 2 {
                return 0;
            }
        }
        0
    }

    pub fn from_fen(s: &str) -> Self {
        let parts = s.split(' ').collect::<Vec<&str>>();
        let squares = from_fen(parts[0]);

        let colour = if parts.len() > 1 {
            if parts[1].to_lowercase().starts_with('w') {
                Colour::White
            } else {
                Colour::Black
            }
        } else {
            Colour::White
        };

        let mut can_castle = 0;
        if parts.len() > 2 {
            for (c, q) in ['K', 'Q', 'k', 'q'].into_iter().zip([
                CASTLE_W_SHORT,
                CASTLE_W_LONG,
                CASTLE_B_SHORT,
                CASTLE_B_LONG,
            ]) {
                if parts[2].contains(c) {
                    can_castle |= q
                }
            }
        }

        let mut move_log = Vec::new();
        if parts.len() > 3 {
            // en passant attack
            if let Some(sq) = misc::parse_chess_coord(parts[3]) {
                let sq = sq as isize;
                let o = if colour.is_white() { -1 } else { 1 };
                let to: usize = (sq + o).try_into().expect("must be positive");
                let frm: usize = (sq - o).try_into().expect("must be positive");
                let m = Move::new(false, true, frm, to);
                move_log.push(m);
            }
        }

        let half_move_clock = if parts.len() > 4 {
            // moves since last irreversible move
            parts[4].parse::<usize>().unwrap_or(0)
        } else {
            0
        };

        let full_move_count = if parts.len() > 5 {
            // full-move count
            parts[5].parse::<usize>().unwrap_or(1)
            // if let Ok(value) = parts[5].parse::<usize>() {
            //     value
            // } else {
            //     0 // TODO - Err ?
            // }
        } else {
            1
        };

        let bitmaps = to_bitmaps(&squares);

        let end_game_material = abs_material(&from_fen(ROOT_FEN)) / 3;
        let hash = calc_hash(&squares, colour);
        let material = material(&squares);
        let rep = HashMap::from([(hash, 1)]);

        Board {
            squares,
            bitmaps,
            colour,
            can_castle,
            end_game_material,
            log_bms: vec![],
            move_log,
            material,
            hash,
            half_move_clock,
            full_move_count,
            rep,
        }
    }

    pub fn to_fen(&self) -> String {
        let mut s = String::new();
        for y in (0..=7).rev() {
            let mut n = 0;
            for x in (0..=7).rev() {
                let idx = x * 8 + y;
                if self[idx] == Nil {
                    n += 1;
                } else {
                    if n > 0 {
                        s.push_str(format!("{}", n).as_str());
                        n = 0;
                    }
                    s.push(self[idx].to_ascii())
                }
            }
            if n > 0 {
                s.push_str(format!("{}", n).as_str())
            }

            if y != 0 {
                s.push('/')
            }
        }
        s.push_str(if self.colour.is_white() { " w" } else { " b" });
        s.push(' ');

        let castling_rights: String = ['K', 'Q', 'k', 'q']
            .into_iter()
            .zip([CASTLE_W_SHORT, CASTLE_W_LONG, CASTLE_B_SHORT, CASTLE_B_LONG])
            .filter(|(_, c)| self.can_castle & c != 0)
            .map(|(x, _)| x)
            .collect();
        s.push_str(if castling_rights.is_empty() {
            "-"
        } else {
            &castling_rights
        });

        // en passant sq
        s.push(' ');
        if let Some(last) = self.move_log.last() {
            if matches!(self[last.to()], Pawn(_)) && last.to().abs_diff(last.frm()) == 2 {
                let idx = last.to() as isize + if self.colour.is_white() { 1 } else { -1 };
                s.push_str(I2SQ[idx as usize])
            } else {
                s.push('-');
            }
        } else {
            s.push('-');
        }

        // reversible moves,move nr
        s.push_str(
            format!(
                " {} {}",
                self.half_moves() - 1,
                self.full_move_count + self.move_log.len() / 2
            )
            .as_str(),
        );
        s
    }

    #[rustfmt::skip]
    pub fn to_csv(&self) -> Vec<u8> {
        const CSV_SIZE: usize = 2 * 6 * 64 + 1 + 4 + 64 + 1;
        let mut v = Vec::with_capacity(CSV_SIZE);
        for p in [
            Pawn(White), Rook(White), Knight(White), Bishop(White), Queen(White), King(White),
            Pawn(Black), Rook(Black), Knight(Black), Bishop(Black), Queen(Black), King(Black),
        ] {
            for pb in &self.squares {
                v.push((p == *pb) as u8);
            }
        }

        // turn & castling rights
        v.push(self.colour.is_white() as u8);
        v.extend(
            [CASTLE_W_SHORT, CASTLE_W_LONG, CASTLE_B_SHORT, CASTLE_B_LONG]
                .map(|c| (self.can_castle & c != 0) as u8),
        );

        // en passant
        if let Some(last) = self.move_log.last() {
            if matches!(self[last.to()], Pawn(_)) && last.to().abs_diff(last.frm()) == 2 {
                let idx = last.to() as isize + if self.colour.is_white() { 1 } else { -1 };
                for i in 0..64 {
                    v.push((i == idx) as u8);
                }
            } else {
                v.resize(v.len() + 64, 0);
            }
        } else {
            v.resize(v.len() + 64, 0);
        };

        v
    }

    pub fn half_moves(&self) -> usize {
        self.half_move_clock + self.rep.iter().map(|(_, &v)| v).sum::<usize>()
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

    pub fn update(&mut self, m: &Move) {
        self.log_bms
            .push((self.bitmaps, self[m.to()], self.hash, self.can_castle));
        let hash;
        self[m.to()] = if m.castle() {
            let (x, y) = if m.to() <= 15 {
                (m.frm() - 24, m.frm() - 8) // short
            } else {
                (m.frm() + 32, m.frm() + 8) // long
            };

            self.bitmaps.kings |= 1 << m.to();
            self.bitmaps.kings ^= 1 << m.frm();
            self.bitmaps.pieces[self.colour as usize] |= 1 << m.to();
            self.bitmaps.pieces[self.colour as usize] ^= 1 << m.frm();
            self.bitmaps.pieces[self.colour as usize] |= 1 << y;
            self.bitmaps.pieces[self.colour as usize] ^= 1 << x;

            match self[m.frm()] {
                King(White) => self.can_castle &= !CASTLE_W_SHORT & !CASTLE_W_LONG,
                King(Black) => self.can_castle &= !CASTLE_B_SHORT & !CASTLE_B_LONG,
                _ => panic!("not castle..."),
            }

            hash = self[m.frm()].hashkey(m.to())
                ^ self[m.frm()].hashkey(m.frm())
                ^ self[x].hashkey(y)
                ^ self[x].hashkey(x);
            self[y] = self.squares[x]; // move rook
            self[x] = Nil;
            self[m.frm()]
        } else if m.transform() {
            self.bitmaps.pieces[self.colour as usize] |= 1 << m.to();
            self.bitmaps.pieces[self.colour as usize] ^= 1 << m.frm();
            self.bitmaps.pawns ^= 1 << m.frm();
            if let Rook(c) | Knight(c) | Bishop(c) | Queen(c) = self[m.to()] {
                self.bitmaps.pieces[c as usize] ^= 1 << m.to();
            }

            let p = m.ptransform(self.colour);
            hash =
                p.hashkey(m.to()) ^ self[m.frm()].hashkey(m.frm()) ^ self[m.to()].hashkey(m.to());
            p
        } else if m.en_passant() {
            // +9  +1 -7
            // +8   0 -8
            // +7  -1 -9
            let x = match m.to() > m.frm() {
                true => m.frm() + 8,  // west
                false => m.frm() - 8, // east
            };

            self.bitmaps.pieces[self.colour as usize] |= 1 << m.to();
            self.bitmaps.pieces[self.colour as usize] ^= 1 << m.frm();
            self.bitmaps.pieces[self.colour.opposite() as usize] ^= 1 << x;
            self.bitmaps.pawns |= 1 << m.to();
            self.bitmaps.pawns ^= 1 << m.frm();
            self.bitmaps.pawns ^= 1 << x;

            hash =
                self[m.frm()].hashkey(m.to()) ^ self[m.frm()].hashkey(m.frm()) ^ self[x].hashkey(x);
            self[x] = Nil;
            self[m.frm()]
        } else {
            self.bitmaps.pieces[self.colour as usize] |= 1 << m.to();
            self.bitmaps.pieces[self.colour as usize] ^= 1 << m.frm();
            match (self[m.frm()], self[m.to()]) {
                (Pawn(_), Pawn(c)) => {
                    self.bitmaps.pawns ^= 1 << m.frm();
                    self.bitmaps.pieces[c as usize] ^= 1 << m.to();
                }
                (Pawn(_), Rook(c) | Bishop(c) | Queen(c) | Knight(c)) => {
                    self.bitmaps.pawns |= 1 << m.to();
                    self.bitmaps.pawns ^= 1 << m.frm();
                    self.bitmaps.pieces[c as usize] ^= 1 << m.to();
                }
                (Pawn(_), _) => {
                    self.bitmaps.pawns |= 1 << m.to();
                    self.bitmaps.pawns ^= 1 << m.frm();
                }
                (King(_), Pawn(c)) => {
                    self.bitmaps.pawns ^= 1 << m.to();
                    self.bitmaps.kings |= 1 << m.to();
                    self.bitmaps.kings ^= 1 << m.frm();
                    self.bitmaps.pieces[c as usize] ^= 1 << m.to();
                }
                (King(_), Rook(c) | Bishop(c) | Queen(c) | Knight(c)) => {
                    self.bitmaps.kings |= 1 << m.to();
                    self.bitmaps.kings ^= 1 << m.frm();
                    self.bitmaps.pieces[c as usize] ^= 1 << m.to();
                }
                (King(_), _) => {
                    self.bitmaps.kings |= 1 << m.to();
                    self.bitmaps.kings ^= 1 << m.frm();
                }
                (_, Nil) => (),
                (_, Pawn(c)) => {
                    self.bitmaps.pawns ^= 1 << m.to();
                    self.bitmaps.pieces[c as usize] ^= 1 << m.to()
                }
                (_, Rook(c) | Knight(c) | Queen(c) | Bishop(c)) => {
                    self.bitmaps.pieces[c as usize] ^= 1 << m.to()
                }
                _ => (),
            }

            hash = self[m.frm()].hashkey(m.to())
                ^ self[m.frm()].hashkey(m.frm())
                ^ self[m.to()].hashkey(m.to());
            self[m.frm()]
        };
        self[m.frm()] = Nil;
        self.material += m.val;
        self.rep_inc();
        self.hash ^= hash ^ WHITE_HASH;
        // self.bitmaps = self.board.to_bitmaps();
        self.colour.flip();
    }

    pub fn backdate(&mut self, m: &Move) {
        let bms = self.log_bms.pop().unwrap();
        let capture;
        (self.bitmaps, capture, self.hash, self.can_castle) = bms;
        self.colour.flip();
        //self.hash ^= m.hash ^ WHITE_HASH;
        self.rep_dec();
        if m.castle() {
            let (frm, to) = if m.to() <= 15 {
                (m.frm() - 24, m.frm() - 8) // short
            } else {
                (m.frm() + 32, m.frm() + 8) // long
            };
            self[frm] = self.squares[to]; // move rook
            self[to] = Nil;
        }
        self[m.frm()] = if m.transform() {
            Pawn(self.colour)
        } else {
            self[m.to()]
        };
        self[m.to()] = capture;

        if m.en_passant() {
            let x = match m.to() > m.frm() {
                true => m.frm() + 8,  // west
                false => m.frm() - 8, // east
            };
            self[x] = match self.squares[m.frm()] {
                Pawn(White) => Pawn(Black),
                Pawn(Black) => Pawn(White),
                _ => unreachable!(),
            }
        }

        self.material -= m.val;
    }

    pub fn eval(&self) -> i16 {
        let s = self.material + self.score_pawn_structure() + self.mobility();
        if self.colour.is_white() { s } else { -s }
        //s * (2 * (colour as i16) - 1)
    }

    pub const fn is_end_game(&self) -> bool {
        abs_material(&self.squares) < self.end_game_material
    }

    pub const fn to_bitmaps(&self) -> Bitmaps {
        to_bitmaps(&self.squares)
    }

    pub fn score_pawn_structure(&self) -> i16 {
        let mut pen: i16 = 0;
        let bm: [u64; 2] = [
            self.bitmaps.pawns & self.bitmaps.pieces[White as usize],
            self.bitmaps.pawns & self.bitmaps.pieces[Black as usize],
        ];
        for (i, &p) in [Pawn(White), Pawn(Black)].iter().enumerate() {
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
            pen += if p == Pawn(White) { -x } else { x };
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

    pub fn mobility(&self) -> i16 {
        self.count_moves(White) as i16 - self.count_moves(Black) as i16
    }

    // true if !colour side can capture colour king
    pub fn in_check(&self, colour: Colour) -> bool {
        let bm_king = self.bitmaps.kings & self.bitmaps.pieces[colour as usize];
        let bm_board = self.bitmaps.pieces[Black as usize] | self.bitmaps.pieces[White as usize];
        let opp = colour.opposite() as usize;
        self.squares
            .iter()
            .enumerate()
            .filter(|(frm, _)| 1 << frm & self.bitmaps.pieces[opp] != 0)
            .any(|(frm, &p)| match p {
                Knight(_) => BM_KNIGHT_MOVES[frm] & bm_king != 0,
                King(_) => BM_KING_MOVES[frm] & bm_king != 0,
                Pawn(_) => BM_PAWN_CAPTURES[opp][frm] & bm_king != 0,
                Rook(_) => ray_check(frm, BM_ROOK_MOVES[frm], bm_board, bm_king),
                Bishop(_) => ray_check(frm, BM_BISHOP_MOVES[frm], bm_board, bm_king),
                Queen(_) => ray_check(frm, BM_QUEEN_MOVES[frm], bm_board, bm_king),
                _ => false,
            })
    }

    pub fn moves(&self, in_check: bool, end_game: bool, last: Option<&Move>) -> Vec<Move> {
        let last = if let Some(m) = last { m } else { &NULL_MOVE };

        let mut v = Vec::with_capacity(50);
        self.squares
            .iter()
            .enumerate()
            .filter(|(frm, _)| 1 << frm & self.bitmaps.pieces[self.colour as usize] != 0)
            .for_each(|(frm, &p)| match p {
                Knight(_) => self.knight_moves(&mut v, frm),
                King(_) => self.king_moves(&mut v, frm, end_game, in_check),
                Pawn(_) => self.pawn_moves(&mut v, frm, last),
                Rook(_) => self.ray_moves(&mut v, frm, BM_ROOK_MOVES[frm]),
                Bishop(_) => self.ray_moves(&mut v, frm, BM_BISHOP_MOVES[frm]),
                Queen(_) => self.ray_moves(&mut v, frm, BM_QUEEN_MOVES[frm]),
                _ => (),
            });
        v
    }

    fn knight_moves(&self, v: &mut Vec<Move>, frm: usize) {
        let mut b = BM_KNIGHT_MOVES[frm] & !self.bitmaps.pieces[self.colour as usize];
        while b != 0 {
            let to = b.trailing_zeros();
            b &= !(1 << to);

            v.push(Move {
                data: pack_data(false, false, Nil, frm, to as usize),
                val: self[frm].val(to as usize)
                    - self[frm].val(frm)
                    - self[to as usize].val(to as usize),
            })
        }
    }

    fn ray_moves(&self, v: &mut Vec<Move>, frm: usize, moves: u64) {
        let bm_board = self.bitmaps.pieces[White as usize] | self.bitmaps.pieces[Black as usize];
        let bl = bm_blockers(frm, moves & bm_board);

        let mut b = moves & !bl & !self.bitmaps.pieces[self.colour as usize];
        while b != 0 {
            let to = b.trailing_zeros();
            b &= !(1 << to);
            v.push(Move {
                data: pack_data(false, false, Nil, frm, to as usize),
                val: self[frm].val(to as usize)
                    - self[frm].val(frm)
                    - self[to as usize].val(to as usize),
            })
        }
    }

    fn pawn_moves(&self, v: &mut Vec<Move>, frm: usize, last: &Move) {
        let bm_board = self.bitmaps.pieces[White as usize] | self.bitmaps.pieces[Black as usize];
        let cap = BM_PAWN_CAPTURES[self.colour as usize][frm]
            & self.bitmaps.pieces[self.colour.opposite() as usize];
        let step1: u64 = BM_PAWN_STEP1[self.colour as usize][frm] & !bm_board;
        let step2: u64 = if self.colour.is_white() {
            step1 << 1
        } else {
            step1 >> 1
        };
        let step2: u64 = step2 & BM_PAWN_STEP2[self.colour as usize][frm] & !bm_board;

        let mut b = cap | step1 | step2;
        while b != 0 {
            let to = b.trailing_zeros();
            b &= !(1 << to);

            match to % 8 {
                0 | 7 => v.extend([
                    Move {
                        data: pack_data(false, false, Queen(self.colour), frm, to as usize),
                        val: Piece::Queen(self.colour).val(to as usize)
                            - self[frm].val(frm)
                            - self[to as usize].val(to as usize),
                    },
                    Move {
                        data: pack_data(false, false, Rook(self.colour), frm, to as usize),
                        val: Piece::Rook(self.colour).val(to as usize)
                            - self[frm].val(frm)
                            - self[to as usize].val(to as usize),
                    },
                    Move {
                        data: pack_data(false, false, Knight(self.colour), frm, to as usize),
                        val: Piece::Knight(self.colour).val(to as usize)
                            - self[frm].val(frm)
                            - self[to as usize].val(to as usize),
                    },
                    Move {
                        data: pack_data(false, false, Bishop(self.colour), frm, to as usize),
                        val: Piece::Bishop(self.colour).val(to as usize)
                            - self[frm].val(frm)
                            - self[to as usize].val(to as usize),
                    },
                ]),
                _ => v.push(Move {
                    data: pack_data(false, false, Nil, frm, to as usize),
                    val: self[frm].val(to as usize)
                        - self[frm].val(frm)
                        - self[to as usize].val(to as usize),
                }),
            }
        }

        // en passant
        if matches!(self[last.to()], Pawn(_)) && last.to().abs_diff(last.frm()) == 2 {
            // square attacked if last move was a step-2 pawn move
            let idx = last.frm() as isize - 2 * self.colour as isize + 1;

            let mut b = BM_PAWN_CAPTURES[self.colour as usize][frm] & 1 << idx;
            while b != 0 {
                let to = b.trailing_zeros();
                b &= !(1 << to);

                v.push(Move {
                    data: pack_data(false, true, Nil, frm, to as usize),
                    val: self[frm].val(to as usize)
                        - self[frm].val(frm)
                        - self[last.to()].val(last.to()),
                });
            }
        }
    }

    fn king_moves(&self, v: &mut Vec<Move>, frm: usize, end_game: bool, in_check: bool) {
        let bm_board = self.bitmaps.pieces[White as usize] | self.bitmaps.pieces[Black as usize];
        // change king valuation in end_game
        let p = match (self[frm], end_game) {
            (King(White), true) => King(Black),
            (King(Black), true) => King(White),
            (_, false) => self[frm],
            _ => panic!(),
        };

        // castling
        // check squares between K & R unoccupied
        const WSHORT: u64 = 1 << 8 | 1 << 16;
        const WLONG: u64 = 1 << 32 | 1 << 40 | 1 << 48;
        const BSHORT: u64 = 1 << 15 | 1 << 23;
        const BLONG: u64 = 1 << 55 | 1 << 47 | 1 << 39;

        let mut b = BM_KING_MOVES[frm] & !self.bitmaps.pieces[self.colour as usize];
        while b != 0 {
            let to = b.trailing_zeros();
            b &= !(1 << to);

            v.push(Move {
                data: pack_data(false, false, Nil, frm, to as usize),
                val: p.val(to as usize) - p.val(frm) - self[to as usize].val(to as usize),
            })
        }

        [
            (
                self.can_castle & CASTLE_W_SHORT != 0
                    && frm == 24
                    && !in_check
                    && self[0] == Rook(White)
                    && bm_board & WSHORT == 0,
                Rook(White),
                8,
                0,
                16,
            ),
            (
                self.can_castle & CASTLE_W_LONG != 0
                    && frm == 24
                    && !in_check
                    && self[56] == Rook(White)
                    && bm_board & WLONG == 0,
                Rook(White),
                40,
                56,
                32,
            ),
            (
                self.can_castle & CASTLE_B_SHORT != 0
                    && frm == 31
                    && !in_check
                    && self[7] == Rook(Black)
                    && bm_board & BSHORT == 0,
                Rook(Black),
                15,
                7,
                23,
            ),
            (
                self.can_castle & CASTLE_B_LONG != 0
                    && frm == 31
                    && !in_check
                    && self[63] == Rook(Black)
                    && bm_board & BLONG == 0,
                Rook(Black),
                47,
                63,
                39,
            ),
        ]
        .iter()
        .filter(|(c, _, _, _, _)| *c)
        .for_each(|(_, r, to, rfrm, rto)| {
            v.push(Move {
                data: pack_data(true, false, Nil, frm, *to as usize),
                val: p.val(*to as usize) - p.val(frm) + r.val(*rto) - r.val(*rfrm),
            })
        })
    }

    // count pseudo legal moves - ignoring en passant & castling
    fn count_moves(&self, colour: Colour) -> u32 {
        let bm_board = self.bitmaps.pieces[White as usize] | self.bitmaps.pieces[Black as usize];
        let bm_own = self.bitmaps.pieces[colour as usize];
        let bm_opp = self.bitmaps.pieces[colour.opposite() as usize];

        self.squares
            .iter()
            .enumerate()
            .filter(|(frm, _)| 1 << frm & self.bitmaps.pieces[colour as usize] != 0)
            .map(|(frm, &p)| match p {
                Knight(_) => (BM_KNIGHT_MOVES[frm] & !bm_own).count_ones(),
                King(_) => (BM_KING_MOVES[frm] & !bm_own).count_ones(),
                Pawn(_) => count_pawn_moves(frm, bm_opp, bm_board, colour),
                Rook(_) => count_ray_moves(frm, BM_ROOK_MOVES[frm], bm_board, bm_own),
                Bishop(_) => count_ray_moves(frm, BM_BISHOP_MOVES[frm], bm_board, bm_own),
                Queen(_) => count_ray_moves(frm, BM_QUEEN_MOVES[frm], bm_board, bm_own),
                _ => 0,
            })
            .sum()
    }
}

// +9  +1 -7
// +8   0 -8
// +7  -1 -9

fn count_pawn_moves(frm: usize, bm_opp: u64, bm_board: u64, colour: Colour) -> u32 {
    // TODO  - calc all at the same time;
    let cap = BM_PAWN_CAPTURES[colour as usize][frm] & bm_opp;
    let step1 = BM_PAWN_STEP1[colour as usize][frm] & !bm_board;
    let step2 = if colour.is_white() {
        step1 << 1
    } else {
        step1 >> 1
    };
    let step2 = step2 & BM_PAWN_STEP2[colour as usize][frm] & !bm_board;
    (cap | step1 | step2).count_ones()
}

fn count_ray_moves(frm: usize, moves: u64, bm_board: u64, bm_own: u64) -> u32 {
    (moves & !bm_own & !bm_blockers(frm, moves & bm_board)).count_ones()
}

fn ray_check(frm: usize, moves: u64, bm_board: u64, bm_king: u64) -> bool {
    moves & bm_king & !bm_blockers(frm, moves & bm_board) != 0
}

const fn to_bitmaps(squares: &[Piece]) -> Bitmaps {
    let mut bm = Bitmaps {
        pieces: [0, 0],
        pawns: 0,
        kings: 0,
    };
    let mut i = 0;
    while i < squares.len() {
        match squares[i] {
            Rook(c) | Knight(c) | Bishop(c) | Queen(c) => bm.pieces[c as usize] |= 1 << i,
            Pawn(c) => {
                bm.pieces[c as usize] |= 1 << i;
                bm.pawns |= 1 << i
            }
            King(c) => {
                bm.pieces[c as usize] |= 1 << i;
                bm.kings |= 1 << i
            }
            Nil => (),
        }
        i += 1;
    }
    bm
}

pub const fn material(squares: &[Piece]) -> i16 {
    let mut i = 0;
    let mut val: i16 = 0;
    while i < squares.len() {
        val += squares[i].val(i);
        i += 1;
    }
    val
}

pub const fn abs_material(squares: &[Piece]) -> i16 {
    let mut i = 0;
    let mut val: i16 = 0;
    while i < squares.len() {
        val += squares[i].val(i).abs();
        i += 1;
    }
    val
}

pub const fn calc_hash(squares: &[Piece], colour: Colour) -> u64 {
    let mut key = match colour {
        Colour::White => WHITE_HASH,
        Colour::Black => 0,
    };

    let mut i = 0;
    while i < squares.len() {
        match squares[i] {
            Piece::Nil => (),
            _ => key ^= squares[i].hashkey(i),
        };
        i += 1;
    }
    key
}

fn from_fen(s: &str) -> [Piece; 64] {
    let mut squares = [Nil; 64];
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
            squares[q] = Piece::from_ascii(c);
        }
    }
    squares
}
