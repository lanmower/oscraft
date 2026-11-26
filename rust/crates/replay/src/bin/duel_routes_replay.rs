use std::path::{Path, PathBuf};
use std::process::ExitCode;

use replay::duel;

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))
}

fn run() -> Result<String, String> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: duel_routes_replay <pack-dir> <out-dir>";
    let pack = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().ok_or(usage)?);
    let image = osdt_data::generate(&pack).map_err(|e| format!("compile {}: {e:?}", pack.display()))?;
    let recorded = duel::record(&image)?;
    duel::verify(&image, &recorded.replay, &recorded.hashes)?;
    std::fs::create_dir_all(&out).map_err(|e| format!("create {}: {e}", out.display()))?;
    write(&out.join("duel-routes.replay"), recorded.replay.as_bytes())?;
    write(&out.join("duel-routes.hashes"), recorded.hash_table().as_bytes())?;
    let last = recorded.hashes.last().map_or(0, |&(_, hash)| hash);
    Ok(format!("ticks={} state_hash={last:016x}", duel::TICKS))
}

fn main() -> ExitCode {
    match run() {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("duel_routes_replay: {message}");
            ExitCode::FAILURE
        }
    }
}
