//! Language server for mod files over stdio with Content-Length framing.
//! `.star` diagnostics are the engine validator's own error text, produced by
//! `osdt_data::generate` on a staged copy of the document's directory.
//! `.lua` diagnostics come from the mod host's load path (`ModHost::load`) on a
//! staged copy of the buffer, so a syntax or load error carries its line.
//! Completion and hover read the engine builtin table from `osdt_data::builtins`
//! (`.star`) and the host globals table below (`.lua`).
//! No network port is opened.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mod_host::ModHost;
use serde_json::{json, Value};

/// What a mod sees from the host, as (name, signature, doc). Globals come from
/// `mod_host::HOST_GLOBALS`, events from `mod_host::EVENTS`; both are the lists
/// the host itself registers from, so completion cannot drift from it.
fn host_entries() -> Vec<(&'static str, String, &'static str)> {
    let globals = mod_host::HOST_GLOBALS
        .iter()
        .map(|name| (*name, name.to_string(), "Host global registered by mod-host."));
    let events = mod_host::EVENTS
        .iter()
        .map(|(name, args)| (*name, format!("{name}({args})"), "Mod event the host calls."));
    globals.chain(events).collect()
}

#[derive(Clone, Copy)]
enum Kind {
    Star,
    Lua,
}

fn main() -> Result<()> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut docs: HashMap<String, String> = HashMap::new();
    while let Some(message) = read_message(&mut input)? {
        if message["method"] == "exit" {
            break;
        }
        for reply in handle(&message, &mut docs) {
            write_message(&mut output, &reply)?;
        }
    }
    Ok(())
}

fn read_message(input: &mut impl BufRead) -> Result<Option<Value>> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            length = Some(value.trim().parse::<usize>()?);
        }
    }
    let length = length.context("message without Content-Length")?;
    let mut body = vec![0u8; length];
    input.read_exact(&mut body)?;
    Ok(Some(serde_json::from_slice(&body)?))
}

fn write_message(output: &mut impl Write, message: &Value) -> Result<()> {
    let body = serde_json::to_string(message)?;
    write!(output, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    output.flush()?;
    Ok(())
}

fn handle(message: &Value, docs: &mut HashMap<String, String>) -> Vec<Value> {
    let method = message["method"].as_str().unwrap_or("");
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let params = &message["params"];
    let uri = params["textDocument"]["uri"].as_str().unwrap_or("").to_owned();
    let mut out = Vec::new();
    match method {
        "initialize" => out.push(respond(&id, json!({
            "capabilities": {
                "textDocumentSync": 1,
                "completionProvider": {"triggerCharacters": ["."]},
                "hoverProvider": true
            },
            "serverInfo": {"name": "oscraft-lsp"}
        }))),
        "textDocument/didOpen" => {
            let text = params["textDocument"]["text"].as_str().unwrap_or("").to_owned();
            out.push(publish(&uri, &text));
            docs.insert(uri, text);
        }
        "textDocument/didChange" => {
            let text = params["contentChanges"][0]["text"].as_str().unwrap_or("").to_owned();
            out.push(publish(&uri, &text));
            docs.insert(uri, text);
        }
        "textDocument/completion" => {
            let items: Vec<Value> = match kind_of(&uri) {
                Some(Kind::Star) => osdt_data::builtins::signatures()
                    .into_iter()
                    .map(|(name, signature)| json!({"label": name, "kind": 3, "detail": signature}))
                    .collect(),
                Some(Kind::Lua) => host_entries()
                    .into_iter()
                    .map(|(name, signature, doc)| json!({"label": name, "kind": 3, "detail": signature, "documentation": doc}))
                    .collect(),
                None => Vec::new(),
            };
            out.push(respond(&id, json!({"isIncomplete": false, "items": items})));
        }
        "textDocument/hover" => {
            let line = params["position"]["line"].as_u64().unwrap_or(0) as usize;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as usize;
            let word = docs.get(&uri).and_then(|text| word_at(text, line, character));
            let result = match (kind_of(&uri), word) {
                (Some(Kind::Star), Some(word)) => match osdt_data::builtins::signatures().into_iter().find(|(name, _)| *name == word) {
                    Some((_, signature)) => json!({
                        "contents": {"kind": "plaintext", "value": format!("{signature}\nengine builtin from the osdt-data schema")}
                    }),
                    None => Value::Null,
                },
                (Some(Kind::Lua), Some(word)) => match host_entries().into_iter().find(|(name, _, _)| *name == word) {
                    Some((_, signature, doc)) => json!({"contents": {"kind": "plaintext", "value": format!("{signature}\n{doc}")}}),
                    None => Value::Null,
                },
                _ => Value::Null,
            };
            out.push(respond(&id, result));
        }
        _ => {
            if !id.is_null() {
                out.push(respond(&id, Value::Null));
            }
        }
    }
    out
}

fn respond(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn publish(uri: &str, text: &str) -> Value {
    let diagnostics = match kind_of(uri) {
        Some(Kind::Star) => match engine_check(&uri_path(uri), text) {
            Ok(()) => Vec::new(),
            Err(message) => vec![diagnostic(0, message, "osdt-data")],
        },
        Some(Kind::Lua) => match lua_check(&uri_path(uri), text) {
            Ok(()) => Vec::new(),
            Err(failure) => vec![diagnostic(failure.line.map_or(0, |line| line.saturating_sub(1)), failure.message, "mod-host")],
        },
        None => Vec::new(),
    };
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {"uri": uri, "diagnostics": diagnostics}
    })
}

