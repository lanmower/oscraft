//! Teams, alliances, shared sight and player slots, as the mod API reaches them.

use crate::arrays::Sim;
use crate::{Refusal, MAX_PLAYERS};

/// Who commands a player slot. A slot nobody holds is `CONTROLLER_OPEN`.
pub const CONTROLLER_OPEN: u8 = 0;
pub const CONTROLLER_HUMAN: u8 = 1;
pub const CONTROLLER_COMPUTER: u8 = 2;

/// The controller names a mod may use, indexed by controller code.
pub const MOD_CONTROLLERS: [&str; 3] = ["open", "human", "computer"];

/// How two players relate: enemies, allies, or allies that also share sight.
pub const MOD_ALLIANCES: [&str; 3] = ["enemy", "ally", "vision"];

pub fn controller_code(name: &str) -> Option<u8> {
    MOD_CONTROLLERS.iter().position(|k| *k == name).map(|k| k as u8)
}

impl Sim {
    /// The team of `player`, or None past MAX_PLAYERS.
    pub fn team_of(&self, player: u8) -> Option<u8> {
        self.team.get(player as usize).copied()
    }

    /// The relation between `a` and `b` as a MOD_ALLIANCES name: "vision" when allied with shared sight, "ally" when
    /// allied or on one team, else "enemy". A player is its own ally. None past MAX_PLAYERS.
    pub fn relation(&self, a: u8, b: u8) -> Option<&'static str> {
        let (ai, bi) = (a as usize, b as usize);
        if ai >= MAX_PLAYERS || bi >= MAX_PLAYERS {
            return None;
        }
        if self.hostile(a, b) {
            return Some(MOD_ALLIANCES[0]);
        }
        Some(MOD_ALLIANCES[if self.vision[ai] & (1 << bi) != 0 { 2 } else { 1 }])
    }

    /// Relates players `a` and `b` as `kind` (an index into MOD_ALLIANCES), in both directions. "enemy" clears the
    /// alliance and shared sight, though two players on one team stay allies; "ally" allies them; "vision" allies them
    /// and shares sight. Refusal: InvalidPlayer past MAX_PLAYERS, nothing changed.
    pub fn set_relation(&mut self, a: u8, b: u8, kind: usize) -> Result<(), Refusal> {
        self.set_alliance(a, b, kind > 0)?;
        let (ai, bi) = (a as usize, b as usize);
        if ai != bi {
            crate::fog::bump_vision_generation();
            if kind == 2 {
                self.vision[ai] |= 1 << bi;
                self.vision[bi] |= 1 << ai;
            } else {
                self.vision[ai] &= !(1 << bi);
                self.vision[bi] &= !(1 << ai);
            }
        }
        Ok(())
    }

    /// Who commands slot `player` (CONTROLLER_*). Past MAX_PLAYERS is open.
    pub fn controller(&self, player: u8) -> u8 {
        self.controller.get(player as usize).copied().unwrap_or(CONTROLLER_OPEN)
    }

    /// Seats `controller` (a CONTROLLER_* code) in slot `player`. Refusal: InvalidPlayer past MAX_PLAYERS.
    pub fn add_player(&mut self, player: u8, controller: u8) -> Result<(), Refusal> {
        if (player as usize) >= MAX_PLAYERS {
            return Err(self.reject(Refusal::InvalidPlayer));
        }
        self.controller[player as usize] = controller;
        Ok(())
    }

    /// Empties slot `player`: it becomes open, returns to a team of its own, and breaks every alliance and shared sight.
    /// Its units stay where they are under the slot. Refusal: InvalidPlayer past MAX_PLAYERS.
    pub fn remove_player(&mut self, player: u8) -> Result<(), Refusal> {
        if (player as usize) >= MAX_PLAYERS {
            return Err(self.reject(Refusal::InvalidPlayer));
        }
        self.controller[player as usize] = CONTROLLER_OPEN;
        self.team[player as usize] = player;
        for q in 0..MAX_PLAYERS as u8 {
            self.set_relation(player, q, 0)?;
        }
        Ok(())
    }
}
