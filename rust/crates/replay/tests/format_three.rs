use std::path::Path;
use replay::{Recording, FORMAT_HEADER};

#[test]
fn format_three_render_parse_round_trip() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
    let image = osdt_data::generate(&dir).expect("compile the moba scenario");
    let sim = sim::load(&image, 4096).expect("load the moba scenario");
    let mut rec = Recording::new("moba.osdt", 4096, &sim);
    let text = rec.render(&sim);
    assert!(text.starts_with(FORMAT_HEADER), "the format-3 header is written");
    let mut parsed = Recording::parse(&text).expect("format-3 text parses");
    let again = parsed.render(&sim);
    assert_eq!(text, again, "render, parse and render again are byte-identical");
}

#[test]
fn a_foreign_header_is_refused() {
    assert!(Recording::parse("not an oscraft replay\n").is_err(), "a bad header fails");
    assert!(Recording::parse("").is_err(), "an empty replay fails");
}
