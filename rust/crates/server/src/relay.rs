use crate::http::{json_string, Request, Response};
use crate::log::Ev;
use crate::room::Room;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const MAX_SESSION_CHARS: usize = 64;
pub const MAX_MESSAGE_CHARS: usize = 256;
pub const CLIENT_ERROR_BURST: u32 = 20;
pub const CLIENT_ERROR_REFILL_TICKS: u64 = 20;
pub const CLIENT_ERROR_TABLE_SESSIONS: usize = 256;
pub const CLIENT_ERROR_LOG_ENV: &str = "OSCRAFT_CLIENT_ERROR_LOG";
pub const CLIENT_ERROR_LOG_DEFAULT: &str = "client-errors.log";
pub const CLIENT_ERROR_LOG_MAX_BYTES: u64 = 1 << 20;

struct Bucket {
    session: String,
    tokens: u32,
    refill_tick: u64,
    dropped: u32,
    last_seen: u64,
}

struct Buckets {
    entries: Vec<Bucket>,
    clock: u64,
    loaded: bool,
    restored: Vec<(String, u32)>,
}

static BUCKETS: Mutex<Buckets> = Mutex::new(Buckets {
    entries: Vec::new(),
    clock: 0,
    loaded: false,
    restored: Vec::new(),
});

pub fn serve_client_error(room: &Room, req: &Request) -> Response {
    let session = bounded(&req.param("session").unwrap_or_default(), MAX_SESSION_CHARS);
    let now = room.tick_now();
    match admit(&session, now) {
        Ok((dropped, tokens)) => {
            let session_json = json_string(&session);
            if dropped > 0 {
                crate::emit!(Ev::ClientErrorDropped, session = session, count = dropped);
                append_log(&format!("client-error-dropped session={} count={}", session_json, dropped));
            }
            let message = bounded(&req.param("message").unwrap_or_default(), MAX_MESSAGE_CHARS);
            let message_json = json_string(&message);
            crate::emit!(Ev::ClientError, session = session, tick = now, tokens = tokens, message = message);
            append_log(&format!(
                "client-error session={} tick={} tokens={} message={}",
                session_json, now, tokens, message_json
            ));
            Response::json(202, "{\"logged\":true}".to_string())
        }
        Err(()) => Response::json(429, "{\"logged\":false,\"reason\":\"rate-limited\"}".to_string()),
    }
}

fn log_path() -> PathBuf {
    std::env::var_os(CLIENT_ERROR_LOG_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(CLIENT_ERROR_LOG_DEFAULT))
}

fn rotated_path(path: &Path) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(".1");
    PathBuf::from(name)
}

fn append_log(line: &str) {
    let path = log_path();
    if fs::metadata(&path).is_ok_and(|m| m.len() >= CLIENT_ERROR_LOG_MAX_BYTES) {
        if let Err(e) = fs::rename(&path, rotated_path(&path)) {
            crate::emit_err!(Ev::ClientErrorLogFailed, op = "rotate", path = path.display(), error = e);
        }
    }
    let written = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut file| file.write_all(format!("{line}\n").as_bytes()));
    if let Err(e) = written {
        crate::emit_err!(Ev::ClientErrorLogFailed, op = "write", path = path.display(), error = e);
    }
}

fn load_restored() -> Vec<(String, u32)> {
    let path = log_path();
    let mut found: Vec<(String, u32)> = Vec::new();
    for file in [rotated_path(&path), path.clone()] {
        let Ok(text) = fs::read_to_string(&file) else {
            continue;
        };
        for (key, tokens) in text.lines().filter_map(parse_state) {
            match found.iter().position(|(k, _)| *k == key) {
                Some(at) => found[at].1 = tokens,
                None => found.push((key, tokens)),
            }
        }
    }
    found
}

fn parse_state(line: &str) -> Option<(String, u32)> {
    let rest = line.strip_prefix("client-error session=")?;
    let bytes = rest.as_bytes();
    if bytes.first() != Some(&b'"') {
        return None;
    }
    let mut end = 1;
    while end < bytes.len() && bytes[end] != b'"' {
        end += if bytes[end] == b'\\' { 2 } else { 1 };
    }
    if end >= bytes.len() {
        return None;
    }
    let after = rest[end + 1..].split_once(" tokens=")?.1;
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    Some((rest[..=end].to_string(), digits.parse::<u32>().ok()?))
}

fn admit(session: &str, now: u64) -> Result<(u32, u32), ()> {
    let mut table = BUCKETS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if !table.loaded {
        table.loaded = true;
        table.restored = load_restored();
    }
    table.clock += 1;
    let stamp = table.clock;
    let index = match table.entries.iter().position(|b| b.session == session) {
        Some(index) => index,
        None => {
            if table.entries.len() == CLIENT_ERROR_TABLE_SESSIONS {
                let oldest = table
                    .entries
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, b)| b.last_seen)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                table.entries.swap_remove(oldest);
            }
            let key = json_string(session);
            let tokens = match table.restored.iter().position(|(k, _)| *k == key) {
                Some(at) => table.restored.swap_remove(at).1.min(CLIENT_ERROR_BURST),
                None => CLIENT_ERROR_BURST,
            };
            table.entries.push(Bucket {
                session: session.to_string(),
                tokens,
                refill_tick: now,
                dropped: 0,
                last_seen: stamp,
            });
            table.entries.len() - 1
        }
    };
    let bucket = &mut table.entries[index];
    bucket.last_seen = stamp;
    let refills = (now.saturating_sub(bucket.refill_tick) / CLIENT_ERROR_REFILL_TICKS)
        .min(CLIENT_ERROR_BURST as u64) as u32;
    bucket.tokens = (bucket.tokens + refills).min(CLIENT_ERROR_BURST);
    bucket.refill_tick = if bucket.tokens == CLIENT_ERROR_BURST {
        now
    } else {
        bucket.refill_tick + refills as u64 * CLIENT_ERROR_REFILL_TICKS
    };
    if bucket.tokens == 0 {
        bucket.dropped += 1;
        return Err(());
    }
    bucket.tokens -= 1;
    Ok((std::mem::take(&mut bucket.dropped), bucket.tokens))
}

fn bounded(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}
