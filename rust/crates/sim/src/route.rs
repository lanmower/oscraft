//! Waypoint routes. A route is the ordered list of points a player's units walk for one lane id; a
//! spawn effect whose p3 carries a lane id sends the unit it spawns along that player's route for the
//! lane: it attack-moves to each waypoint in turn, advancing once within `ROUTE_REACH_MILLI` of the
//! current one, and after the last waypoint marches on the nearest hostile building. An order issued
//! to the unit by anything else ends the route. Lane and step are per-unit sim state, hashed and saved.

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::loader::{LoadError, CELL_MILLI};
use crate::*;

/// Waypoint record size in the ROUT section: player, lane, count, reserved, then 64 x/y pairs.
const ROUT_RECORD: usize = 8 + MAX_ROUTE_POINTS * 8;
pub const MAX_ROUTE_POINTS: usize = 64;
pub const MAX_ROUTE_LANE: u16 = 3;
/// Distance to a waypoint, in milli-units, at which a unit moves on to the next one.
pub const ROUTE_REACH_MILLI: i32 = 3000;
/// Bit of a spawn effect's p3 where the lane id starts; the id takes the top two bits.
pub const LANE_SHIFT: u32 = 30;

/// The waypoints of one lane for one player, in milli-units.
#[derive(Clone, Debug, Default)]
pub struct Route {
    pub player: u16,
    pub lane: u16,
    pub points: Vec<(i32, i32)>,
}

/// The lane id a spawn effect's p3 carries, 0 for none.
pub(crate) fn lane_of_spec(spec: i32) -> u8 {
    ((spec as u32) >> LANE_SHIFT) as u8
}

/// Reads the ROUT section: `(off, count)` locates it in `bytes`; `w` x `h` is the grid and `flags` its cells.
pub(crate) fn parse_routes(bytes: &[u8], (off, count): (usize, usize), w: usize, h: usize, flags: &[u8]) -> Result<Vec<Route>, LoadError> {
    let mut out: Vec<Route> = Vec::with_capacity(count);
    for n in 0..count {
        let at = off + n * ROUT_RECORD;
        let rec = bytes.get(at..at + ROUT_RECORD).ok_or(LoadError::Truncated("ROUT record"))?;
        let u16_at = |k: usize| u16::from_le_bytes([rec[k], rec[k + 1]]);
        let u32_at = |k: usize| u32::from_le_bytes([rec[k], rec[k + 1], rec[k + 2], rec[k + 3]]);
        let (player, lane, points) = (u16_at(0), u16_at(2), u16_at(4) as usize);
        if player as usize >= MAX_PLAYERS || lane == 0 || lane > MAX_ROUTE_LANE || points == 0 || points > MAX_ROUTE_POINTS {
            return Err(LoadError::Range("route player, lane or point count"));
        }
        if out.iter().any(|r| r.player == player && r.lane == lane) {
            return Err(LoadError::Range("route repeats a player and lane"));
        }
        let mut list = Vec::with_capacity(points);
        for k in 0..MAX_ROUTE_POINTS {
            let (x, y) = (u32_at(8 + k * 8) as usize, u32_at(12 + k * 8) as usize);
            if k >= points {
                if x != 0 || y != 0 {
                    return Err(LoadError::Range("route unused point is not zero"));
                }
                continue;
            }
            let (cx, cy) = (x / CELL_MILLI as usize, y / CELL_MILLI as usize);
            if cx >= w || cy >= h || flags[cy * w + cx] & 1 != 0 {
                return Err(LoadError::Range("route point off the grid or on a blocked cell"));
            }
            list.push((x as i32, y as i32));
        }
        out.push(Route { player, lane, points: list });
    }
    Ok(out)
}

/// Per-unit route progress: the lane id being followed (0 for none) and the index of the next waypoint.
#[derive(Clone, Debug)]
pub(crate) struct RouteState {
    pub(crate) lane: Vec<u8>,
    pub(crate) step: Vec<u8>,
}

impl RouteState {
    pub(crate) fn new(cap: usize) -> RouteState {
        RouteState { lane: vec![0; cap], step: vec![0; cap] }
    }
}

impl Sim {
    pub(crate) fn route_clear(&mut self, i: usize) {
        self.routes.lane[i] = 0;
        self.routes.step[i] = 0;
    }

    fn route_point(&self, player: u8, lane: u8, step: usize) -> Option<(i32, i32, bool)> {
        let route = self.cat.routes.iter().find(|r| r.player == player as u16 && r.lane == lane as u16)?;
        let (x, y) = *route.points.get(step)?;
        Some((fixed::from_milli(x), fixed::from_milli(y), step + 1 == route.points.len()))
    }

    /// Sends unit `j` of `player` along that player's route for `lane`. False when the player has no
    /// such route or the first order is refused; the unit then keeps no route.
    pub(crate) fn start_route(&mut self, j: usize, player: u8, lane: u8) -> bool {
        let Some((x, y, _)) = self.route_point(player, lane, 0) else {
            return false;
        };
        if self.issue_order_kind(j, ORDER_ATTACK_MOVE, x, y).is_err() {
            return false;
        }
        self.routes.lane[j] = lane;
        self.routes.step[j] = 0;
        true
    }

    /// Advances unit `i` along its route: on reaching the current waypoint, the next one is ordered, or
    /// after the last the unit marches on the nearest hostile building. A changed order ends the route.
    pub(crate) fn tick_route(&mut self, i: usize) {
        let lane = self.routes.lane[i];
        if lane == 0 {
            return;
        }
        let step = self.routes.step[i] as usize;
        let player = self.player[i];
        let Some((x, y, last)) = self.route_point(player, lane, step) else {
            self.route_clear(i);
            return;
        };
        if self.order_kind[i] != ORDER_ATTACK_MOVE || self.tgt_x[i] != x || self.tgt_y[i] != y {
            self.route_clear(i);
            return;
        }
        let reach = fixed::from_milli(ROUTE_REACH_MILLI) as i64;
        if dist2(self.pos_x[i], self.pos_y[i], x, y) > reach * reach {
            return;
        }
        if last {
            self.route_clear(i);
            self.march_to_enemy_base(i, player);
            return;
        }
        if let Some((nx, ny, _)) = self.route_point(player, lane, step + 1) {
            if self.issue_order_kind(i, ORDER_ATTACK_MOVE, nx, ny).is_ok() {
                self.routes.step[i] = (step + 1) as u8;
            }
        }
    }
}
