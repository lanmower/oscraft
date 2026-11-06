import { readItem, writeItem, removeItem } from "./safe_storage.js";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const here = dirname(fileURLToPath(import.meta.url));
const play = readFileSync(resolve(here, "play.js"), "utf8");
if (/localStorage[.]/.test(play)) throw new Error("play.js still calls localStorage directly");
const original = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
const boom = () => { throw new Error("storage denied"); };
Object.defineProperty(globalThis, "localStorage", { configurable: true, writable: true, value: { getItem: boom, setItem: boom, removeItem: boom } });
try {
  if (readItem("k") !== null) throw new Error("readItem did not contain the failure");
  if (!writeItem("k", "v")) throw new Error("writeItem did not report the failure");
  if (!removeItem("k")) throw new Error("removeItem did not report the failure");
} finally {
  if (original) Object.defineProperty(globalThis, "localStorage", original);
  else delete globalThis.localStorage;
}
console.log("safe storage ok (3 guarded helpers, no raw localStorage in play.js)");
