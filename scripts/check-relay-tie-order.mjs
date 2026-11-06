import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const src = readFileSync(join(root, "rust/crates/relay/src/main.rs"), "utf8");
const comparator = /fn event_order\(a: &Value, b: &Value\) -> std::cmp::Ordering \{([\s\S]*?)\n\}/.exec(src);
if (!comparator) throw new Error("relay event_order comparator is missing");
if (!comparator[1].includes("\"created_at\"") || !comparator[1].includes("\"id\"")) throw new Error("the comparator is not total on created_at and id");
if (!/hits\.sort_by\(\|a, b\| event_order\(/.test(src)) throw new Error("on_req does not use the total-order comparator");
if (!/fn equal_timestamps_sort_by_id/.test(src)) throw new Error("the named tie-order test is missing");
console.log("relay tie order ok (total comparator and named test)");
