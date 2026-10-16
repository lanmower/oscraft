pub mod budget;
pub mod http;
pub mod lockstep;
pub mod log;
pub mod mods;
pub mod order;
pub mod replay;
pub mod relay;
pub mod room;
pub mod supervise;
pub mod static_files;

use std::collections::hash_map::RandomState;
use std::env;
use std::fs::File;
use std::hash::{BuildHasher, Hasher};
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use http::{json_string, read_request, ReadError, Request, Response};
use order::{parse_order, DEFAULT_ORDER_RATE, MAX_ORDER_BODY};
use budget::spawn_reporter;
use log::Ev;
use mods::ModLink;
use room::{spawn_ticker, DeltaError, Room, RoomConfig, TICKS_PER_SECOND};

const DEFAULT_MAX_ROOMS: usize = 16;
const DEFAULT_MAX_UNITS: usize = 4096;
const DEFAULT_ADDR: &str = "0.0.0.0:8080";

/// Connections served at once. One more is answered 503 and logged as ev=conn_refused.
pub const MAX_CONNECTIONS: usize = 64;

pub struct Options {
    pub addr: String,
    pub rooms: Vec<String>,
    pub grid_w: i32,
    pub grid_h: i32,
    pub max_units: usize,
    pub rate: u32,
    pub max_rooms: usize,
    pub tokens: Vec<(String, String)>,
    pub data: Option<PathBuf>,
    pub tick_budget_us: u64,
    pub ticks: Option<u64>,
    pub mod_path: Option<PathBuf>,
    pub test_panic_tick: Option<u64>,
    pub test_panic_mod_tick: Option<u64>,
    pub site: Option<PathBuf>,
}

pub struct Server {
    pub mods: Arc<ModLink>,
    pub rooms: Vec<Arc<Room>>,
    pub max_rooms: usize,
    pub site: Option<PathBuf>,
}

pub fn run() -> Result<(), String> {
    let opts = parse_args()?;
    let rooms = build_rooms(&opts)?;
    let tick_budget = Duration::from_micros(opts.tick_budget_us);
    let link = Arc::new(ModLink::default());
    if let Some(path) = &opts.mod_path {
        let image_path = opts.data.as_ref().ok_or("--mod needs --data: the mod host runs on the catalog image")?;
        let image = std::fs::read(image_path).map_err(|e| format!("--data {}: {e}", image_path.display()))?;
        if let Some(tick) = opts.test_panic_mod_tick {
            link.arm_test_panic(tick);
        }
        mods::spawn(image, path.clone(), tick_budget, Arc::clone(&link))?;
        crate::emit!(Ev::ModLoaded, path = path.display(), version = link.version());
    }

    rooms[0].attach_mod_feed(Arc::clone(&link));
    for room in &rooms {
        if let Some(tick) = opts.test_panic_tick {
            room.arm_test_panic(tick);
        }
        spawn_ticker(Arc::clone(room), tick_budget);
        spawn_reporter(Arc::clone(room), opts.ticks);
        crate::emit!(Ev::OrderToken, room = room.name, token = room.gate.token());
    }

    let listener = TcpListener::bind(&opts.addr).map_err(|e| format!("bind {}: {e}", opts.addr))?;
    crate::emit!(Ev::Serving, rooms = rooms.len(), addr = &opts.addr, rate = opts.rate);

    let server = Arc::new(Server {
        mods: Arc::clone(&link),
        rooms,
        max_rooms: opts.max_rooms,
        site: opts.site,
    });
    let active = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => match admit(&active) {
                Some(slot) => {
                    let server = Arc::clone(&server);
                    thread::spawn(move || {
                        let _slot = slot;
                        handle(stream, &server)
                    });
                }
                None => refuse(&mut stream),
            },
            Err(e) => crate::emit_err!(Ev::AcceptError, error = e),
        }
    }
    Ok(())
}

struct Slot(Arc<AtomicUsize>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

fn admit(active: &Arc<AtomicUsize>) -> Option<Slot> {
    active
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| (n < MAX_CONNECTIONS).then_some(n + 1))
        .ok()
        .map(|_| Slot(Arc::clone(active)))
}

