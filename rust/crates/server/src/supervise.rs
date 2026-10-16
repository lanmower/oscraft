//! Restarts a thread's work when it panics or returns: each death is one ev= line, the restart
//! waits a bounded backoff, and past MAX_RESTARTS_PER_MINUTE the thread stops and logs
//! supervisor_giveup while the rest of the server keeps serving.

use std::any::Any;
use std::collections::VecDeque;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::thread;
use std::time::{Duration, Instant};

use crate::log::Ev;

pub const BACKOFF_START: Duration = Duration::from_millis(100);
pub const BACKOFF_CAP: Duration = Duration::from_secs(2);
pub const MAX_RESTARTS_PER_MINUTE: usize = 5;
const RESTART_WINDOW: Duration = Duration::from_secs(60);

pub enum Outcome {
    Stop,
    Retry,
}

pub fn supervise(name: &'static str, died: Ev, restarted: Ev, mut attempt: impl FnMut() -> Outcome) {
    let mut recent: VecDeque<Instant> = VecDeque::new();
    let mut backoff = BACKOFF_START;
    loop {
        let reason = match catch_unwind(AssertUnwindSafe(|| attempt())) {
            Ok(Outcome::Stop) => return,
            Ok(Outcome::Retry) => "returned".to_string(),
            Err(payload) => panic_text(payload.as_ref()),
        };
        crate::emit!(died, thread = name, reason = reason);
        let now = Instant::now();
        while recent.front().is_some_and(|t| now.duration_since(*t) > RESTART_WINDOW) {
            recent.pop_front();
        }
        if recent.len() >= MAX_RESTARTS_PER_MINUTE {
            crate::emit!(Ev::SupervisorGiveup, thread = name, restarts = recent.len());
            return;
        }
        recent.push_back(now);
        thread::sleep(backoff);
        backoff = (backoff * 2).min(BACKOFF_CAP);
        crate::emit!(restarted, thread = name, restarts = recent.len());
    }
}

fn panic_text(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-string panic".to_string())
}
