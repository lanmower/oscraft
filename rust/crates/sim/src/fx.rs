use crate::arrays::Sim;

pub const FX_CAP: usize = 4096;
const HIT_CAP: usize = 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct FxEvent {
    pub kind: String,
    pub x: i32,
    pub y: i32,
    pub source: i32,
    pub target: i32,
    pub team: i32,
    pub colour: String,
    pub duration: i32,
    pub text: String,
    pub scale: i32,
    pub name: String,
    pub value: i32,
}

impl FxEvent {
    pub fn new(kind: &str, x: i32, y: i32) -> FxEvent {
        FxEvent {
            kind: kind.to_owned(),
            x,
            y,
            source: -1,
            target: -1,
            team: -1,
            colour: String::new(),
            duration: 0,
            text: String::new(),
            scale: 0,
            name: String::new(),
            value: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub attacker: i32,
    pub victim: u32,
    pub dealt: i32,
    pub attack: bool,
}

impl Sim {
    pub fn push_fx(&mut self, event: FxEvent) -> bool {
        if self.fx.len() >= FX_CAP {
            return false;
        }
        self.fx.push(event);
        true
    }

    pub fn take_fx(&mut self) -> Vec<FxEvent> {
        std::mem::take(&mut self.fx)
    }

    pub(crate) fn log_hit(&mut self, attacker: i32, victim: usize, dealt: i32) {
        if dealt > 0 && self.hits.len() < HIT_CAP {
            self.hits.push(Hit { attacker, victim: victim as u32, dealt, attack: self.spell.attack_hit });
        }
    }

    pub fn take_hits(&mut self) -> Vec<Hit> {
        std::mem::take(&mut self.hits)
    }
}
