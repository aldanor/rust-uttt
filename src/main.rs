use std::time::SystemTime;

fn main() {
    let board = Bitboard::default();
    benchmark("movegen", || {
        println!("{}", move_gen(board, 7));
    });
}

fn benchmark<F>(name: &str, mut func: F)
where
    F: FnMut() -> (),
{
    println!(">>> Starting {}...", name);
    let time = SystemTime::now();
    func();
    let unwrap = time.elapsed().unwrap();
    println!(
        "<<< Finished. Elapsed: {}s {}ms",
        unwrap.as_secs(),
        unwrap.subsec_millis()
    );
}

pub const FIELD: usize = 0x1FF << 16;
pub const SQUARE: usize = 0xFFFF;
pub const ALL_FIELDS_LEGAL: usize = 0x1 << 25;

pub const WIN: [usize; 8] = [0o421, 0o124, 0o700, 0o070, 0o007, 0o111, 0o222, 0o444];
pub const ALL_FIELDS: usize = 0o777;

#[macro_export]
macro_rules! field {
    ($val:expr) => {
        ($val & FIELD) >> 16
    };
}

#[macro_export]
macro_rules! square {
    ($val:expr) => {
        $val & SQUARE
    };
}

#[derive(Copy, Clone, Debug)]
pub struct Bitboard {
    pub valid_field: usize,
    pub board: [[usize; 9]; 2],
    pub turn: usize,
    cached_meta_field: [usize; 2],
    cached_meta_field_dirty: bool,
    cached_game_over: bool,
    cached_game_over_dirty: bool,
}

impl Default for Bitboard {
    fn default() -> Self {
        Bitboard {
            valid_field: ALL_FIELDS,
            board: [[0; 9]; 2],
            turn: 0,
            cached_meta_field: [0; 2],
            cached_game_over: false,
            cached_meta_field_dirty: true,
            cached_game_over_dirty: true,
        }
    }
}

impl Bitboard {
    fn dirty(&mut self) {
        self.cached_game_over_dirty = true;
        self.cached_meta_field_dirty = true;
    }

    fn taken(&self, pos: usize) -> bool {
        (self.board[0][to_index(field!(pos))] | self.board[1][to_index(field!(pos))]) & square!(pos)
            != 0
    }

    pub fn make_move(&mut self, mov: usize) {
        let field = field!(mov);
        let square = square!(mov);
        self.board[self.turn][to_index(field)] |= square;
        if self.field_is_blocked(to_index(square)) || is_won(field) {
            self.valid_field = ALL_FIELDS;
        } else {
            self.valid_field = square;
        }
        self.turn = 1 - self.turn;
        self.dirty();
    }

    pub fn get_all_moves(&self) -> Vec<usize> {
        let mut list = Vec::new();

        for i in 0..9 {
            if (1 << i) & self.valid_field != 0 {
                for s in 0..9 {
                    let mov = (1 << s) | ((1 << i) << 16);
                    if !self.taken(mov) && !self.field_is_blocked(i) {
                        list.push(
                            mov | (if self.valid_field == 0o777 {
                                ALL_FIELDS_LEGAL
                            } else {
                                0
                            }),
                        );
                    }
                }
            }
        }
        return list;
    }

    pub fn undo_move(&mut self, mov: usize) {
        let field = field!(mov);
        let square = square!(mov);
        self.board[1 - self.turn][to_index(field)] &= !square;
        if mov & ALL_FIELDS_LEGAL != 0 {
            self.valid_field = ALL_FIELDS;
        } else {
            self.valid_field = field;
        }
        self.turn = 1 - self.turn;
        self.dirty();
    }

    fn field_is_blocked(&self, field: usize) -> bool {
        let white_field = self.board[0][field];
        let black_field = self.board[1][field];
        is_won(white_field) || is_won(black_field) || is_tied(white_field | black_field)
    }

    pub fn get_meta_field(&mut self) -> [usize; 2] {
        if !self.cached_meta_field_dirty {
            return self.cached_meta_field;
        }
        let mut field = [0; 2];
        for p in 0..2 {
            field[p] = (0..9)
                .map(|i| (is_won(self.board[p][i]) as usize) << (8 - i))
                .fold(0, |x, y| x | y);
        }
        self.cached_meta_field = field;
        self.cached_meta_field_dirty = false;
        field
    }

    pub fn game_over(&mut self) -> bool {
        if !self.cached_game_over_dirty {
            return self.cached_game_over;
        }

        let sum = (0..9).fold(0, |acc, i| acc + self.board[0][i].count_ones());
        if sum < 9 {
            return false;
        }
        let meta_field = self.get_meta_field();
        let ret = is_won(meta_field[0]) | is_won(meta_field[1]) || self.game_tied();
        self.cached_game_over = ret;
        self.cached_game_over_dirty = false;
        return ret;
    }

    pub fn game_tied(&self) -> bool {
        (0..9).all(|i| {
            is_won(self.board[0][i])
                || is_won(self.board[1][i])
                || is_tied(self.board[0][i] | self.board[1][i])
        })
    }
}

pub fn to_index(value: usize) -> usize {
    value.trailing_zeros() as _
}

pub fn is_tied(field: usize) -> bool {
    field == ALL_FIELDS
}

pub fn is_won(field: usize) -> bool {
    WIN.iter().any(|&w| field & w == w)
}

pub fn move_gen(mut board: Bitboard, depth: usize) -> usize {
    if board.game_over() {
        return 0;
    } else {
        let moves = board.get_all_moves();
        return moves.len()
            + if depth > 0 {
                moves
                    .iter()
                    .map(|mov| {
                        board.make_move(*mov);
                        let ret = move_gen(board, depth - 1);
                        board.undo_move(*mov);
                        ret
                    })
                    .sum()
            } else {
                0
            };
    }
}
