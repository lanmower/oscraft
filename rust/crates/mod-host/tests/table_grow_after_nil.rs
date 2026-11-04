use std::path::Path;

use mod_host::{build_image, ModHost};

const SPACINGS: [u32; 4] = [3, 7, 13, 97];

fn grow_after_nil_module(spacing: u32) -> String {
    format!(
        r#"
local t = {{}}
local live = {{}}
for round = 1, 12 do
    for i = 1, 24 do
        local k = round * 100000 + i * {spacing}
        t[k] = i
        if i % 3 == 0 then
            live[#live + 1] = k
        else
            t[k] = nil
        end
    end
end
for i = 1, #live do
    if t[live[i]] == nil then
        error("a live key was lost after the table grew")
    end
end
"#
    )
}

#[test]
fn a_key_set_then_cleared_with_nil_does_not_panic_when_the_table_grows() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
    let image = build_image(&dir).expect("build the moba image");
    let mut host = ModHost::new(&image).expect("load the moba image");
    for spacing in SPACINGS {
        let name = format!("grow_after_nil_{spacing}.lua");
        host.load_source(&name, &grow_after_nil_module(spacing))
            .unwrap_or_else(|error| panic!("spacing {spacing}: {error}"));
    }
}
