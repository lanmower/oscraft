use std::path::Path;

use osdt_data::persist;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.as_slice() {
        [_, mode, path] if mode == "check" => {
            let image = std::fs::read(path)?;
            persist::check_version(&image)?;
            println!("version ok");
        }
        [_, src, out] => {
            let image = osdt_data::generate(Path::new(src))?;
            std::fs::write(out, image)?;
        }
        _ => anyhow::bail!("usage: osdt-data <src_dir> <out_file> | osdt-data check <image>"),
    }
    Ok(())
}
