use crate::bitmaps::*;
use crate::val::*;
use crate::val::{Colour::*, Piece::*};
use std::fmt;

// bitpacking - 1st 12 bits (6+6) for from/to, remaining 4 bits for castling and
// pawn transforms & enpassant. Castling, en passant & transform are mutually exclusive.
const CASTLE_BIT: u16 = 1 << 12;
const EN_PASSANT_BIT: u16 = 1 << 13;
const TRANSFORM_BIT: u16 = 1 << 14;
const TO_SHIFT: u16 = 6;
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

// count pseudo legal moves - ignoring en passant & castling
pub fn count_moves(board: &[Piece; 64], colour: Colour, bm_white: u64, bm_black: u64) -> u32 {
    let (bm_own, bm_opp) = if colour.is_white() {
        (bm_white, bm_black)
    } else {
        (bm_black, bm_white)
    };
    let bm_board = bm_white | bm_black;

    board
        .iter()
        .enumerate()
        .map(|(frm, &p)| match p {
            Knight(c) if c == colour => (BM_KNIGHT_MOVES[frm] & !bm_own).count_ones(),
            King(c) if c == colour => (BM_KING_MOVES[frm] & !bm_own).count_ones(),
            Pawn(c) if c == colour => count_pawn_moves(frm, bm_opp, bm_board, colour),
            Rook(c) if c == colour => count_ray_moves(frm, BM_ROOK_MOVES[frm], bm_board, bm_own),
            Bishop(c) if c == colour => {
                count_ray_moves(frm, BM_BISHOP_MOVES[frm], bm_board, bm_own)
            }
            Queen(c) if c == colour => count_ray_moves(frm, BM_QUEEN_MOVES[frm], bm_board, bm_own),
            _ => 0,
        })
        .sum()
}

// +9  +1 -7
// +8   0 -8
// +7  -1 -9

fn count_pawn_moves(frm: usize, bm_opp: u64, bm_board: u64, colour: Colour) -> u32 {
    // TODO  - calc all at the same time;
    let cidx = if colour.is_white() { 0 } else { 1 };
    let cap = BM_PAWN_CAPTURES[cidx][frm] & bm_opp;
    let step1 = BM_PAWN_STEP1[cidx][frm] & !bm_board;
    let step2 = if colour.is_white() {
        step1 << 1
    } else {
        step1 >> 1
    };
    let step2 = step2 & BM_PAWN_STEP2[cidx][frm] & !bm_board;
    (cap | step1 | step2).count_ones()
}

fn count_ray_moves(frm: usize, moves: u64, bm_board: u64, bm_own: u64) -> u32 {
    let bl: u64 = bm2vec(moves & bm_board)
        .iter()
        .fold(0, |a, i| a | BM_BLOCKED[frm][*i]);
    (moves & !bl & !bm_own).count_ones()
}

// true if !colour side can capture colour king
pub fn in_check(
    board: &[Piece; 64],
    colour: Colour,
    bm_wking: u64,
    bm_bking: u64,
    bm_board: u64,
) -> bool {
    board.iter().enumerate().any(|(frm, &p)| match (colour, p) {
        (White, Knight(Black)) => (BM_KNIGHT_MOVES[frm] & bm_wking) != 0,
        (Black, Knight(White)) => (BM_KNIGHT_MOVES[frm] & bm_bking) != 0,
        (White, King(Black)) => (BM_KING_MOVES[frm] & bm_wking) != 0,
        (Black, King(White)) => (BM_KING_MOVES[frm] & bm_bking) != 0,
        (White, Pawn(Black)) => BM_PAWN_CAPTURES[1][frm] & bm_wking != 0,
        (Black, Pawn(White)) => BM_PAWN_CAPTURES[0][frm] & bm_bking != 0,
        (White, Rook(Black)) => ray_check(frm, BM_ROOK_MOVES[frm], bm_board, bm_wking),
        (Black, Rook(White)) => ray_check(frm, BM_ROOK_MOVES[frm], bm_board, bm_bking),
        (White, Bishop(Black)) => ray_check(frm, BM_BISHOP_MOVES[frm], bm_board, bm_wking),
        (Black, Bishop(White)) => ray_check(frm, BM_BISHOP_MOVES[frm], bm_board, bm_bking),
        (White, Queen(Black)) => ray_check(frm, BM_QUEEN_MOVES[frm], bm_board, bm_wking),
        (Black, Queen(White)) => ray_check(frm, BM_QUEEN_MOVES[frm], bm_board, bm_bking),
        _ => false,
    })
}

