import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const relayPath = resolve(here, "../rust/crates/relay/src/main.rs");

export function checkRelaySource(src) {
  const prod = src.split("#[cfg(test)]")[0];
  const comparator = /fn event_order\(a: &Value, b: &Value\) -> std::cmp::Ordering \{([\s\S]*?)\n\}/.exec(prod);
  if (!comparator) throw new Error("relay event_order comparator is missing");
  if (!comparator[1].includes("\"created_at\"") || !comparator[1].includes("\"id\"")) throw new Error("the comparator is not total on created_at and id");
  if (!prod.includes("created_b.cmp(&created_a).then_with(|| id_a.cmp(id_b))")) throw new Error("the comparator lost its created_at-then-id order");
  const eventOrderCalls = [...prod.matchAll(/event_order\(/g)].length;
  const sortCalls = [...prod.matchAll(/sort_by\(/g)].length;
  if (eventOrderCalls !== 2) throw new Error("event_order is defined or called outside on_req (" + eventOrderCalls + " occurrences)");
  if (sortCalls !== 1) throw new Error("a second sort_by appeared outside on_req (" + sortCalls + " occurrences)");
  if (!prod.includes("hits.sort_by(|a, b| event_order(")) throw new Error("on_req does not use the total-order comparator");
  if (!src.includes("fn equal_timestamps_sort_by_id")) throw new Error("the named REQ tie-order test is missing");
}

function selfTest() {
  const fixture = [
    "fn event_order(a: &Value, b: &Value) -> std::cmp::Ordering {",
    "    created_b.cmp(&created_a).then_with(|| id_a.cmp(id_b))",
    "    \"created_at\" \"id\"",
    "}",
    "fn deliver(state: &State, ev: &Value) {",
    "    send(state, sub.conn, ev);",
    "}",
    "fn on_req(state: &mut State) {",
    "    hits.sort_by(|a, b| event_order(*a, *b));",
    "}",
    "fn equal_timestamps_sort_by_id() {}",
  ].join("\n");
  checkRelaySource(fixture);
  const rawStringTrap = fixture.replace("    send(state, sub.conn, ev);", "    let trap = r#\"} \"#; hits.sort_by(|a, b| event_order(*a, *b));");
  let caught = false;
  try { checkRelaySource(rawStringTrap); } catch { caught = true; }
  if (!caught) failMutation("a sort_by after a raw-string brace in deliver");
  const outsideOnReq = fixture.replace("fn deliver(", "fn extra(state: &State) { event_order(&a, &b); }\nfn deliver(");
  caught = false;
  try { checkRelaySource(outsideOnReq); } catch { caught = true; }
  if (!caught) failMutation("an event_order call outside on_req");
  console.log("relay REQ order ok (global assertions, 2 mutation cases, live delivery stays arrival-ordered)");
}

function failMutation(what) { throw new Error("the checker missed " + what); }

const src = readFileSync(relayPath, "utf8");
checkRelaySource(src);
if (process.argv.includes("--self-test")) selfTest();
else console.log("relay REQ order ok (global assertions, live delivery stays arrival-ordered)");
