use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use crate::http::{Response, PROTOCOL_VERSION};

pub const SITE_PREFIX: &str = "/site/";

pub fn serve(root: &Path, request_path: &str) -> Response {
    let rel = request_path.strip_prefix(SITE_PREFIX).unwrap_or("");
    let rel = if rel.is_empty() { "index.html" } else { rel };
    match resolve(root, rel) {
        Some(path) => match fs::read(&path) {
            Ok(body) => Response {
                status: 200,
                headers: vec![("Content-Type", mime(&path).to_string())],
                body,
            },
            Err(_) => Response::error(404, "no such page file"),
        },
        None => Response::error(404, "no such page file"),
    }
}

fn resolve(root: &Path, rel: &str) -> Option<PathBuf> {
    let mut path = root.to_path_buf();
    for part in Path::new(rel).components() {
        match part {
            Component::Normal(name) => path.push(name),
            _ => return None,
        }
    }
    path.is_file().then_some(path)
}

fn mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "glb" => "model/gltf-binary",
        _ => "application/octet-stream",
    }
}

const PAGE_SCRIPT: &str = "main.js";

struct Stamp {
    path: PathBuf,
    mtime: Option<SystemTime>,
    len: u64,
    hash: u64,
}

static STAMP: Mutex<Option<Stamp>> = Mutex::new(None);

pub fn version(root: &Path) -> Response {
    let path = root.join(PAGE_SCRIPT);
    let Ok(meta) = fs::metadata(&path) else {
        return Response::error(404, "no page script");
    };
    let (mtime, len) = (meta.modified().ok(), meta.len());
    let mut cached = STAMP.lock().unwrap_or_else(|e| e.into_inner());
    let fresh = matches!(&*cached, Some(s) if s.path == path && s.mtime == mtime && s.len == len);
    if !fresh {
        let Ok(body) = fs::read(&path) else {
            return Response::error(404, "no page script");
        };
        *cached = Some(Stamp { path, mtime, len, hash: fnv1a(&body) });
    }
    let hash = cached.as_ref().map_or(0, |s| s.hash);
    Response::json(200, format!("{{\"proto\":{},\"page\":\"{hash:016x}\"}}", PROTOCOL_VERSION))
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3))
}