fn refuse(stream: &mut TcpStream) {
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    crate::emit!(Ev::ConnRefused, limit = MAX_CONNECTIONS);
    let response = Response::error(503, "connection limit reached; retry shortly").with_header("Retry-After", "1");
    let _ = response.write_to(stream);
}

pub fn build_rooms(opts: &Options) -> Result<Vec<Arc<Room>>, String> {
    if opts.rooms.is_empty() {
        return Err("no rooms: pass --room NAME at least once".into());
    }
    if opts.rooms.len() > opts.max_rooms {
        return Err(format!(
            "{} rooms asked for and --max-rooms is {}; raise --max-rooms",
            opts.rooms.len(),
            opts.max_rooms
        ));
    }

    let mut rooms: Vec<Arc<Room>> = Vec::new();
    for name in &opts.rooms {
        let key = name.to_lowercase();
        if rooms.iter().any(|r| r.name == key) {
            return Err(format!("room {name} is given twice"));
        }
        let token = opts
            .tokens
            .iter()
            .find(|(n, _)| *n == key)
            .map(|(_, t)| t.clone())
            .unwrap_or_else(random_token);
        rooms.push(Room::new(RoomConfig {
            name: key,
            grid_w: opts.grid_w,
            grid_h: opts.grid_h,
            max_units: opts.max_units,
            token,
            rate: opts.rate,
            data: opts.data.clone(),
        })?);
    }
    Ok(rooms)
}

pub fn parse_args() -> Result<Options, String> {
    let mut opts = Options {
        addr: DEFAULT_ADDR.to_string(),
        rooms: Vec::new(),
        grid_w: 64,
        grid_h: 64,
        max_units: DEFAULT_MAX_UNITS,
        rate: DEFAULT_ORDER_RATE,
        max_rooms: DEFAULT_MAX_ROOMS,
        tokens: Vec::new(),
        data: None,
        tick_budget_us: 1_000_000 / u64::from(TICKS_PER_SECOND),
        ticks: None,
        mod_path: None,
        test_panic_tick: None,
        test_panic_mod_tick: None,
        site: None,
    };
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--addr" => opts.addr = value,
            "--room" => opts.rooms.push(value),
            "--grid" => {
                let (w, h) = value.split_once(',').ok_or("--grid is W,H")?;
                opts.grid_w = parse_num(w, &flag)?;
                opts.grid_h = parse_num(h, &flag)?;
            }
            "--max-units" => opts.max_units = parse_num(&value, &flag)?,
            "--order-rate" => opts.rate = parse_num(&value, &flag)?,
            "--max-rooms" => opts.max_rooms = parse_num(&value, &flag)?,
            "--data" => opts.data = Some(PathBuf::from(value)),
            "--tick-budget-us" => opts.tick_budget_us = parse_num(&value, &flag)?,
            "--ticks" => opts.ticks = Some(parse_num(&value, &flag)?),
            "--mod" => opts.mod_path = Some(PathBuf::from(value)),
            "--test-panic-tick" => opts.test_panic_tick = Some(parse_num(&value, &flag)?),
            "--test-panic-mod-tick" => opts.test_panic_mod_tick = Some(parse_num(&value, &flag)?),
            "--site" => opts.site = Some(PathBuf::from(value)),
            "--order-token" => {
                let (name, token) = value.split_once('=').ok_or("--order-token is NAME=TOKEN")?;
                opts.tokens.push((name.to_lowercase(), token.to_string()));
            }
            other => return Err(format!("unknown flag {other}")),
        }
    }
    if opts.rate == 0 || opts.grid_w <= 0 || opts.grid_h <= 0 || opts.max_units == 0 || opts.tick_budget_us == 0 {
        return Err("--order-rate, --grid, --max-units and --tick-budget-us must be positive".into());
    }
    Ok(opts)
}

fn parse_num<T: FromStr>(value: &str, flag: &str) -> Result<T, String> {
    value
        .parse::<T>()
        .map_err(|_| format!("{flag} value {value} is not a number"))
}

