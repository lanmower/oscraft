use crate::arrays::Sim;
use crate::TICKS_PER_SECOND;

pub const FULL_TURN: i32 = 65536;
pub const FACE_TOLERANCE: i32 = FULL_TURN / 32;

const ATAN: [i32; 16] = [8192, 4836, 2555, 1297, 651, 326, 163, 81, 41, 20, 10, 5, 3, 1, 1, 0];

pub fn angle_of(dx: i64, dy: i64) -> Option<i32> {
    if dx == 0 && dy == 0 {
        return None;
    }
    let (mut x, mut y) = (dx.abs(), dy.abs());
    let mut a = 0i32;
    for (k, step) in ATAN.iter().enumerate() {
        if y >= 0 {
            let (nx, ny) = (x + (y >> k), y - (x >> k));
            x = nx;
            y = ny;
            a += step;
        } else {
            let (nx, ny) = (x - (y >> k), y + (x >> k));
            x = nx;
            y = ny;
            a -= step;
        }
    }
    let quarter = a.clamp(0, FULL_TURN / 4);
    let angle = match (dx < 0, dy < 0) {
        (false, false) => quarter,
        (true, false) => FULL_TURN / 2 - quarter,
        (true, true) => FULL_TURN / 2 + quarter,
        (false, true) => FULL_TURN - quarter,
    };
    Some(angle.rem_euclid(FULL_TURN))
}

pub fn angle_diff(to: i32, from: i32) -> i32 {
    let d = (to - from).rem_euclid(FULL_TURN);
    if d >= FULL_TURN / 2 { d - FULL_TURN } else { d }
}

impl Sim {
    pub fn set_turn_rate(&mut self, i: usize, deg_per_s: i32) -> bool {
        if i >= self.n {
            return false;
        }
        let per_tick = if deg_per_s <= 0 {
            0
        } else {
            ((i64::from(deg_per_s) * i64::from(FULL_TURN) / 360) / i64::from(TICKS_PER_SECOND)).clamp(1, i64::from(FULL_TURN / 2)) as i32
        };
        self.turn_step[i] = per_tick;
        true
    }

    pub fn turn_step_of(&self, i: usize) -> i32 {
        self.turn_step.get(i).copied().unwrap_or(0)
    }

    pub fn heading_of(&self, i: usize) -> i32 {
        self.heading.get(i).copied().unwrap_or(0)
    }

    pub fn set_heading(&mut self, i: usize, heading: i32) -> bool {
        if i >= self.n {
            return false;
        }
        self.heading[i] = heading.rem_euclid(FULL_TURN);
        true
    }

    pub(crate) fn turn_toward(&mut self, i: usize, target: i32) {
        let step = self.turn_step[i];
        let diff = angle_diff(target, self.heading[i]);
        self.heading[i] = if step == 0 || diff.abs() <= step {
            target.rem_euclid(FULL_TURN)
        } else {
            (self.heading[i] + diff.signum() * step).rem_euclid(FULL_TURN)
        };
    }

    pub(crate) fn face_move(&mut self, i: usize, px: i32, py: i32) {
        if self.turn_step[i] == 0 {
            return;
        }
        let (dx, dy) = (i64::from(self.pos_x[i]) - i64::from(px), i64::from(self.pos_y[i]) - i64::from(py));
        if let Some(bearing) = angle_of(dx, dy) {
            self.turn_toward(i, bearing);
        }
    }

    pub(crate) fn face_target(&mut self, i: usize, j: usize) -> bool {
        if self.turn_step[i] == 0 {
            return true;
        }
        let (dx, dy) = (i64::from(self.pos_x[j]) - i64::from(self.pos_x[i]), i64::from(self.pos_y[j]) - i64::from(self.pos_y[i]));
        let Some(bearing) = angle_of(dx, dy) else {
            return true;
        };
        self.turn_toward(i, bearing);
        angle_diff(bearing, self.heading[i]).abs() <= FACE_TOLERANCE
    }
}
