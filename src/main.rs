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

// bitboard.rs
pub const WHITE: usize = 0; // X
pub const BLACK: usize = 1; // O

pub const FIELD: usize = 0x1FF << 16;
pub const SQUARE: usize = 0xFFFF;
pub const ALL_FIELDS_LEGAL: usize = 0x1 << 25;

pub const DIAGS: [usize; 2] = [0o421, 0o124];
pub const ROWS: [usize; 3] = [0o700, 0o070, 0o007];
pub const COLS: [usize; 3] = [0o111, 0o222, 0o444];
pub const ALL_FIELDS: usize = 0o777;

pub const USIZE: u32 = (std::mem::size_of::<usize>() * 8 - 1) as u32;

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
        let white_field = self.board[WHITE][field];
        let black_field = self.board[BLACK][field];
        is_won(white_field) || is_won(black_field) || is_tied(white_field | black_field)
    }

    pub fn get_meta_field(&mut self) -> [usize; 2] {
        if !self.cached_meta_field_dirty {
            return self.cached_meta_field;
        }

        let mut field: [usize; 2] = [0; 2];
        for p in 0..2 {
            field[p] = (is_won(self.board[p][0]) as usize) << 8
                | (is_won(self.board[p][1]) as usize) << 7
                | (is_won(self.board[p][2]) as usize) << 6
                | (is_won(self.board[p][3]) as usize) << 5
                | (is_won(self.board[p][4]) as usize) << 4
                | (is_won(self.board[p][5]) as usize) << 3
                | (is_won(self.board[p][6]) as usize) << 2
                | (is_won(self.board[p][7]) as usize) << 1
                | (is_won(self.board[p][8]) as usize) << 0;
        }
        self.cached_meta_field = field;
        self.cached_meta_field_dirty = false;
        field
    }

    pub fn game_over(&mut self) -> bool {
        if !self.cached_game_over_dirty {
            return self.cached_game_over;
        }

        let mut sum = 0;
        for i in 0..9 {
            sum += self.board[0][i].count_ones();
        }
        if sum < 9 {
            return false;
        }
        let meta_field = self.get_meta_field();
        let ret = is_won(meta_field[WHITE]) | is_won(meta_field[BLACK]) || self.game_tied();
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

pub fn to_index(no: usize) -> usize {
    (USIZE - no.leading_zeros()) as usize
}

pub fn is_tied(field: usize) -> bool {
    field == ALL_FIELDS
}

pub fn is_won(field: usize) -> bool {
    (field & DIAGS[0]) == DIAGS[0]
        || (field & DIAGS[1]) == DIAGS[1]
        || (field & ROWS[0]) == ROWS[0]
        || (field & ROWS[1]) == ROWS[1]
        || (field & ROWS[2]) == ROWS[2]
        || (field & COLS[0]) == COLS[0]
        || (field & COLS[1]) == COLS[1]
        || (field & COLS[2]) == COLS[2]
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
