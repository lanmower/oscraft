#[cfg(target_arch = "wasm32")]
fn main() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    wasip3_guest::run(stdin.lock(), stdout.lock())
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: oscraft_wasip3 <guest.wasm>")?;
    let bytes = std::fs::read(&path).map_err(|e| format!("read guest {path}: {e}"))?;
    if bytes.len() < 8 || &bytes[..4] != b"\0asm" {
        return Err(format!("guest {path} is not a wasm binary"));
    }
    println!("wasip3 build ok bytes={}", bytes.len());
    Ok(())
}
