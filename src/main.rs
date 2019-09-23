use std::time::SystemTime;

fn main() {
    benchmark("movegen", || {
        println!("{}", move_gen(7));
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

type Index = u8;
type Bits = u16;

pub const WIN: [Bits; 8] = [0o421, 0o124, 0o700, 0o070, 0o007, 0o111, 0o222, 0o444];
pub const ALL_FIELDS: Bits = 0o777;

#[derive(Copy, Clone, Default)]
pub struct Pos {
    pub field: Index,
    pub square: Bits,
}

#[derive(Copy, Clone)]
pub struct Move {
    pos: Pos,
    valid_field: Option<Index>,
    field_status: FieldStatus,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum FieldStatus {
    Tied,
    Won(Index),
    None,
}

impl FieldStatus {
    pub fn blocked(self) -> bool {
        self != FieldStatus::None
    }

    pub fn won(self, p: usize) -> bool {
        self == FieldStatus::Won(p as _)
    }
}

impl Default for FieldStatus {
    fn default() -> Self {
        FieldStatus::None
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Bitboard {
    pub valid_field: Option<Index>,
    pub board: [[Bits; 9]; 2],
    pub turn: usize,
    field_status: [FieldStatus; 9],
    meta_field: Option<[Bits; 2]>,
    game_over: Option<bool>,
}

impl Bitboard {
    fn dirty(&mut self) {
        self.meta_field = None;
        self.game_over = None;
    }

    fn get(&self, p: usize, field: Index) -> Bits {
        let f = field as usize;
        unsafe { *self.board.get_unchecked(p).get_unchecked(f) }
    }

    fn get_mut(&mut self, p: usize, field: Index) -> &mut Bits {
        let f = field as usize;
        unsafe { self.board.get_unchecked_mut(p).get_unchecked_mut(f) }
    }

    fn get_fields(&self, field: Index) -> (Bits, Bits) {
        (self.get(0, field), self.get(1, field))
    }

    pub fn make_move(&mut self, pos: Pos) {
        let other = self.get(1 - self.turn, pos.field);
        let square = self.get_mut(self.turn, pos.field);
        *square |= pos.square;
        if is_won(*square) {
            self.field_status[pos.field as usize] = FieldStatus::Won(self.turn as _);
            self.valid_field = None;
        } else if is_tied(*square | other) {
            self.field_status[pos.field as usize] = FieldStatus::Tied;
            self.valid_field = None;
        } else {
            self.valid_field = Some(pos.square.trailing_zeros() as _);
        }
        self.turn = 1 - self.turn;
        self.dirty();
    }

    pub fn get_all_moves(&self, moves: &mut Vec<Move>) -> usize {
        let valid_field = self.valid_field;
        let available_fields = match valid_field {
            Some(field) => field..field + 1,
            _ => 0..9,
        };
        let mut n_moves = 0;
        for field in available_fields {
            let field_status = self.field_status[field as usize];
            if field_status.blocked() {
                continue;
            }
            let (white, black) = self.get_fields(field);
            let any = white | black;
            for square in 0..9 {
                let square = 1 << square;
                let taken = any & square != 0;
                if taken {
                    continue;
                }
                let pos = Pos { field, square };
                moves.push(Move {
                    pos,
                    valid_field,
                    field_status,
                });
                n_moves += 1;
            }
        }
        n_moves
    }

    pub fn undo_move(&mut self, mov: &Move) {
        let pos = mov.pos;
        *self.get_mut(1 - self.turn, pos.field) &= !pos.square;
        self.valid_field = mov.valid_field;
        self.field_status[pos.field as usize] = mov.field_status;
        self.turn = 1 - self.turn;
        self.dirty();
    }

    pub fn get_meta_field(&mut self) -> [u16; 2] {
        if let Some(field) = self.meta_field {
            return field;
        }
        let mut field = [0; 2];
        for p in 0..2 {
            field[p] = (0..9)
                .map(|i| (self.field_status[i].won(p) as u16) << (8 - i as u16))
                .fold(0, |x, y| x | y);
        }
        self.meta_field = Some(field);
        field
    }

    pub fn game_over(&mut self) -> bool {
        if let Some(game_over) = self.game_over {
            return game_over;
        }
        let sum = (0..9).fold(0, |acc, i| acc + self.board[0][i].count_ones());
        if sum < 9 {
            return false;
        }
        let meta_field = self.get_meta_field();
        let game_over = is_won(meta_field[0]) | is_won(meta_field[1]) || self.game_tied();
        self.game_over = Some(game_over);
        game_over
    }

    pub fn game_tied(&self) -> bool {
        (0..9).all(|i| self.field_status[i].blocked())
    }
}

pub fn is_tied(field: Bits) -> bool {
    field == ALL_FIELDS
}

pub fn is_won(field: Bits) -> bool {
    WIN.iter().any(|&w| field & w == w)
}

pub fn move_gen_impl(board: &mut Bitboard, depth: usize, moves: &mut Vec<Move>) -> usize {
    if board.game_over() {
        return 0;
    } else {
        let n_moves = board.get_all_moves(moves);
        let mut sum = n_moves;
        if depth > 0 {
            for _ in 1..=n_moves {
                let mov = moves.pop().unwrap();
                board.make_move(mov.pos);
                let n = move_gen_impl(board, depth - 1, moves);
                board.undo_move(&mov);
                sum += n;
            }
        } else {
            for _ in 0..n_moves {
                let _ = moves.pop();
            }
        }
        sum
    }
}

pub fn move_gen(depth: usize) -> usize {
    let mut bitboard = Default::default();
    let mut moves = Vec::with_capacity(1 << 16);
    move_gen_impl(&mut bitboard, depth, &mut moves)
}
