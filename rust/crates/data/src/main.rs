use std::path::Path;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        anyhow::bail!("usage: osdt-data <src_dir> <out_file>");
    }
    let image = osdt_data::generate(Path::new(&args[1]))?;
    std::fs::write(&args[2], image)?;
    Ok(())
}
