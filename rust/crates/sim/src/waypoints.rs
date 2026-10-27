use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::loader::{LoadError, CELL_MILLI};
use crate::*;

const ROUT_RECORD: usize = 8 + MAX_WAYPOINTS * 8;
pub const MAX_WAYPOINTS: usize = 64;
pub const MAX_ROUTE_ID: u16 = 3;
const REQUEST_CAP: usize = 1024;

#[derive(Clone, Debug, Default)]
pub struct Route {
    pub player: u16,
    pub id: u16,
    pub points: Vec<(i32, i32)>,
}

pub(crate) fn parse_waypoints(bytes: &[u8], (off, count): (usize, usize), w: usize, h: usize, flags: &[u8]) -> Result<Vec<Route>, LoadError> {
    let mut out: Vec<Route> = Vec::with_capacity(count);
    for n in 0..count {
        let at = off + n * ROUT_RECORD;
        let rec = bytes.get(at..at + ROUT_RECORD).ok_or(LoadError::Truncated("ROUT record"))?;
        let u16_at = |k: usize| u16::from_le_bytes([rec[k], rec[k + 1]]);
        let u32_at = |k: usize| u32::from_le_bytes([rec[k], rec[k + 1], rec[k + 2], rec[k + 3]]);
        let (player, id, points) = (u16_at(0), u16_at(2), u16_at(4) as usize);
        if player as usize >= MAX_PLAYERS || id == 0 || id > MAX_ROUTE_ID || points == 0 || points > MAX_WAYPOINTS {
            return Err(LoadError::Range("route player, id or point count"));
        }
        if out.iter().any(|r| r.player == player && r.id == id) {
            return Err(LoadError::Range("route repeats a player and id"));
        }
        let mut list = Vec::with_capacity(points);
        for k in 0..MAX_WAYPOINTS {
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
        out.push(Route { player, id, points: list });
    }
    Ok(out)
}

#[derive(Clone, Debug)]
pub(crate) struct DestState {
    pub(crate) id: Vec<u8>,
    pub(crate) step: Vec<u8>,
    pub(crate) requests: Vec<(u32, u8)>,
}

impl DestState {
    pub(crate) fn new(cap: usize) -> DestState {
        DestState { id: vec![0; cap], step: vec![0; cap], requests: Vec::new() }
    }
}

impl Sim {
    pub(crate) fn waypoint_clear(&mut self, i: usize) {
        self.dests.id[i] = 0;
        self.dests.step[i] = 0;
    }

    pub(crate) fn waypoint_at(&self, player: u8, id: u8, step: usize) -> Option<(i32, i32, bool)> {
        let route = self.cat.routes.iter().find(|r| r.player == player as u16 && r.id == id as u16)?;
        let (x, y) = *route.points.get(step)?;
        Some((fixed::from_milli(x), fixed::from_milli(y), step + 1 == route.points.len()))
    }

    pub fn route_len(&self, player: u8, id: u8) -> usize {
        self.cat.routes.iter().find(|r| r.player == player as u16 && r.id == id as u16).map_or(0, |r| r.points.len())
    }

    pub fn route_point(&self, player: u8, id: u8, step: usize) -> (i32, i32) {
        self.waypoint_at(player, id, step).map_or((0, 0), |(x, y, _)| (x, y))
    }

    pub(crate) fn request_destination(&mut self, j: usize, player: u8) {
        if self.dests.requests.len() < REQUEST_CAP {
            self.dests.requests.push((j as u32, player));
        }
    }

    pub fn take_destination_requests(&mut self) -> Vec<(usize, u8)> {
        std::mem::take(&mut self.dests.requests).into_iter().map(|(u, p)| (u as usize, p)).collect()
    }

    pub(crate) fn walk_waypoint(&mut self, i: usize) {
        let id = self.dests.id[i];
        if id == 0 {
            return;
        }
        let step = self.dests.step[i] as usize;
        let player = self.player[i];
        let Some((x, y, last)) = self.waypoint_at(player, id, step) else {
            self.waypoint_clear(i);
            return;
        };
        if self.order_kind[i] != ORDER_ATTACK_MOVE || self.tgt_x[i] != x || self.tgt_y[i] != y {
            self.waypoint_clear(i);
            return;
        }
        let reach = fixed::from_milli(self.attributed.rules.waypoint_reach_milli) as i64;
        if dist2(self.pos_x[i], self.pos_y[i], x, y) > reach * reach {
            return;
        }
        if last {
            self.waypoint_clear(i);
            self.request_destination(i, player);
            return;
        }
        if let Some((nx, ny, _)) = self.waypoint_at(player, id, step + 1) {
            if self.issue_order_kind(i, ORDER_ATTACK_MOVE, nx, ny).is_ok() {
                self.dests.step[i] = (step + 1) as u8;
            }
        }
    }
}
