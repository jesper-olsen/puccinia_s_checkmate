#[rustfmt::skip]
pub const BRATKO_KOPEC: [(&str, &str); 24] = [
    //https://www.chessprogramming.org/Bratko-Kopec_Test
    // fen, turn-castling-enpassant, best move
    ("1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - -", "Qd1+"),
    ("3r1k2/4npp1/1ppr3p/p6P/P2PPPP1/1NR5/5K2/2R5 w - -", "d5"),
    ("2q1rr1k/3bbnnp/p2p1pp1/2pPp3/PpP1P1P1/1P2BNNP/2BQ1PRK/7R b - -", "f5"),
    ("rnbqkb1r/p3pppp/1p6/2ppP3/3N4/2P5/PPP1QPPP/R1B1KB1R w KQkq -", "e6"),
    ("r1b2rk1/2q1b1pp/p2ppn2/1p6/3QP3/1BN1B3/PPP3PP/R4RK1 w - -", "Nd5 a4"),
    ("2r3k1/pppR1pp1/4p3/4P1P1/5P2/1P4K1/P1P5/8 w - -", "g6"),
    ("1nk1r1r1/pp2n1pp/4p3/q2pPp1N/b1pP1P2/B1P2R2/2P1B1PP/R2Q2K1 w - -", "Nf6"),
    ("4b3/p3kp2/6p1/3pP2p/2pP1P2/4K1P1/P3N2P/8 w - -", "f5"),
    ("2kr1bnr/pbpq4/2n1pp2/3p3p/3P1P1B/2N2N1Q/PPP3PP/2KR1B1R w - -", "f5"),
    ("3rr1k1/pp3pp1/1qn2np1/8/3p4/PP1R1P2/2P1NQPP/R1B3K1 b - -", "Ne5"),
    ("2r1nrk1/p2q1ppp/bp1p4/n1pPp3/P1P1P3/2PBB1N1/4QPPP/R4RK1 w - -", "f4"),
    ("r3r1k1/ppqb1ppp/8/4p1NQ/8/2P5/PP3PPP/R3R1K1 b - -", "Bf5"),
    ("r2q1rk1/4bppp/p2p4/2pP4/3pP3/3Q4/PP1B1PPP/R3R1K1 w - -", "b4"),
    ("rnb2r1k/pp2p2p/2pp2p1/q2P1p2/8/1Pb2NP1/PB2PPBP/R2Q1RK1 w - -", "Qd2 Qe1"),
    ("2r3k1/1p2q1pp/2b1pr2/p1pp4/6Q1/1P1PP1R1/P1PN2PP/5RK1 w - -", "Qxg7+"),
    ("r1bqkb1r/4npp1/p1p4p/1p1pP1B1/8/1B6/PPPN1PPP/R2Q1RK1 w kq -", "Ne4"),
    ("r2q1rk1/1ppnbppp/p2p1nb1/3Pp3/2P1P1P1/2N2N1P/PPB1QP2/R1B2RK1 b - -", "h5"),
    ("r1bq1rk1/pp2ppbp/2np2p1/2n5/P3PP2/N1P2N2/1PB3PP/R1B1QRK1 b - -", "Nb3"),
    ("3rr3/2pq2pk/p2p1pnp/8/2QBPP2/1P6/P5PP/4RRK1 b - -", "Rxe4"),
    ("r4k2/pb2bp1r/1p1qp2p/3pNp2/3P1P2/2N3P1/PPP1Q2P/2KRR3 w - -", "g4"),
    ("3rn2k/ppb2rpp/2ppqp2/5N2/2P1P3/1P5Q/PB3PPP/3RR1K1 w - -", "Nh6"),
    ("2r2rk1/1bqnbpp1/1p1ppn1p/pP6/N1P1P3/P2B1N1P/1B2QPP1/R2R2K1 b - -", "Bxe4"),
    ("r1bqk2r/pp2bppp/2p5/3pP3/P2Q1P2/2N1B3/1PP3PP/R4RK1 b kq -", "f6"),
    ("r2qnrnk/p2b2b1/1p1p2pp/2pPpp2/1PP1P3/PRNBB3/3QNPPP/5RK1 w - -", "f4"),
];

