//! Hosts the active Lua mod beside the rooms. A ModHost holds Rc and Lua state, so it is
//! built and driven on one thread; HTTP reads only the atomics in ModLink.

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, Weak};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use mod_host::{Cmd, ModHost};
use sim::{EffectEvent, Payout};
use crate::log::Ev;
use crate::room::Room;
use crate::supervise::{supervise, Outcome};

// Commands the room has not applied yet are kept up to this many; the excess is dropped and logged.
const MOD_CMD_QUEUE_CAP: usize = 4096;

#[derive(Default)]
pub struct ModLink {
    version: AtomicU64,
    tick: AtomicU64,
    cmds: Mutex<Vec<Cmd>>,
    // Payable deaths of the room's sim, waiting for the mod thread to feed them to on_died.
    payouts: Mutex<Vec<Payout>>,
    // Mod-owned effects the room's sim applied, waiting for the mod thread to feed them to on_effect.
    effects: Mutex<Vec<EffectEvent>>,
    test_panic_at: AtomicU64,
    image: Mutex<Option<Vec<u8>>>,
    gold_by_mod: AtomicBool,
}

impl ModLink {
    pub fn arm_test_panic(&self, tick: u64) {
        self.test_panic_at.store(tick + 1, Ordering::Relaxed);
    }

