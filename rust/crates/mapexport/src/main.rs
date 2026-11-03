use std::path::PathBuf;
use std::process::ExitCode;

fn run() -> Result<String, String> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: mapexport <package-dir> <out.osdt>";
    let dir = PathBuf::from(args.next().ok_or(usage)?);
    let out = args.next().ok_or(usage)?;
    let manifest_path = dir.join("manifest.json");
    let text = std::fs::read_to_string(&manifest_path).map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
    let manifest: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", manifest_path.display()))?;
    let data = manifest["data"].as_array().ok_or_else(|| format!("{}: data must be a list", manifest_path.display()))?;
    let files = data
        .iter()
        .map(|v| v.as_str().map(|name| dir.join(name)).ok_or_else(|| format!("{}: data entries are file names", manifest_path.display())))
        .collect::<Result<Vec<PathBuf>, String>>()?;
    let image = osdt_data::generate_files(&files).map_err(|e| format!("generate the map image from {}: {e:#}", dir.display()))?;
    std::fs::write(&out, &image).map_err(|e| format!("write {out}: {e}"))?;
    Ok(format!("wrote {} bytes to {out}", image.len()))
}

fn main() -> ExitCode {
    match run() {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("mapexport: {message}");
            ExitCode::FAILURE
        }
    }
}
