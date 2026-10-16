use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::http::Response;

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