    fn take_test_panic(&self, tick: u64) -> bool {
        if self.test_panic_at.load(Ordering::Relaxed) != tick + 1 {
            return false;
        }
        self.test_panic_at.store(0, Ordering::Relaxed);
        true
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Whether the loaded mod owns kill gold (`GOLD_BY_MOD = true`). Set once the mod is live.
    pub fn gold_by_mod(&self) -> bool {
        self.gold_by_mod.load(Ordering::Acquire)
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

    /// Queues the room's payable deaths for the mod's on_died. Taken in order, at most the queue cap.
    pub fn push_payouts(&self, deaths: Vec<Payout>) {
        let mut queue = self.payouts.lock().unwrap_or_else(|p| p.into_inner());
        let room = MOD_CMD_QUEUE_CAP.saturating_sub(queue.len());
        if deaths.len() > room {
            crate::emit!(Ev::ModCmdsDropped, count = deaths.len() - room);
        }
        queue.extend(deaths.into_iter().take(room));
    }

    fn take_payouts(&self) -> Vec<Payout> {
        std::mem::take(&mut *self.payouts.lock().unwrap_or_else(|p| p.into_inner()))
    }

    /// Queues the room's applied mod-owned effects for on_effect, in order, at most the queue cap.
    pub fn push_effects(&self, events: Vec<EffectEvent>) {
        let mut queue = self.effects.lock().unwrap_or_else(|p| p.into_inner());
        let room = MOD_CMD_QUEUE_CAP.saturating_sub(queue.len());
        if events.len() > room {
            crate::emit!(Ev::ModCmdsDropped, count = events.len() - room);
        }
        queue.extend(events.into_iter().take(room));
    }

    fn take_effects(&self) -> Vec<EffectEvent> {
        std::mem::take(&mut *self.effects.lock().unwrap_or_else(|p| p.into_inner()))
    }

    pub fn stage_image(&self, image: Vec<u8>) {
        *self.image.lock().unwrap_or_else(|p| p.into_inner()) = Some(image);
    }

    fn take_image(&self) -> Option<Vec<u8>> {
        self.image.lock().unwrap_or_else(|p| p.into_inner()).take()
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
            link.gold_by_mod.store(host.gold_by_mod(), Ordering::Release);
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
        if link.take_test_panic(ticks) {
            panic!("test-only panic armed by --test-panic-mod-tick fired at tick {}", ticks);
        }
        if let Some(image) = link.take_image() {
            match host.replace_image(&image) {
                Ok(()) => crate::emit!(Ev::ModImageReloaded, bytes = image.len()),
                Err(e) => crate::emit!(Ev::ModImageReloadFailed, error = format!("{e:?}")),
            }
        }
        match host.reload() {
            Ok(true) => crate::emit!(Ev::ModReloadStaged),
            Ok(false) => {}
            Err(e) => crate::emit!(Ev::ModReloadFailed, error = e),
        }
        // The room's kills reach on_died before this step, so the gold they queue is applied at the
        // room's next boundary with room ids.
        for payout in link.take_payouts() {
            match host.feed_payout(&payout) {
                Ok(cmds) => link.push_commands(cmds),
                Err(e) => crate::emit!(Ev::ModError, tick = ticks, error = e),
            }
        }
        let effects = link.take_effects();
        if !effects.is_empty() {
            match host.feed_effects(&effects) {
                Ok(cmds) => link.push_commands(cmds),
                Err(e) => crate::emit!(Ev::ModError, tick = ticks, error = e),
            }
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

// A data directory is compiled from its .osd files by the CLI's generate path; a file is read as
// an image. The error is the bare reason; the caller names the flag.
pub fn load_data(path: &Path, max_units: usize) -> Result<sim::Sim, String> {
    if !path.is_dir() {
        return sim::load_file(path, max_units).map_err(|e| format!("{e:?}"));
    }
    let image = mod_host::build_image(path)?;
    let mut sim = sim::load(&image, max_units).map_err(|e| format!("{e:?}"))?;
    if let Ok(rules) = fs::read_to_string(path.join("hero_rules.txt")) {
        if !sim.set_hero_rules_text(&rules) {
            return Err("hero_rules.txt has a bad line".to_string());
        }
    }
    Ok(sim)
}

const DATA_POLL: Duration = Duration::from_millis(500);

// The image a --data path names: a directory is compiled from its .osd files, a file is read as is.
pub fn load_image(path: &Path) -> Result<Vec<u8>, String> {
    if path.is_dir() {
        return mod_host::build_image(path);
    }
    fs::read(path).map_err(|e| e.to_string())
}

// Polls the .osd files of a data directory for the mod host. A change is rebuilt and staged on the
// link, and the mod thread swaps it in between ticks. A failed rebuild is logged and the image in
// service stays. The thread ends when the link is dropped.
pub fn watch_image(dir: PathBuf, link: Weak<ModLink>) {
    thread::spawn(move || {
        let mut seen = star_files(&dir);
        loop {
            thread::sleep(DATA_POLL);
            let Some(live) = link.upgrade() else { return };
            let now = star_files(&dir);
            if now == seen {
                continue;
            }
            seen = now;
            match load_image(&dir) {
                Ok(image) => live.stage_image(image),
                Err(error) => crate::emit!(Ev::ModImageReloadFailed, error = error),
            }
        }
    });
}

// Polls the .osd files of a data directory. A change is rebuilt and validated off the tick path and
// staged on the room, which swaps it in at its next tick. A failed rebuild is logged and the image in
// service stays. The thread ends when the room is dropped.
pub fn watch_data(room: Weak<Room>, dir: PathBuf, max_units: usize) {
    thread::spawn(move || {
        let mut seen = star_files(&dir);
        loop {
            thread::sleep(DATA_POLL);
            let Some(live) = room.upgrade() else { return };
            let now = star_files(&dir);
            if now == seen {
                continue;
            }
            seen = now;
            let staged = load_data(&dir, max_units).and_then(|sim| live.stage_data(sim));
            if let Err(error) = staged {
                crate::emit!(Ev::DataReloadFailed, room = live.name, error = error);
            }
        }
    });
}

type StarFiles = Vec<(PathBuf, Option<SystemTime>, u64)>;

fn star_files(dir: &Path) -> StarFiles {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut files: StarFiles = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "osd"))
        .filter_map(|path| {
            let meta = fs::metadata(&path).ok()?;
            Some((path, meta.modified().ok(), meta.len()))
        })
        .collect();
    files.sort();
    files
}
