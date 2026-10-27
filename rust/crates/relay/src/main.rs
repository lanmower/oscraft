use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};
use tungstenite::{accept, Error as WsError, Message};

type Conn = u64;

#[derive(Default)]
struct State {
    events: HashMap<String, Value>,
    by_key: HashMap<String, String>,
    subs: Vec<Sub>,
    clients: HashMap<Conn, Sender<String>>,
}

struct Sub {
    conn: Conn,
    id: String,
    filters: Vec<Value>,
}

fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn send(state: &State, conn: Conn, msg: &Value) {
    if let Some(tx) = state.clients.get(&conn) {
        let _ = tx.send(msg.to_string());
    }
}

fn tag_value(ev: &Value, name: &str) -> Option<String> {
    ev.get("tags")?
        .as_array()?
        .iter()
        .find(|t| t.get(0).and_then(Value::as_str) == Some(name))
        .and_then(|t| t.get(1))
        .map(text)
}

fn has_tag_in(ev: &Value, name: &str, wanted: &Value) -> bool {
    let wanted = match wanted.as_array() {
        Some(w) => w,
        None => return false,
    };
    ev.get("tags")
        .and_then(Value::as_array)
        .map_or(false, |tags| {
            tags.iter().any(|t| {
                t.get(0).and_then(Value::as_str) == Some(name)
                    && t.get(1).map_or(false, |v| wanted.contains(v))
            })
        })
}

fn matches(ev: &Value, filter: &Value) -> bool {
    let filter = match filter.as_object() {
        Some(f) => f,
        None => return false,
    };
    let id = ev.get("id").and_then(Value::as_str).unwrap_or("");
    let pubkey = ev.get("pubkey").and_then(Value::as_str).unwrap_or("");
    let kind = ev.get("kind").and_then(Value::as_u64);
    let created = ev.get("created_at").and_then(Value::as_i64).unwrap_or(0);
    let has_prefix = |field: &str, prefixes: &Value| {
        prefixes.as_array().map_or(false, |ps| {
            ps.iter().any(|p| p.as_str().map_or(false, |p| field.starts_with(p)))
        })
    };
    if let Some(ids) = filter.get("ids") {
        if !has_prefix(id, ids) {
            return false;
        }
    }
    if let Some(authors) = filter.get("authors") {
        if !has_prefix(pubkey, authors) {
            return false;
        }
    }
    if let Some(kinds) = filter.get("kinds").and_then(Value::as_array) {
        if !kinds.iter().any(|k| kind.is_some() && k.as_u64() == kind) {
            return false;
        }
    }
    if let Some(since) = filter.get("since").and_then(Value::as_i64) {
        if created < since {
            return false;
        }
    }
    if let Some(until) = filter.get("until").and_then(Value::as_i64) {
        if created > until {
            return false;
        }
    }
    for (key, wanted) in filter {
        if let Some(name) = key.strip_prefix('#') {
            if !has_tag_in(ev, name, wanted) {
                return false;
            }
        }
    }
    true
}

fn replace_key(ev: &Value) -> Option<String> {
    let kind = ev.get("kind")?.as_u64()?;
    let pubkey = ev.get("pubkey").and_then(Value::as_str).unwrap_or("");
    if kind == 0 || kind == 3 || (10000..20000).contains(&kind) {
        return Some(format!("{pubkey}:{kind}"));
    }
    if (30000..40000).contains(&kind) {
        let d = tag_value(ev, "d").unwrap_or_default();
        return Some(format!("{pubkey}:{kind}:{d}"));
    }
    None
}

fn deliver(state: &State, ev: &Value) {
    let mut delivered = 0;
    for sub in &state.subs {
        if sub.filters.iter().any(|f| matches(ev, f)) {
            send(state, sub.conn, &json!(["EVENT", sub.id, ev]));
            delivered += 1;
        }
    }
    if ev.get("kind").and_then(Value::as_u64) == Some(30078) {
        if let Some(to) = tag_value(ev, "p") {
            let short: String = to.chars().take(8).collect();
            println!("ev=relay_deliver to={short} delivered={delivered} subs={}", state.subs.len());
        }
    }
}

