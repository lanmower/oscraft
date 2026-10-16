use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;

pub const MAX_HEADER_BYTES: usize = 8192;
const MAX_HEADERS: usize = 64;

pub struct Request {
    pub method: String,
    pub path: String,
    pub query: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub enum ReadError {
    Malformed(&'static str),
    TooLarge,
    Io(io::Error),
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn param(&self, key: &str) -> Option<String> {
        self.query.split('&').find_map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (k == key).then(|| percent_decode(v))
        })
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = (bytes[i] == b'%' && i + 2 < bytes.len())
            .then(|| {
                std::str::from_utf8(&bytes[i + 1..i + 3])
                    .ok()
                    .and_then(|h| u8::from_str_radix(h, 16).ok())
            })
            .flatten();
        match (bytes[i], hex) {
            (_, Some(v)) => {
                out.push(v);
                i += 3;
            }
            (b'+', None) => {
                out.push(b' ');
                i += 1;
            }
            (b, None) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn read_request(stream: &TcpStream, max_body: usize) -> Result<Request, ReadError> {
    let clone = stream.try_clone().map_err(ReadError::Io)?;
    let mut reader = BufReader::new(clone.take((MAX_HEADER_BYTES + max_body) as u64));
    let mut line = String::new();
    if reader.read_line(&mut line).map_err(ReadError::Io)? == 0 {
        return Err(ReadError::Malformed("empty request"));
    }
    let mut parts = line.trim_end().split(' ');
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts
        .next()
        .filter(|t| !t.is_empty())
        .ok_or(ReadError::Malformed("request line has no target"))?;
    let version = parts.next().unwrap_or_default();
    if !version.starts_with("HTTP/1.") {
        return Err(ReadError::Malformed("only HTTP/1.x is served"));
    }
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target.to_string(), String::new()),
    };

    let mut headers: Vec<(String, String)> = Vec::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).map_err(ReadError::Io)? == 0 {
            return Err(ReadError::Malformed("request ends inside its headers"));
        }
        let text = line.trim_end_matches(['\r', '\n']);
        if text.is_empty() {
            break;
        }
        if headers.len() == MAX_HEADERS {
            return Err(ReadError::Malformed("too many headers"));
        }
        let (name, value) = text
            .split_once(':')
            .ok_or(ReadError::Malformed("a header line has no colon"))?;
        headers.push((name.trim().to_string(), value.trim().to_string()));
    }

    let mut req = Request {
        method,
        path,
        query,
        headers,
        body: Vec::new(),
    };
    if req.header("transfer-encoding").is_some() {
        return Err(ReadError::Malformed("chunked bodies are not served; send Content-Length"));
    }
    let length = match req.header("content-length") {
        None => 0,
        Some(v) => v
            .parse::<usize>()
            .map_err(|_| ReadError::Malformed("Content-Length is not a number"))?,
    };
    if length > max_body {
        return Err(ReadError::TooLarge);
    }
    req.body = vec![0u8; length];
    reader.read_exact(&mut req.body).map_err(ReadError::Io)?;
    Ok(req)
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(&'static str, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn json(status: u16, body: String) -> Self {
        Self {
            status,
            headers: vec![("Content-Type", "application/json".to_string())],
            body: body.into_bytes(),
        }
    }

    pub fn error(status: u16, msg: &str) -> Self {
        Self::json(status, format!("{{\"error\":{}}}", json_string(msg)))
    }

    pub fn with_header(mut self, name: &'static str, value: &str) -> Self {
        self.headers.push((name, value.to_string()));
        self
    }

    pub fn write_to(&self, w: &mut impl Write) -> io::Result<()> {
        let mut buf = Vec::with_capacity(128 + self.body.len());
        write!(buf, "HTTP/1.1 {} {}\r\n", self.status, reason(self.status))?;
        for (k, v) in &self.headers {
            write!(buf, "{k}: {v}\r\n")?;
        }
        write!(
            buf,
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            self.body.len()
        )?;
        buf.extend_from_slice(&self.body);
        w.write_all(&buf)?;
        w.flush()
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

pub fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
