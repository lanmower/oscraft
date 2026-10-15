use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const ORDER_PLAYERS: u32 = 16;
pub const MAX_ORDER_BODY: usize = 4096;
pub const DEFAULT_ORDER_RATE: u32 = 8;

const FIXED_ONE: f64 = 65536.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Order {
    pub player: u32,
    pub x: i32,
    pub y: i32,
}

pub fn parse_order(body: &[u8], grid_w: i32, grid_h: i32) -> Result<Order, String> {
    let mut player = None;
    let mut x = None;
    let mut y = None;
    for (key, value) in parse_number_object(body)? {
        let slot = match key.as_str() {
            "player" => &mut player,
            "x" => &mut x,
            "y" => &mut y,
            other => {
                return Err(format!(
                    "order body has unknown field {other}: it takes player, x and y"
                ))
            }
        };
        if slot.replace(value).is_some() {
            return Err(format!("order body repeats {key}"));
        }
    }
    let (Some(player), Some(x), Some(y)) = (player, x, y) else {
        return Err("order body must carry player, x and y".into());
    };
    if player.fract() != 0.0 || !(0.0..ORDER_PLAYERS as f64).contains(&player) {
        return Err(format!(
            "player {player} is out of range: a map has players 0..{}",
            ORDER_PLAYERS - 1
        ));
    }
    let fx = world_to_fixed(x).ok_or_else(|| format!("x {x} is not a world coordinate"))?;
    let fy = world_to_fixed(y).ok_or_else(|| format!("y {y} is not a world coordinate"))?;
    let (max_x, max_y) = (i64::from(grid_w) << 16, i64::from(grid_h) << 16);
    if fx < 0 || fy < 0 || i64::from(fx) > max_x || i64::from(fy) > max_y {
        return Err(format!(
            "({x}, {y}) is off this map: x is 0..{grid_w}, y is 0..{grid_h}"
        ));
    }
    Ok(Order {
        player: player as u32,
        x: fx,
        y: fy,
    })
}

fn world_to_fixed(v: f64) -> Option<i32> {
    if !v.is_finite() || !(-32768.0..=32767.0).contains(&v) {
        return None;
    }
    Some((v * FIXED_ONE).round() as i32)
}

fn parse_number_object(body: &[u8]) -> Result<Vec<(String, f64)>, String> {
    let text = std::str::from_utf8(body).map_err(|_| "order body is not UTF-8".to_string())?;
    let mut rest = text
        .trim_start()
        .strip_prefix('{')
        .ok_or("order body must be a JSON object")?
        .trim_start();
    let mut fields = Vec::new();
    if let Some(after) = rest.strip_prefix('}') {
        rest = after;
    } else {
        loop {
            let (key, after_key) = take_key(rest)?;
            let after_colon = after_key
                .trim_start()
                .strip_prefix(':')
                .ok_or("order body needs ':' after each key")?;
            let (value, after_value) = take_number(after_colon.trim_start())?;
            fields.push((key, value));
            let after_value = after_value.trim_start();
            if let Some(next) = after_value.strip_prefix(',') {
                rest = next.trim_start();
            } else if let Some(next) = after_value.strip_prefix('}') {
                rest = next;
                break;
            } else {
                return Err("order body separates fields with ',' and closes with '}'".into());
            }
        }
    }
    if rest.trim().is_empty() {
        Ok(fields)
    } else {
        Err("order body has bytes after the object".into())
    }
}

fn take_key(s: &str) -> Result<(String, &str), String> {
    let body = s
        .strip_prefix('"')
        .ok_or("order body keys must be quoted strings")?;
    let end = body.find('"').ok_or("order body has an unterminated key")?;
    let key = &body[..end];
    if key.contains('\\') {
        return Err("order body keys may not hold escapes".into());
    }
    Ok((key.to_string(), &body[end + 1..]))
}

fn take_number(s: &str) -> Result<(f64, &str), String> {
    let len = s
        .bytes()
        .take_while(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.' | b'e' | b'E'))
        .count();
    let value = s[..len]
        .parse::<f64>()
        .map_err(|_| "order body values must be numbers".to_string())?;
    Ok((value, &s[len..]))
}

pub struct OrderGate {
    token: String,
    rate: u32,
    window: Mutex<(Instant, u32)>,
}

impl OrderGate {
    pub fn new(token: String, rate: u32) -> Self {
        Self {
            token,
            rate,
            window: Mutex::new((Instant::now(), 0)),
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn authorized(&self, header: Option<&str>) -> bool {
        let Some(value) = header else {
            return false;
        };
        let bytes = value.as_bytes();
        if bytes.len() < 7 || !bytes[..7].eq_ignore_ascii_case(b"bearer ") {
            return false;
        }
        let given = std::str::from_utf8(&bytes[7..]).unwrap_or("").trim();
        constant_time_eq(given.as_bytes(), self.token.as_bytes())
    }

    pub fn take(&self) -> Option<u32> {
        let mut window = self.window.lock().unwrap_or_else(|p| p.into_inner());
        let now = Instant::now();
        if now.duration_since(window.0) >= Duration::from_secs(1) {
            *window = (now, 0);
        }
        if window.1 >= self.rate {
            return None;
        }
        window.1 += 1;
        Some(self.rate - window.1)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
