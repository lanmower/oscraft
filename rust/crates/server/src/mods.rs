//! Hosts the active Lua mod beside the rooms. A ModHost holds Rc and Lua state, so it is
//! built and driven on one thread; HTTP reads only the atomics in ModLink.

use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use mod_host::{Cmd, ModHost};
use crate::log::Ev;
use crate::supervise::{supervise, Outcome};

// Commands the room has not applied yet are kept up to this many; the excess is dropped and logged.
const MOD_CMD_QUEUE_CAP: usize = 4096;

#[derive(Default)]
pub struct ModLink {
    version: AtomicU64,
    tick: AtomicU64,
    cmds: Mutex<Vec<Cmd>>,
}

impl ModLink {
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    pub fn tick(&self) -> u64 {
        self.tick.load(Ordering::Acquire)
    }

    pub fn push_commands(&self, cmds: Vec<Cmd>) {
        let mut queue = self.cmds.lock().unwrap_or_else(|p| p.into_inner());
        let room = MOD_CMD_QUEUE_CAP.saturating_sub(queue.len());
        if cmds.len() > room {
            crate::emit!(Ev::ModCmdsDropped, count = cmds.len() - room);
        }
        queue.extend(cmds.into_iter().take(room));
    }

    pub fn take_commands(&self, max: usize) -> Vec<Cmd> {
        let mut queue = self.cmds.lock().unwrap_or_else(|p| p.into_inner());
        let n = max.min(queue.len());
        queue.drain(..n).collect()
    }
}

// Builds the host and loads the mod on a new thread, and returns once the mod is live.
// The version is published before the return, so a caller sees version 1 at once.
pub fn spawn(image: Vec<u8>, path: PathBuf, budget: Duration, link: Arc<ModLink>) -> Result<(), String> {
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
    let mut ready = Some(ready_tx);
    thread::spawn(move || {
        supervise("mod_thread", Ev::ModThreadDied, Ev::ModThreadRestarted, || {
            let mut host = match open(&image, &path) {
                Ok(host) => host,
                Err(e) => {
                    return match ready.take() {
                        Some(tx) => {
                            let _ = tx.send(Err(e));
                            Outcome::Stop
                        }
                        None => {
                            crate::emit!(Ev::ModError, tick = 0, error = e);
                            Outcome::Retry
                        }
                    };
                }
            };
            link.version.store(host.version(), Ordering::Release);
            if let Some(tx) = ready.take() {
                let _ = tx.send(Ok(()));
            }
            drive(&mut host, &link, budget);
            Outcome::Retry
        })
    });
    ready_rx
        .recv()
        .map_err(|_| "mod host thread ended before the mod loaded".to_string())?
}

fn open(image: &[u8], path: &Path) -> Result<ModHost, String> {
    let mut host = ModHost::new(image).map_err(|e| format!("mod host image: {e:?}"))?;
    host.load(path).map_err(|e| format!("--mod {}: {e}", path.display()))?;
    Ok(host)
}

// One tick per budget. A staged reload is validated here, between ticks, and goes live in advance.
fn drive(host: &mut ModHost, link: &ModLink, budget: Duration) {
    let state = host.state().clone();
    let mut next = Instant::now() + budget;
    let mut ticks = 0u64;
    let mut seen = (host.version(), String::new());
    loop {
        let now = Instant::now();
        if next > now {
            thread::sleep(next - now);
        }
        match host.reload() {
            Ok(true) => crate::emit!(Ev::ModReloadStaged),
            Ok(false) => {}
            Err(e) => crate::emit!(Ev::ModReloadFailed, error = e),
        }
        if let Err(e) = host.advance(&state) {
            crate::emit!(Ev::ModError, tick = ticks, error = e);
        }
        link.push_commands(host.take_commands());
        ticks += 1;
        let version = host.version();
        link.version.store(version, Ordering::Release);
        link.tick.store(ticks, Ordering::Release);
        let probe = state
            .get::<Option<String>>("probe")
            .ok()
            .flatten()
            .unwrap_or_else(|| "none".to_string());
        if (version, probe.clone()) != seen {
            crate::emit!(Ev::ModTick, tick = ticks, version = version, probe = probe);
            seen = (version, probe);
        }
        while next <= Instant::now() {
            next += budget;
        }
    }
}
