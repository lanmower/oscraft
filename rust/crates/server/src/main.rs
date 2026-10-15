fn main() {
    if let Err(e) = server::run() {
        eprintln!("oscraftd: {e}");
        std::process::exit(1);
    }
}
