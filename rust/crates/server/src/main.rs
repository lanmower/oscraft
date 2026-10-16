fn main() {
    if let Err(e) = server::run() {
        server::emit_err!(server::log::Ev::Fatal, error = e);
        std::process::exit(1);
    }
}
