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
use sim::Payout;
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
    // Deaths that did not fit the payout queue cap; they move into the queue as it drains, in order.
    payout_carry: Mutex<Vec<Payout>>,
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

    /// Queues the room's payable deaths for the mod's on_died, in order. The queue holds at most the cap;
    /// the rest waits in the carry and moves into the queue as the mod thread takes from it, so no payout
    /// is lost. The event's count is the number of deaths still waiting in the carry.
    pub fn push_payouts(&self, deaths: Vec<Payout>) {
        let mut queue = self.payouts.lock().unwrap_or_else(|p| p.into_inner());
        let mut carry = self.payout_carry.lock().unwrap_or_else(|p| p.into_inner());
        carry.extend(deaths);
        let room = MOD_CMD_QUEUE_CAP.saturating_sub(queue.len()).min(carry.len());
        queue.extend(carry.drain(..room));
        if !carry.is_empty() {
            crate::emit!(Ev::ModCmdsDropped, count = carry.len());
        }
    }

    // Takes the whole queue and refills it from the carry, up to the cap.
    fn take_payouts(&self) -> Vec<Payout> {
        let mut queue = self.payouts.lock().unwrap_or_else(|p| p.into_inner());
        let mut carry = self.payout_carry.lock().unwrap_or_else(|p| p.into_inner());
        let taken = std::mem::take(&mut *queue);
        let n = MOD_CMD_QUEUE_CAP.min(carry.len());
        queue.extend(carry.drain(..n));
        taken
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
    // A missing hero_rules.txt keeps the defaults; an unreadable or bad one is an error, so a reload
    // keeps the image in service (the last-known-good rules) and the caller logs the reason.
    match fs::read_to_string(path.join("hero_rules.txt")) {
        Ok(rules) => {
            if !sim.set_hero_rules_text(&rules) {
                return Err("hero_rules.txt has a bad line".to_string());
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("hero_rules.txt cannot be read: {e}")),
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

// Polls the .osd files and hero_rules.txt of a data directory. A change is rebuilt and validated off the tick path and
// staged on the room, which swaps it in at its next tick. A failed rebuild is logged and the image in
// service stays. The thread ends when the room is dropped.
pub fn watch_data(room: Weak<Room>, dir: PathBuf, max_units: usize) {
    thread::spawn(move || {
        let mut seen = data_files(&dir);
        loop {
            thread::sleep(DATA_POLL);
            let Some(live) = room.upgrade() else { return };
            let now = data_files(&dir);
            if now == seen {
                continue;
            }
            seen = now;
            let staged = load_data(&dir, max_units).and_then(|sim| live.stage_data(sim, &dir.display().to_string()));
            if let Err(error) = staged {
                crate::emit!(Ev::DataReloadFailed, room = live.name, error = error);
            }
        }
    });
}

type StarFiles = Vec<(PathBuf, Option<SystemTime>, u64)>;

// The files whose change reloads a data directory: its .osd files and hero_rules.txt.
fn data_files(dir: &Path) -> StarFiles {
    let mut files = star_files(dir);
    let rules = dir.join("hero_rules.txt");
    if let Ok(meta) = fs::metadata(&rules) {
        files.push((rules, meta.modified().ok(), meta.len()));
        files.sort();
    }
    files
}

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

#[cfg(all(test, feature = "server-tests"))]
mod tests {
    use super::*;

    fn payout(victim: u32) -> Payout {
        Payout { tick: 0, victim, victim_player: 0, killer: None, killer_player: None, killer_hero: false, bounty: 1 }
    }

    // A fresh copy of the duel-routes scenario's .osd files in its own temp dir.
    fn copy_scenario(name: &str) -> PathBuf {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("duel-routes");
        let dst = std::env::temp_dir().join(format!("mods-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dst);
        fs::create_dir_all(&dst).expect("temp dir");
        for entry in fs::read_dir(&src).expect("scenario dir") {
            let path = entry.expect("entry").path();
            if path.extension().is_some_and(|ext| ext == "osd") {
                fs::copy(&path, dst.join(path.file_name().expect("name"))).expect("copy osd");
            }
        }
        dst
    }

    // Deaths past the queue cap wait in the carry and reach the mod in order; none is dropped.
    #[test]
    fn payouts_past_the_queue_cap_are_carried_and_delivered_in_order() {
        let link = ModLink::default();
        let total = MOD_CMD_QUEUE_CAP as u32 + 10;
        link.push_payouts((0..total).map(payout).collect());
        assert_eq!(link.payouts.lock().expect("queue").len(), MOD_CMD_QUEUE_CAP, "the queue must stay bounded");
        let first = link.take_payouts();
        assert_eq!(first.len(), MOD_CMD_QUEUE_CAP);
        let second = link.take_payouts();
        let victims: Vec<u32> = first.iter().chain(second.iter()).map(|p| p.victim).collect();
        assert_eq!(victims, (0..total).collect::<Vec<u32>>(), "a payout was dropped or reordered");
        assert!(link.take_payouts().is_empty());
    }

    // An unreadable hero_rules.txt and a bad rule line are both refused, so a reload keeps the image in service.
    #[test]
    fn unreadable_or_bad_hero_rules_are_refused_and_a_missing_file_keeps_defaults() {
        let dir = copy_scenario("rules-read");
        fs::create_dir_all(dir.join("hero_rules.txt")).expect("rules as a directory");
        let err = load_data(&dir, 64).err().expect("an unreadable rules file is refused");
        assert!(err.contains("cannot be read"), "wrong error: {err}");
        fs::remove_dir(dir.join("hero_rules.txt")).expect("remove");

        fs::write(dir.join("hero_rules.txt"), "hero no_such_unit 1\n").expect("write bad rules");
        let err = load_data(&dir, 64).err().expect("a bad rule line is refused");
        assert!(err.contains("bad line"), "wrong error: {err}");

        fs::remove_file(dir.join("hero_rules.txt")).expect("remove");
        assert!(load_data(&dir, 64).is_ok(), "a missing rules file keeps the defaults");
        let _ = fs::remove_dir_all(&dir);
    }

    // The watcher's file set includes hero_rules.txt, so creating or editing it counts as a change.
    #[test]
    fn the_data_watch_set_includes_hero_rules_so_an_edit_is_seen() {
        let dir = copy_scenario("rules-watch");
        let before = data_files(&dir);
        assert!(!before.iter().any(|(p, _, _)| p.ends_with("hero_rules.txt")));
        fs::write(dir.join("hero_rules.txt"), "# rules\n").expect("write rules");
        let created = data_files(&dir);
        assert_ne!(before, created, "creating hero_rules.txt was not seen");
        fs::write(dir.join("hero_rules.txt"), "# rules, edited\n").expect("edit rules");
        assert_ne!(created, data_files(&dir), "editing hero_rules.txt was not seen");
        let _ = fs::remove_dir_all(&dir);
    }
}
