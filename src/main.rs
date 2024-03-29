use std::time::SystemTime;

use once_cell::sync::Lazy;

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

pub static IS_WON: Lazy<Vec<bool>> = Lazy::new(|| {
    (0..1024)
        .map(|field| WIN.iter().any(|&w| field & w == w))
        .collect()
});

#[repr(packed)]
#[derive(Copy, Clone, Default)]
pub struct Pos {
    pub field: Index,
    pub square: Bits,
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct Move {
    pos: Pos,
    all_valid: bool,
    field_status: FieldStatus,
    meta_field: Bits,
    n_blocked: u8,
}

#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum FieldStatus {
    Won0 = 0,
    Won1 = 1,
    Tied,
    None,
}

impl FieldStatus {
    pub fn blocked(self) -> bool {
        self != FieldStatus::None
    }

    pub fn won(self, p: usize) -> bool {
        (self as u8) as usize == p
    }
}

impl Default for FieldStatus {
    fn default() -> Self {
        FieldStatus::None
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Bitboard {
    valid_field: Option<Index>,
    board: [[Bits; 9]; 2],
    turn: usize,
    field_status: [FieldStatus; 9],
    meta_field: [Bits; 2],
    game_over: bool,
    n_blocked: u8,
}

impl Bitboard {
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

    fn get_field_status(&mut self, field: Index) -> FieldStatus {
        unsafe { *self.field_status.get_unchecked(field as usize) }
    }

    fn get_meta_field(&mut self, p: usize) -> Bits {
        unsafe { *self.meta_field.get_unchecked(p) }
    }

    fn set_field_status(&mut self, field: Index, status: FieldStatus) {
        unsafe { *self.field_status.get_unchecked_mut(field as usize) = status };
    }

    fn set_meta_field(&mut self, p: usize, meta_field: Bits) {
        unsafe { *self.meta_field.get_unchecked_mut(p) = meta_field };
    }

    pub fn make_move(&mut self, pos: Pos) {
        let square = self.get_mut(self.turn, pos.field);
        *square |= pos.square;
        let square = *square;
        if is_won(square) {
            self.set_field_status(pos.field, unsafe { std::mem::transmute(self.turn as u8) });
            self.valid_field = None;
            let meta = self.get_meta_field(self.turn) | (1 << pos.field as Bits);
            self.set_meta_field(self.turn, meta);
            self.n_blocked += 1;
            if self.n_blocked == 9 || is_won(meta) {
                self.game_over = true;
            }
        } else {
            let other = self.get(1 - self.turn, pos.field);
            if is_tied(square | other) {
                self.set_field_status(pos.field, FieldStatus::Tied);
                self.valid_field = None;
                self.n_blocked += 1;
                if self.n_blocked == 9 {
                    self.game_over = true;
                }
            } else {
                self.valid_field = Some(pos.square.trailing_zeros() as _);
            }
        }
        self.turn = 1 - self.turn;
    }

    pub fn get_all_moves<F: FnMut(&mut Bitboard, Move)>(&mut self, mut f: F) {
        let all_valid = self.valid_field.is_none();
        let available_fields = match self.valid_field {
            Some(field) => field..field + 1,
            _ => 0..9,
        };
        let meta_field = self.get_meta_field(self.turn);
        let n_blocked = self.n_blocked;
        for field in available_fields {
            let field_status = self.get_field_status(field);
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
                f(
                    self,
                    Move {
                        pos,
                        all_valid,
                        field_status,
                        meta_field,
                        n_blocked,
                    },
                );
            }
        }
    }

    pub fn undo_move(&mut self, mov: &Move) {
        let pos = mov.pos;
        self.turn = 1 - self.turn;
        *self.get_mut(self.turn, pos.field) &= !pos.square;
        self.valid_field = if mov.all_valid { None } else { Some(pos.field) };
        self.set_field_status(pos.field, mov.field_status);
        self.set_meta_field(self.turn, mov.meta_field);
        self.n_blocked = mov.n_blocked;
        self.game_over = false;
    }

    pub fn game_over(&self) -> bool {
        self.game_over
    }
}

pub fn is_tied(field: Bits) -> bool {
    field == ALL_FIELDS
}

pub fn is_won(field: Bits) -> bool {
    unsafe { *IS_WON.get_unchecked(field as usize) }
}

pub fn move_gen_impl(board: &mut Bitboard, depth: usize) -> usize {
    if board.game_over() {
        0
    } else {
        let mut sum = 0;
        if depth != 0 {
            board.get_all_moves(|b, mov| {
                b.make_move(mov.pos);
                sum += 1 + move_gen_impl(b, depth - 1);
                b.undo_move(&mov);
            });
        } else {
            board.get_all_moves(|_, _| {
                sum += 1;
            })
        }
        sum
    }
}

pub fn move_gen(depth: usize) -> usize {
    move_gen_impl(&mut Default::default(), depth)
}