fn ray_check(frm: usize, moves: u64, bm_board: u64, bm_king: u64) -> bool {
    let bl: u64 = bm2vec(moves & bm_board)
        .iter()
        .fold(0, |a, i| a | BM_BLOCKED[frm][*i]);
    (moves & !bl & bm_king) != 0
}

struct Bitmaps {
    bm_board: u64,
    bm_own: u64,
    bm_opp: u64,
}

pub fn moves(
    board: &[Piece; 64],
    colour: Colour,
    in_check: bool,
    end_game: bool,
    can_castle: &[bool; 4],
    last: Option<&Move>,
    bm_white: u64,
    bm_black: u64,
) -> Vec<Move> {
    let (bm_own, bm_opp) = if colour.is_white() {
        (bm_white, bm_black)
    } else {
        (bm_black, bm_white)
    };
    let bm_board = bm_white | bm_black;
    let bitmaps = Bitmaps {
        bm_board,
        bm_own,
        bm_opp,
    };

    let last = if let Some(m) = last { m } else { &NULL_MOVE };

    let mut v = Vec::with_capacity(50);
    board.iter().enumerate().for_each(|(frm, &p)| match p {
        Knight(c) if c == colour => knight_moves(&mut v, board, frm, &bitmaps),
        King(c) if c == colour => {
            king_moves(&mut v, board, frm, &bitmaps, end_game, can_castle, in_check)
        }
        Pawn(c) if c == colour => pawn_moves(&mut v, board, frm, last, &bitmaps, colour),
        Rook(c) if c == colour => ray_moves(&mut v, board, frm, BM_ROOK_MOVES[frm], &bitmaps),
        Bishop(c) if c == colour => ray_moves(&mut v, board, frm, BM_BISHOP_MOVES[frm], &bitmaps),
        Queen(c) if c == colour => ray_moves(&mut v, board, frm, BM_QUEEN_MOVES[frm], &bitmaps),
        _ => (),
    });
    v
}

fn knight_moves(v: &mut Vec<Move>, board: &[Piece; 64], frm: usize, bitmaps: &Bitmaps) {
    v.extend(
        bm2vec(BM_KNIGHT_MOVES[frm] & !bitmaps.bm_own)
            .iter()
            .map(|&to| Move {
                data: pack_data(false, false, Nil, frm, to),
                val: board[frm].val(to) - board[frm].val(frm) - board[to].val(to),
            }),
    );
}

fn ray_moves(v: &mut Vec<Move>, board: &[Piece; 64], frm: usize, moves: u64, bitmaps: &Bitmaps) {
    let bl: u64 = bm2vec(moves & bitmaps.bm_board)
        .iter()
        .fold(0, |a, i| a | BM_BLOCKED[frm][*i]);
    v.extend(
        bm2vec(moves & !bl & !bitmaps.bm_own)
            .iter()
            .map(|&to| Move {
                data: pack_data(false, false, Nil, frm, to),
                val: board[frm].val(to) - board[frm].val(frm) - board[to].val(to),
            }),
    );
}