#[rustfmt::skip]
pub const KAUFMAN: [(&str, &str); 25] = [
    // https://www.chessprogramming.org/Kaufman_Test
    // fen, turn-castling-enpassant, best move/alt move
    ("1rbq1rk1/p1b1nppp/1p2p3/8/1B1pN3/P2B4/1P3PPP/2RQ1R1K w - - bm", "Nf6+", ),
    ("3r2k1/p2r1p1p/1p2p1p1/q4n2/3P4/PQ5P/1P1RNPP1/3R2K1 b - - bm", "Nxd4", ),
    ("3r2k1/1p3ppp/2pq4/p1n5/P6P/1P6/1PB2QP1/1K2R3 w - - am", "Rd1", ),
    ("r1b1r1k1/1ppn1p1p/3pnqp1/8/p1P1P3/5P2/PbNQNBPP/1R2RB1K w - - bm", "Rxb2", ),
    ("2r4k/pB4bp/1p4p1/6q1/1P1n4/2N5/P4PPP/2R1Q1K1 b - - bm", "Qxc1", ),
    ("r5k1/3n1ppp/1p6/3p1p2/3P1B2/r3P2P/PR3PP1/2R3K1 b - - am", "Rxa2", ),
    ("2r2rk1/1bqnbpp1/1p1ppn1p/pP6/N1P1P3/P2B1N1P/1B2QPP1/R2R2K1 b - - bm", "Bxe4", ),
    ("5r1k/6pp/1n2Q3/4p3/8/7P/PP4PK/R1B1q3 b - - bm", "h6"),
    ("r3k2r/pbn2ppp/8/1P1pP3/P1qP4/5B2/3Q1PPP/R3K2R w KQkq - bm", "Be2", ),
    ("3r2k1/ppq2pp1/4p2p/3n3P/3N2P1/2P5/PP2QP2/K2R4 b - - bm", "Nxc3", ),
    ("q3rn1k/2QR4/pp2pp2/8/P1P5/1P4N1/6n1/6K1 w - - bm", "Nf5"),
    ("6k1/p3q2p/1nr3pB/8/3Q1P2/6P1/PP5P/3R2K1 b - - bm", "Rd6"),
    ("1r4k1/7p/5np1/3p3n/8/2NB4/7P/3N1RK1 w - - bm", "Nxd5"),
    ("1r2r1k1/p4p1p/6pB/q7/8/3Q2P1/PbP2PKP/1R3R2 w - - bm", "Rxb2", ),
    ("r2q1r1k/pb3p1p/2n1p2Q/5p2/8/3B2N1/PP3PPP/R3R1K1 w - - bm", "Bxf5", ),
    ("8/4p3/p2p4/2pP4/2P1P3/1P4k1/1P1K4/8 w - - bm", "b4"),
    ("1r1q1rk1/p1p2pbp/2pp1np1/6B1/4P3/2NQ4/PPP2PPP/3R1RK1 w - - bm", "e5", ),
    ("q4rk1/1n1Qbppp/2p5/1p2p3/1P2P3/2P4P/6P1/2B1NRK1 b - - bm", "Qc8", ),
    ("r2q1r1k/1b1nN2p/pp3pp1/8/Q7/PP5P/1BP2RPN/7K w - - bm", "Qxd7", ),
    ("8/5p2/pk2p3/4P2p/2b1pP1P/P3P2B/8/7K w - - bm", "Bg4"),
    ("8/2k5/4p3/1nb2p2/2K5/8/6B1/8 w - - bm", "Kxb5"),
    ("1B1b4/7K/1p6/1k6/8/8/8/8 w - - bm", "Ba7"),
    ("rn1q1rk1/1b2bppp/1pn1p3/p2pP3/3P4/P2BBN1P/1P1N1PP1/R2Q1RK1 b - - bm", "Ba6", ),
    ("8/p1ppk1p1/2n2p2/8/4B3/2P1KPP1/1P5P/8 w - - bm", "Bxc6"),
    ("8/3nk3/3pp3/1B6/8/3PPP2/4K3/8 w - - bm", "Bxd7"),
];

// Lasker position - test for transposition table - winning move Ka1-b1
pub const LASKER: [(&str, &str); 1] = [("8/k7/3p4/p2P1p2/P2P1P2/8/8/K7 w - -", "Kb1")];
