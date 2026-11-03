use crate::arrays::Sim;
use crate::loader::Catalog;

const MILLI_PER_GAME_NUM: i64 = 125;
const MILLI_PER_GAME_DEN: i64 = 8;

pub fn acquire_milli(game: i32) -> i32 {
    (game as i64 * MILLI_PER_GAME_NUM / MILLI_PER_GAME_DEN) as i32
}

pub(crate) fn per_kind(cat: &Catalog) -> Vec<i32> {
    cat.unit_ext.iter().map(|ext| acquire_milli(ext.acquire as i32)).collect()
}

#[derive(Clone, Debug)]
pub(crate) struct ChaseState {
    pub(crate) target: Vec<i32>,
    pub(crate) ret_kind: Vec<u8>,
    pub(crate) ret_x: Vec<i32>,
    pub(crate) ret_y: Vec<i32>,
}

impl ChaseState {
    pub(crate) fn new(cap: usize) -> ChaseState {
        ChaseState {
            target: vec![-1; cap],
            ret_kind: vec![0; cap],
            ret_x: vec![0; cap],
            ret_y: vec![0; cap],
        }
    }
}

impl Sim {
    pub(crate) fn chase_clear(&mut self, i: usize) {
        self.chase.target[i] = -1;
    }

    pub(crate) fn chasing(&self, i: usize) -> bool {
        self.chase.target[i] >= 0
    }

    pub fn acquire_of(&self, i: usize) -> i32 {
        self.acquire.get(self.type_of[i] as usize).copied().unwrap_or(0)
    }

    pub fn chase_get(&self, i: usize) -> (i32, u8, i32, i32) {
        (self.chase.target[i], self.chase.ret_kind[i], self.chase.ret_x[i], self.chase.ret_y[i])
    }

    pub fn chase_set(&mut self, i: usize, target: i32, kind: u8, x: i32, y: i32) {
        self.chase.target[i] = target;
        self.chase.ret_kind[i] = kind;
        self.chase.ret_x[i] = x;
        self.chase.ret_y[i] = y;
    }
}

impl crate::arrays::Sim {
    pub fn set_acquire(&mut self, name: &str, game: u32) -> Result<(), &'static str> {
        let k = self.cat.unit_index(name).ok_or("unknown unit")?;
        self.cat.unit_ext[k].acquire = game;
        self.acquire[k] = acquire_milli(game as i32);
        self.cat_digest = crate::hash::catalog_digest(&self.cat, &self.grid);
        Ok(())
    }
}