fn pawn_moves(
    v: &mut Vec<Move>,
    board: &[Piece; 64],
    frm: usize,
    last: &Move,
    bitmaps: &Bitmaps,
    colour: Colour,
) {
    let cidx = if colour.is_white() { 0 } else { 1 };
    let cap = BM_PAWN_CAPTURES[cidx][frm] & bitmaps.bm_opp;
    let step1: u64 = BM_PAWN_STEP1[cidx][frm] & !bitmaps.bm_board;
    let step2: u64 = if colour.is_white() {
        step1 << 1
    } else {
        step1 >> 1
    };
    let step2: u64 = step2 & BM_PAWN_STEP2[cidx][frm] & !bitmaps.bm_board;
    let vto = bm2vec(cap | step1 | step2);

    v.extend(vto.iter().flat_map(|&to| {
        match to % 8 {
            0 | 7 => vec![
                Move {
                    data: pack_data(false, false, Queen(colour), frm, to),
                    val: Piece::Queen(colour).val(to) - board[frm].val(frm) - board[to].val(to),
                },
                Move {
                    data: pack_data(false, false, Rook(colour), frm, to),
                    val: Piece::Rook(colour).val(to) - board[frm].val(frm) - board[to].val(to),
                },
                Move {
                    data: pack_data(false, false, Knight(colour), frm, to),
                    val: Piece::Knight(colour).val(to) - board[frm].val(frm) - board[to].val(to),
                },
                Move {
                    data: pack_data(false, false, Bishop(colour), frm, to),
                    val: Piece::Bishop(colour).val(to) - board[frm].val(frm) - board[to].val(to),
                },
            ]
            .into_iter(),
            _ => vec![Move {
                data: pack_data(false, false, Nil, frm, to),
                val: board[frm].val(to) - board[frm].val(frm) - board[to].val(to),
            }]
            .into_iter(),
        }
    }));

    // en passant
    if matches!(board[last.to()], Pawn(_)) && last.to().abs_diff(last.frm()) == 2 {
        // square attacked if last move was a step-2 pawn move
        let idx = last.frm() as isize + if colour.is_white() { -1 } else { 1 };

        v.extend(
            bm2vec(BM_PAWN_CAPTURES[cidx][frm] & 1 << idx)
                .iter()
                .map(|&to| Move {
                    data: pack_data(false, true, Nil, frm, to),
                    val: board[frm].val(to) - board[frm].val(frm) - board[last.to()].val(last.to()),
                }),
        );
    }
}

fn king_moves(
    v: &mut Vec<Move>,
    board: &[Piece; 64],
    frm: usize,
    bitmaps: &Bitmaps,
    end_game: bool,
    can_castle: &[bool; 4],
    in_check: bool,
) {
    // change king valuation in end_game
    let p = match (board[frm], end_game) {
        (King(White), true) => King(Black),
        (King(Black), true) => King(White),
        (_, false) => board[frm],
        _ => panic!(),
    };

    // castling
    // check squares between K & R unoccupied
    const WSHORT: u64 = 1 << 8 | 1 << 16;
    const WLONG: u64 = 1 << 32 | 1 << 40 | 1 << 48;
    const BSHORT: u64 = 1 << 15 | 1 << 23;
    const BLONG: u64 = 1 << 55 | 1 << 47 | 1 << 39;

    #[rustfmt::skip]
    let cc2 = [
        (can_castle[0] && frm == 24 && !in_check && board[0] == Rook(White) && bitmaps.bm_board & WSHORT == 0,
         Rook(White), 8, 0, 16,),
        (can_castle[1] && frm == 24 && !in_check && board[56] == Rook(White) && bitmaps.bm_board & WLONG == 0,
         Rook(White), 40, 56, 32,),
        (can_castle[2] && frm == 31 && !in_check && board[7] == Rook(Black) && bitmaps.bm_board & BSHORT == 0,
         Rook(Black), 15, 7, 23,),
        (can_castle[3] && frm == 31 && !in_check && board[63] == Rook(Black) && bitmaps.bm_board & BLONG == 0,
         Rook(Black), 47, 63, 39,),
    ];

    v.extend(
        bm2vec(BM_KING_MOVES[frm] & !bitmaps.bm_own)
            .iter()
            .map(|&to| Move {
                data: pack_data(false, false, Nil, frm, to),
                //castle: false,
                //en_passant: false,
                //transform: false,
                val: p.val(to) - p.val(frm) - board[to].val(to),
            })
            .chain(
                cc2.iter()
                    .filter(|(c, _, _, _, _)| *c)
                    .map(|(_, r, to, rfrm, rto)| Move {
                        data: pack_data(true, false, Nil, frm, *to as usize),
                        val: p.val(*to as usize) - p.val(frm) + r.val(*rto) - r.val(*rfrm),
                    }),
            ),
    );
}

pub const fn board2bm(board: &[Piece; 64]) -> (u64, u64) {
    let (mut w, mut b): (u64, u64) = (0, 0);
    let mut i = 0;
    while i < 64 {
        if let Rook(c) | Knight(c) | Bishop(c) | King(c) | Queen(c) | Pawn(c) = board[i] {
            if c.is_white() {
                w |= 1 << i
            } else {
                b |= 1 << i
            }
        }
        i += 1;
    }
    (w, b)
}

pub const fn board2bm_pawns(board: &[Piece; 64]) -> u64 {
    let mut b: u64 = 0;
    let mut i = 0;
    while i < 64 {
        if let Pawn(_) = board[i] {
            b |= 1 << i;
        }
        i += 1;
    }
    b
}