fn on_event(state: &mut State, conn: Conn, ev: &Value) {
    let id = ev.get("id").and_then(Value::as_str).map(str::to_string);
    let kind = ev.get("kind").and_then(Value::as_u64);
    let (id, kind) = match (id, kind) {
        (Some(id), Some(kind)) => (id, kind),
        (id, _) => {
            let reply = json!(["OK", id.unwrap_or_default(), false, "invalid: malformed event"]);
            send(state, conn, &reply);
            return;
        }
    };
    if state.events.contains_key(&id) {
        send(state, conn, &json!(["OK", id, true, "duplicate: already have this event"]));
        return;
    }
    if (20000..30000).contains(&kind) {
        send(state, conn, &json!(["OK", id, true, ""]));
        deliver(state, ev);
        return;
    }
    if let Some(key) = replace_key(ev) {
        if let Some(old_id) = state.by_key.get(&key).cloned() {
            let old_created = state
                .events
                .get(&old_id)
                .and_then(|o| o.get("created_at"))
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let created = ev.get("created_at").and_then(Value::as_i64).unwrap_or(0);
            if old_created > created || (old_created == created && old_id.as_str() < id.as_str()) {
                send(state, conn, &json!(["OK", id, true, "duplicate: superseded by a newer event"]));
                return;
            }
            state.events.remove(&old_id);
        }
        state.by_key.insert(key, id.clone());
    }
    state.events.insert(id.clone(), ev.clone());
    send(state, conn, &json!(["OK", id, true, ""]));
    deliver(state, ev);
}

fn on_req(state: &mut State, conn: Conn, sub_id: String, filters: Vec<Value>) {
    state.subs.push(Sub { conn, id: sub_id.clone(), filters: filters.clone() });
    for f in &filters {
        let mut hits: Vec<&Value> = state.events.values().filter(|ev| matches(ev, f)).collect();
        hits.sort_by_key(|ev| std::cmp::Reverse(ev.get("created_at").and_then(Value::as_i64).unwrap_or(0)));
        if let Some(limit) = f.get("limit").and_then(Value::as_u64) {
            hits.truncate(limit as usize);
        }
        for ev in hits {
            send(state, conn, &json!(["EVENT", sub_id, ev]));
        }
    }
    send(state, conn, &json!(["EOSE", sub_id]));
}

fn on_message(state: &mut State, conn: Conn, raw: &str) {
    let msg: Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return,
    };
    let parts = match msg.as_array() {
        Some(p) => p,
        None => return,
    };
    match parts.first().and_then(Value::as_str) {
        Some("REQ") => {
            let sub_id = parts.get(1).map(text).unwrap_or_default();
            println!("ev=relay_req sub={sub_id} filters={}", parts.len().saturating_sub(2));
            let filters: Vec<Value> = parts.iter().skip(2).cloned().collect();
            on_req(state, conn, sub_id, filters);
        }
        Some("EVENT") => {
            let missing = Value::Null;
            let ev = parts.get(1).unwrap_or(&missing);
            if let Some(kind) = ev.get("kind") {
                println!("ev=relay_event kind={}", text(kind));
            }
            on_event(state, conn, ev);
        }
        Some("CLOSE") => {
            let sub_id = parts.get(1).map(text).unwrap_or_default();
            state.subs.retain(|sub| !(sub.conn == conn && sub.id == sub_id));
        }
        _ => {}
    }
}

fn handle(stream: TcpStream, conn: Conn, state: Arc<Mutex<State>>) {
    let mut ws = match accept(stream) {
        Ok(ws) => ws,
        Err(_) => return,
    };
    if ws.get_ref().set_read_timeout(Some(Duration::from_millis(20))).is_err() {
        return;
    }
    let (tx, rx) = channel::<String>();
    lock(&state).clients.insert(conn, tx);
    println!("ev=relay_connect");
    let mut open = true;
    while open {
        let mut failed = false;
        while let Ok(out) = rx.try_recv() {
            if ws.send(Message::Text(out)).is_err() {
                failed = true;
                break;
            }
        }
        if failed {
            break;
        }
        match ws.read() {
            Ok(Message::Text(raw)) => on_message(&mut lock(&state), conn, &raw),
            Ok(Message::Close(_)) => open = false,
            Ok(_) => {}
            Err(WsError::Io(e)) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(_) => open = false,
        }
    }
    let mut s = lock(&state);
    s.clients.remove(&conn);
    s.subs.retain(|sub| sub.conn != conn);
}

fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("RELAY_PORT").ok())
        .and_then(|p| p.parse().ok())
        .unwrap_or(7447);
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("bind the relay port");
    println!("ev=relay_listening url=ws://127.0.0.1:{port}");
    let state = Arc::new(Mutex::new(State::default()));
    let mut next: Conn = 0;
    for stream in listener.incoming().flatten() {
        next += 1;
        let (conn, state) = (next, Arc::clone(&state));
        thread::spawn(move || handle(stream, conn, state));
    }
}
