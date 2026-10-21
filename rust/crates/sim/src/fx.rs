//! Display effects: the events mod code queues with game.effect(kind, x, y, params). The engine
//! stores them and the host drains them once per frame; it decides nothing about them. Transient:
//! neither hashed nor saved, so a queued effect never changes a state hash or a replay.
//! Hits are logged the same way (the on_hit hook reads them), and feed the damage effects a mod draws.

use crate::arrays::Sim;

/// Most effects the queue holds between drains. A run that never drains it keeps the first ones.
pub const FX_CAP: usize = 4096;
/// Most hits the log holds between drains.
const HIT_CAP: usize = 1024;

/// One display event. `kind` is a name from the data (the effects section of the page's UI data).
/// x and y are fixed-point world coordinates; an event anchored to a unit names it in `source` or
/// `target` and the page reads its position. The other fields are the small parameter table a
/// mod passes: -1 and 0 and "" mean absent.
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
    /// An event of `kind` at (`x`, `y`) with every parameter absent.
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

/// One hit that dealt damage: the attacker (-1 for none), the victim and the damage taken.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub attacker: i32,
    pub victim: u32,
    pub dealt: i32,
}

impl Sim {
    /// Queues a display event. False when the queue is full (the event is dropped).
    pub fn push_fx(&mut self, event: FxEvent) -> bool {
        if self.fx.len() >= FX_CAP {
            return false;
        }
        self.fx.push(event);
        true
    }

    /// Takes the display events queued since the last call, oldest first.
    pub fn take_fx(&mut self) -> Vec<FxEvent> {
        std::mem::take(&mut self.fx)
    }

    /// Logs a hit that dealt `dealt` damage to unit `victim`, for the on_hit hook.
    pub(crate) fn log_hit(&mut self, attacker: i32, victim: usize, dealt: i32) {
        if dealt > 0 && self.hits.len() < HIT_CAP {
            self.hits.push(Hit { attacker, victim: victim as u32, dealt });
        }
    }

    /// Takes the hits that dealt damage since the last call, oldest first.
    pub fn take_hits(&mut self) -> Vec<Hit> {
        std::mem::take(&mut self.hits)
    }
}