fn random_token() -> String {
    let mut buf = [0u8; 16];
    let from_urandom = File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut buf));
    if from_urandom.is_err() {
        for (i, chunk) in buf.chunks_mut(8).enumerate() {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_usize(i);
            chunk.copy_from_slice(&hasher.finish().to_le_bytes()[..chunk.len()]);
        }
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn handle(mut stream: TcpStream, server: &Server) {
    let timeout = Some(Duration::from_secs(5));
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);
    let response = match read_request(&stream, MAX_ORDER_BODY) {
        Ok(req) => server.route(&req),
        Err(ReadError::TooLarge) => Response::error(413, "request body is over 4096 bytes"),
        Err(ReadError::Malformed(why)) => Response::error(400, why),
        Err(ReadError::Io(_)) => return,
    };
    let _ = response.write_to(&mut stream);
}

impl Server {
    pub fn route(&self, req: &Request) -> Response {
        match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/") => self.index(),
            ("GET", "/resources") => self.resources(),
            ("GET", "/state") => self.on_room(req, serve_state),
            ("GET", "/delta") => self.on_room(req, serve_delta),
            ("GET", "/catalog") => self.on_room(req, serve_catalog),
            ("GET", "/mod-version") => self.mod_version(),
            ("GET", "/mod-events") => self.mod_events(req),
            ("POST", "/order") => self.on_room(req, serve_order),
            ("POST", "/client-error") => self.on_room(req, relay::serve_client_error),
            ("GET", path) if path.starts_with(static_files::SITE_PREFIX) => match &self.site {
                Some(root) => static_files::serve(root, path),
                None => Response::error(404, "no such endpoint"),
            },
            (_, "/order") => Response::error(405, "orders are POSTed").with_header("Allow", "POST"),
            _ => Response::error(404, "no such endpoint"),
        }
    }

    fn on_room(&self, req: &Request, handler: fn(&Room, &Request) -> Response) -> Response {
        let key = req.param("map").unwrap_or_default().to_lowercase();
        let found = if key.is_empty() {
            self.rooms.first()
        } else {
            self.rooms.iter().find(|r| r.name == key)
        };
        match found {
            Some(room) => handler(room, req),
            None => Response::error(404, "no such map"),
        }
    }

    pub fn index(&self) -> Response {
        let rows: Vec<String> = self
            .rooms
            .iter()
            .map(|r| {
                format!(
                    "{{\"map\":{},\"grid\":[{},{}],\"max_units\":{},\"order\":{},\"orders_per_second\":{}}}",
                    json_string(&r.name),
                    r.grid_w,
                    r.grid_h,
                    r.max_units,
                    json_string(&format!("/order?map={}", r.name)),
                    r.gate.rate()
                )
            })
            .collect();
        Response::json(200, format!("{{\"maps\":[{}]}}", rows.join(",")))
    }

    fn mod_version(&self) -> Response {
        Response::json(200, format!(r#"{{"version":{},"tick":{}}}"#, self.mods.version(), self.mods.tick()))
    }

    fn mod_events(&self, req: &Request) -> Response {
        let since = req.param("since").and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        let version = self.mods.version();
        Response::json(200, format!(r#"{{"version":{version},"since":{since},"changed":{}}}"#, version > since))
    }

    pub fn resources(&self) -> Response {
        let rows: Vec<String> = self
            .rooms
            .iter()
            .map(|r| {
                let s = r.stats();
                format!(
                    "{{\"map\":{},\"tick\":{},\"overruns\":{},\"worst_tick_ms\":{:.3},\"orders_drained\":{}}}",
                    json_string(&r.name),
                    s.tick,
                    s.overruns,
                    s.worst.as_secs_f64() * 1000.0,
                    s.orders_drained
                )
            })
            .collect();
        Response::json(
            200,
            format!(
                "{{\"rooms\":{},\"max_rooms\":{},\"ticks_per_second\":{},\"per_room\":[{}]}}",
                self.rooms.len(),
                self.max_rooms,
                TICKS_PER_SECOND,
                rows.join(",")
            ),
        )
    }
}

pub fn serve_state(room: &Room, req: &Request) -> Response {
    let snap = room.snapshot();
    let unit = match req.param("unit").and_then(|raw| raw.parse::<usize>().ok()) {
        Some(id) => match room.unit_position(id) {
            Some((x, y)) => format!("{{\"id\":{id},\"x\":{x},\"y\":{y}}}"),
            None => "null".to_string(),
        },
        None => "null".to_string(),
    };
    Response::json(
        200,
        format!(
            "{{\"map\":{},\"tick\":{},\"state_hash\":\"{:016x}\",\"max_units\":{},\"units\":{},\"unit\":{}}}",
            json_string(&room.name),
            snap.tick,
            snap.state_hash,
            room.max_units,
            snap.units,
            unit
        ),
    )
}

pub fn serve_delta(room: &Room, req: &Request) -> Response {
    let Some(raw) = req.param("since") else {
        return Response::error(400, "since is required: GET /delta?since=<tick>");
    };
    let Ok(since) = raw.parse::<u32>() else {
        return Response::error(400, "since must be a tick number");
    };
    match room.delta_since(since) {
        Ok(delta) => {
            let units: Vec<String> = delta.units.iter().map(u32::to_string).collect();
            Response::json(
                200,
                format!(
                    "{{\"map\":{},\"since\":{},\"tick\":{},\"changed\":{},\"units\":[{}]}}",
                    json_string(&room.name),
                    since,
                    delta.tick,
                    units.len(),
                    units.join(",")
                ),
            )
        }
        Err(DeltaError::Future { since, tick }) => Response::json(
            400,
            format!(
                "{{\"error\":\"since_in_future\",\"since\":{},\"tick\":{}}}",
                since, tick
            ),
        ),
    }
}

pub fn serve_catalog(room: &Room, _req: &Request) -> Response {
    let body = room.with_catalog(|cat| {
        let units: Vec<String> = cat
            .units
            .iter()
            .map(|u| json_string(cat.name(u.name)))
            .collect();
        let note = if room.loaded {
            "catalog loaded from --data"
        } else {
            "no --data given: the catalog is empty"
        };
        format!(
            "{{\"map\":{},\"loaded\":{},\"note\":{},\"units\":[{}],\"abilities\":{},\"behaviours\":{}}}",
            json_string(&room.name),
            room.loaded,
            json_string(note),
            units.join(","),
            cat.abilities.len(),
            cat.behaviours.len()
        )
    });
    Response::json(200, body)
}

pub fn serve_order(room: &Room, req: &Request) -> Response {
    let rejected = |status: u16, msg: &str| {
        Response::json(
            status,
            format!("{{\"accepted\":false,\"error\":{}}}", json_string(msg)),
        )
    };
    if !room.gate.authorized(req.header("Authorization")) {
        return rejected(
            401,
            "this room accepts orders from a caller holding its token: Authorization: Bearer <token>",
        )
        .with_header("WWW-Authenticate", "Bearer");
    }
    let order = match parse_order(&req.body, room.grid_w, room.grid_h) {
        Ok(o) => o,
        Err(msg) => return rejected(400, &msg),
    };
    if !room.player_has_units(order.player as u8) {
        return Response::json(
            422,
            format!(
                "{{\"accepted\":false,\"code\":\"player_has_no_units\",\"error\":{}}}",
                json_string(&format!(
                    "player {} has no live units on {}: nothing to order",
                    order.player, room.name
                ))
            ),
        );
    }
    let Some(left) = room.gate.take() else {
        return rejected(
            429,
            &format!("this room accepts {} order(s) per second", room.gate.rate()),
        )
        .with_header("Retry-After", "1");
    };
    if let Err(msg) = room.submit(order) {
        return rejected(503, &msg);
    }
    Response::json(
        200,
        format!(
            "{{\"accepted\":true,\"player\":{},\"x\":{:.4},\"y\":{:.4},\"tick\":{},\"orders_left_this_second\":{},\"orders_per_second\":{}}}",
            order.player,
            f64::from(order.x) / 65536.0,
            f64::from(order.y) / 65536.0,
            room.tick_now(),
            left,
            room.gate.rate()
        ),
    )
}