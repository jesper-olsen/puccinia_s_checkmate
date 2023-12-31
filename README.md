# Puccinia's Checkmate

Puccinia's Checkmate - a rusty chess library:
* Principle variation negamax search with alpha beta pruning (See Knuth).
* Transposition table to avoid re-searching cycles
* Evaluation based on material, pawn structure & mobility
* Checks draw by 3x repetition and 50 move rule
* Opening library

References:
* ["An Analysis of Alpha-Beta Pruning", Donald E. Knuth and Ronald W. Moore, Artificial Intelligence 6 (1975), 293-326](http://www-public.telecom-sudparis.eu/~gibson/Teaching/Teaching-ReadingMaterial/KnuthMoore75.pdf) 
* ["The Bratko-Kopec Experiment: A Comparison of Human and Computer Performance in Chess", D. Kopec and I Bratko](http://spider.sci.brooklyn.cuny.edu/~kopec)

Two example apps included - terminal CLI app (src/bin) and browser web application ([examples/spa](https://github.com/jesper-olsen/puccinia_s_checkmate/tree/main/examples/spa)).

Run CLI app like this: 

```
% cargo run --release -- --bin main -h 

Usage: main [OPTIONS]

Options:
  -n, --n <N>    break off search threshold - positions generated [default: 1000000]
  -d, --d <D>    max depth of regular search [default: 30]
  -m, --m <M>    number of (half) moves before stopping [default: -1]
  -w, --w        play white (human-computer)
  -b, --b        play black (human-computer)
  -l, --l        library bypass
  -k, --k        benchmark test positions - Bratko-Kopec / Kaufman
  -v, --v        verbose output
  -h, --help     Print help
  -V, --version  Print version

```

Run CLI app like this to play white:
```
% cargo run --release -- --bin main -w 

8 rnbqkbnr
7 pppppppp
6 ........
5 ........
4 ........
3 ........
2 PPPPPPPP
1 RNBQKBNR
  ABCDEFGH
Your Move (White):

8 rnbqkbnr
7 pppppppp
6 ........
5 ........
4 ...P....
3 ........
2 PPP.PPPP
1 RNBQKBNR
  ABCDEFGH
1. d4

8 rnbqkb.r
7 pppppppp
6 .....n..
5 ........
4 ...P....
3 ........
2 PPP.PPPP
1 RNBQKBNR
  ABCDEFGH
2. Nf6
Your Move (White):
```

Run CLI app like this to benchmark on Bratko-Kopec positions:
```
% cargo run --release --bin main -- -k -n 10000000

Position  1; Searched:   2530720, Score:  9995, Move (black): d6 d1 =  Qd1; Expected: Qd1+
Position  2; Searched:  47606262, Score:    50, Move (white): e4 e5 =   e5; Expected: d5
Position  3; Searched:  30176141, Score:   111, Move (black): h7 h5 =   h5; Expected: f5
Position  4; Searched:  35791689, Score:   684, Move (white): e5 e6 =   e6; Expected: e6
Position  5; Searched: 167519756, Score:    30, Move (white): e3 f4 =  Bf4; Expected: Nd5 a4
Position  6; Searched:  12076819, Score:   159, Move (white): g5 g6 =   g6; Expected: g6
Position  7; Searched:  14896339, Score:   180, Move (white): a3 e7 = Bxe7; Expected: Nf6
Position  8; Searched:  26587248, Score:   145, Move (white): f4 f5 =   f5; Expected: f5
Position  9; Searched:  10266332, Score:   484, Move (white): f1 d3 =  Bd3; Expected: f5
Position 10; Searched:  52663794, Score:   157, Move (black): b6 b5 =  Qb5; Expected: Ne5
Position 11; Searched: 125901588, Score:   241, Move (white): f2 f4 =   f4; Expected: f4
Position 12; Searched:  14641867, Score:   158, Move (black): d7 c6 =  Bc6; Expected: Bf5
Position 13; Searched:  52286797, Score:    48, Move (white): b2 b4 =   b4; Expected: b4
Position 14; Searched:  10068581, Score:   423, Move (white): d1 d2 =  Qd2; Expected: Qd2 Qe1
Position 15; Searched:  15262129, Score:   463, Move (white): f1 f6 = Rxf6; Expected: Qxg7+
Position 16; Searched:  23199111, Score:    10, Move (white): g5 e7 = Bxe7; Expected: Ne4
Position 17; Searched:  11139690, Score:  -197, Move (black): b7 b5 =   b5; Expected: h5
Position 18; Searched:  12075913, Score:  -236, Move (black): c5 a4 = Nxa4; Expected: Nb3
Position 19; Searched:  10144231, Score:   449, Move (black): e8 e4 = Rxe4; Expected: Rxe4
Position 20; Searched:  61817779, Score:   108, Move (white): e2 h5 =  Qh5; Expected: g4
Position 21; Searched:  13910112, Score:   -52, Move (white): f5 h6 =  Nh6; Expected: Nh6
Position 22; Searched:  52585934, Score:   -10, Move (black): f6 e8 =  Ne8; Expected: Bxe4
Position 23; Searched:  10497436, Score:  -294, Move (black): b7 b5 =   b5; Expected: f6
Position 24; Searched:  23343422, Score:   -54, Move (white): c3 b5 =  Nb5; Expected: f4

Correct: [1, 4, 6, 8, 11, 13, 14, 19, 21] 9/24
Points: 9.25
Time: 59387 ms => 2474 ms/position
Search total: 836989690; Time 59387 ms => 14093 nodes/ms
```
