pub mod http;
pub mod order;
pub mod room;

use std::collections::hash_map::RandomState;
use std::env;
use std::fs::File;
use std::hash::{BuildHasher, Hasher};
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use http::{json_string, read_request, ReadError, Request, Response};
use order::{parse_order, DEFAULT_ORDER_RATE, MAX_ORDER_BODY};
use room::{spawn_ticker, DeltaError, Room, RoomConfig, TICKS_PER_SECOND};

const DEFAULT_MAX_ROOMS: usize = 16;
const DEFAULT_MAX_UNITS: usize = 4096;
const DEFAULT_ADDR: &str = "0.0.0.0:8080";

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
}

pub struct Server {
    pub rooms: Vec<Arc<Room>>,
    pub max_rooms: usize,
}

pub fn run() -> Result<(), String> {
    let opts = parse_args()?;
    let rooms = build_rooms(&opts)?;

    for room in &rooms {
        spawn_ticker(Arc::clone(room));
        println!("order-token {} {}", room.name, room.gate.token());
    }

    let listener = TcpListener::bind(&opts.addr).map_err(|e| format!("bind {}: {e}", opts.addr))?;
    println!(
        "serving {} room(s) on {} (order: POST /order?map=<name>, {} per second per room)",
        rooms.len(),
        opts.addr,
        opts.rate
    );

    let server = Arc::new(Server {
        rooms,
        max_rooms: opts.max_rooms,
    });
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let server = Arc::clone(&server);
                thread::spawn(move || handle(stream, &server));
            }
            Err(e) => eprintln!("accept: {e}"),
        }
    }
    Ok(())
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
            "--order-token" => {
                let (name, token) = value.split_once('=').ok_or("--order-token is NAME=TOKEN")?;
                opts.tokens.push((name.to_lowercase(), token.to_string()));
            }
            other => return Err(format!("unknown flag {other}")),
        }
    }
    if opts.rate == 0 || opts.grid_w <= 0 || opts.grid_h <= 0 || opts.max_units == 0 {
        return Err("--order-rate, --grid and --max-units must be positive".into());
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
            ("POST", "/order") => self.on_room(req, serve_order),
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

pub fn serve_state(room: &Room, _req: &Request) -> Response {
    let snap = room.snapshot();
    Response::json(
        200,
        format!(
            "{{\"map\":{},\"tick\":{},\"state_hash\":\"{:016x}\",\"max_units\":{},\"units\":{}}}",
            json_string(&room.name),
            snap.tick,
            snap.state_hash,
            room.max_units,
            snap.units
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
