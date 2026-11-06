import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const src = readFileSync(join(root, "rust/crates/relay/src/main.rs"), "utf8");
const comparator = /fn event_order\(a: &Value, b: &Value\) -> std::cmp::Ordering \{([\s\S]*?)\n\}/.exec(src);
if (!comparator) throw new Error("relay event_order comparator is missing");
if (!comparator[1].includes("\"created_at\"") || !comparator[1].includes("\"id\"")) throw new Error("the comparator is not total on created_at and id");
const req = /fn on_req\([\s\S]*?\n\}/.exec(src);
if (!req || !/hits\.sort_by\(\|a, b\| event_order\(/.test(req[0])) throw new Error("on_req does not use the total-order comparator");
const deliver = /fn deliver\([\s\S]*?\n\}/.exec(src);
if (!deliver) throw new Error("relay live delivery is missing");
if (/sort_by|event_order/.test(deliver[0])) throw new Error("live delivery must stay arrival-ordered, not sorted");
if (!/fn equal_timestamps_sort_by_id/.test(src)) throw new Error("the named REQ tie-order test is missing");
console.log("relay REQ order ok (total comparator, named test, live delivery stays arrival-ordered)");