fn diagnostic(line: u64, message: String, source: &str) -> Value {
    json!({
        "range": {"start": {"line": line, "character": 0}, "end": {"line": line, "character": 0}},
        "severity": 1,
        "source": source,
        "message": message,
    })
}

fn kind_of(uri: &str) -> Option<Kind> {
    if uri.ends_with(".star") {
        Some(Kind::Star)
    } else if uri.ends_with(".lua") {
        Some(Kind::Lua)
    } else {
        None
    }
}

fn uri_path(uri: &str) -> PathBuf {
    let path = uri.strip_prefix("file://").unwrap_or(uri);
    let bytes = path.as_bytes();
    let windows_drive = bytes.len() > 2 && bytes[0] == b'/' && bytes[2] == b':';
    PathBuf::from(if windows_drive { &path[1..] } else { path })
}

/// Runs the engine on a staged copy of the document's directory, so the
/// buffer text is what gets validated. The staged path in any engine message
/// is rewritten to the document's own path.
fn engine_check(doc: &Path, text: &str) -> std::result::Result<(), String> {
    let name = doc.file_name().ok_or("document has no file name")?;
    let scratch = std::env::temp_dir().join(format!("oscraft-lsp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let verdict = match stage(doc, name, text, &scratch) {
        Ok(staged) => osdt_data::generate(&scratch)
            .err()
            .map(|e| e.to_string().replace(&staged, &doc.display().to_string())),
        Err(e) => Some(e.to_string()),
    };
    let _ = std::fs::remove_dir_all(&scratch);
    match verdict {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

fn stage(doc: &Path, name: &OsStr, text: &str, scratch: &Path) -> io::Result<String> {
    std::fs::create_dir_all(scratch)?;
    if let Some(dir) = doc.parent() {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            let sibling = path.file_name();
            let is_star_file = path.extension().is_some_and(|ext| ext == "star");
            if let (true, Some(sibling)) = (is_star_file, sibling) {
                if sibling != name {
                    std::fs::copy(&path, scratch.join(sibling))?;
                }
            }
        }
    }
    let staged = scratch.join(name);
    std::fs::write(&staged, text)?;
    Ok(staged.display().to_string())
}

struct LuaFailure {
    line: Option<u64>,
    message: String,
}

/// Loads the buffer through the mod host, the same `ModHost::load` a mod takes
/// at startup, from a staged copy named after the document.
fn lua_check(doc: &Path, text: &str) -> std::result::Result<(), LuaFailure> {
    let name = doc.file_name().ok_or_else(|| LuaFailure {
        line: None,
        message: "document has no file name".into(),
    })?;
    let scratch = std::env::temp_dir().join(format!("oscraft-lsp-lua-{}", std::process::id()));
    let verdict = lua_load(name, text, &scratch);
    let _ = std::fs::remove_dir_all(&scratch);
    verdict
}

fn lua_load(name: &OsStr, text: &str, scratch: &Path) -> std::result::Result<(), LuaFailure> {
    let fail = |message: String| LuaFailure { line: None, message };
    std::fs::create_dir_all(scratch).map_err(|e| fail(e.to_string()))?;
    let staged = scratch.join(name);
    std::fs::write(&staged, text).map_err(|e| fail(e.to_string()))?;
    let image = osdt_data::generate(&data_dir()).map_err(|e| fail(format!("engine data: {e}")))?;
    let mut host = ModHost::new(&image).map_err(|e| fail(e.to_string()))?;
    host.load(&staged).map_err(|e| locate(name, &e.to_string()))
}

/// The engine's `.star` sources: `OSCRAFT_DATA_DIR` when set, else the workspace's rust/data.
fn data_dir() -> PathBuf {
    std::env::var_os("OSCRAFT_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"))
}

/// Reads Lua's "<name>:<line>: <message>" for a chunk named after the file. mlua
/// shortens long chunk names to "...<tail>", so the file name is matched, not the full path.
fn locate(name: &OsStr, text: &str) -> LuaFailure {
    let prefix = format!("{}:", name.to_string_lossy());
    if let Some(at) = text.find(&prefix) {
        let rest = &text[at + prefix.len()..];
        let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
        if let Ok(line) = rest[..digits].parse::<u64>() {
            let message = rest[digits..].trim_start_matches(':').trim().to_string();
            return LuaFailure { line: Some(line), message };
        }
    }
    LuaFailure { line: None, message: text.to_string() }
}

fn word_at(text: &str, line: usize, character: usize) -> Option<String> {
    let chars: Vec<char> = text.lines().nth(line)?.chars().collect();
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let at = character.min(chars.len());
    let mut start = at;
    while start > 0 && is_word(chars[start - 1]) {
        start -= 1;
    }
    let mut end = at;
    while end < chars.len() && is_word(chars[end]) {
        end += 1;
    }
    (start < end).then(|| chars[start..end].iter().collect())
}