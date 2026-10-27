
#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};
use serde_json::{json, Value};

mod common;
use common::scenario;

const HEAD: &str = r#"
local function show(case, got)
  game.ui_event("vm", "{\"case\":\"" .. case .. "\",\"got\":\"" .. got .. "\"}")
end
return {
  on_action = function(name, args)
    if name ~= "vm" then return end
"#;
const TAIL: &str = r#"
  end,
}
"#;

fn run(body: &str) -> Vec<(String, String)> {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let src = [HEAD, body, TAIL].concat();
    host.load_mods(&[("vm.lua".to_owned(), src)]).expect("the case module loads");
    host.dispatch_action("vm", &json!({})).expect("dispatch");
    host.advance().expect("tick");
    host.take_ui()
        .iter()
        .map(|body| {
            let v: Value = serde_json::from_str(&format!("{{{body}}}")).expect("ui body");
            (v["data"]["case"].as_str().expect("case").to_owned(), v["data"]["got"].as_str().expect("got").to_owned())
        })
        .collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(c, g)| ((*c).to_owned(), (*g).to_owned())).collect()
}

#[test]
fn a_swap_of_two_targets_reads_both_values_first() {
    let got = run(r#"
    local a, b = { k = "A" }, { k = "B" }
    a, b = b, a
    show("swap", a.k .. b.k)
"#);
    assert_eq!(got, pairs(&[("swap", "BA")]), "a, b = b, a swaps the two values");
}

#[test]
fn a_rotation_of_three_targets_reads_all_values_first() {
    let got = run(r#"
    local a, b, c = { k = "A" }, { k = "B" }, { k = "C" }
    a, b, c = b, c, a
    show("rotate", a.k .. b.k .. c.k)
"#);
    assert_eq!(got, pairs(&[("rotate", "BCA")]), "a, b, c = b, c, a rotates the three values");
}

#[test]
fn an_indexed_swap_reads_both_slots_first() {
    let got = run(r#"
    local t = { "one", "two" }
    t[1], t[2] = t[2], t[1]
    show("indexed", t[1] .. t[2])
"#);
    assert_eq!(got, pairs(&[("indexed", "twoone")]), "t[1], t[2] = t[2], t[1] swaps the slots");
}

#[test]
fn a_target_reads_what_the_statement_has_not_stored_yet() {
    let got = run(r#"
    local i = 3
    local arr = {}
    i, arr[i] = i + 1, 20
    show("manual", tostring(arr[3]) .. " " .. tostring(arr[4]) .. " " .. tostring(i))
    local old = {}
    local cur = old
    cur, cur.x = {}, 5
    show("prefix", tostring(old.x) .. " " .. tostring(cur.x))
    local p, q = 1, 2
    p, q = 10, p
    show("reads", tostring(p) .. " " .. tostring(q))
"#);
    assert_eq!(
        got,
        pairs(&[("manual", "20 nil 4"), ("prefix", "5 nil"), ("reads", "10 1")]),
        "every value and table and key is read before the stores"
    );
}

#[test]
fn a_single_assignment_and_call_results_are_unchanged() {
    let got = run(r#"
    local s = 1
    s = s + 1
    show("single", tostring(s))
    local function two() return "p", "q" end
    local m, n, o
    m, n, o = 0, two()
    show("calls", tostring(m) .. n .. o)
"#);
    assert_eq!(got, pairs(&[("single", "2"), ("calls", "0pq")]), "one value and a call's results still store as before");
}

#[test]
fn a_single_field_target_evaluates_its_table_and_key_before_the_value() {
    let got = run(r#"
    local log = {}
    local function note(s) log[#log + 1] = s end
    local function f() note("f") return 1 end
    local function g() note("g") return 2 end
    local function obj() note("obj") return {} end
    local t = {}
    t[f()] = g()
    show("index", tostring(log[1]) .. "," .. tostring(log[2]))
    log = {}
    obj().x = g()
    show("prefix", tostring(log[1]) .. "," .. tostring(log[2]))
    local x
    x = g()
    show("local", tostring(x))
    show("stored", tostring(t[1]))
"#);
    assert_eq!(
        got,
        pairs(&[("index", "f,g"), ("prefix", "obj,g"), ("local", "2"), ("stored", "2")]),
        "a field target's table and key are evaluated before the value; a local target stores the value"
    );
}
