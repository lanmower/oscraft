use crate::arrays::Sim;
use crate::{Refusal, MAX_PLAYERS};

pub const CONTROLLER_OPEN: u8 = 0;
pub const CONTROLLER_HUMAN: u8 = 1;
pub const CONTROLLER_COMPUTER: u8 = 2;

pub const MOD_CONTROLLERS: [&str; 3] = ["open", "human", "computer"];

pub const MOD_ALLIANCES: [&str; 3] = ["enemy", "ally", "vision"];

pub fn controller_code(name: &str) -> Option<u8> {
    MOD_CONTROLLERS.iter().position(|k| *k == name).map(|k| k as u8)
}

impl Sim {
    pub fn team_of(&self, player: u8) -> Option<u8> {
        self.team.get(player as usize).copied()
    }

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

    pub fn controller(&self, player: u8) -> u8 {
        self.controller.get(player as usize).copied().unwrap_or(CONTROLLER_OPEN)
    }

    pub fn add_player(&mut self, player: u8, controller: u8) -> Result<(), Refusal> {
        if (player as usize) >= MAX_PLAYERS {
            return Err(self.reject(Refusal::InvalidPlayer));
        }
        self.controller[player as usize] = controller;
        Ok(())
    }

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
