import { once } from "node:events";
import { EphemeralRelay } from "../../js/client/vendor/wireweave/src/ephemeral-relay.js";
import { WebSocketServer } from "../../js/client/vendor/wireweave/node_modules/ws/wrapper.mjs";

const MATCH_ROOM = /^#match-/;

export function raceVerdict(hashes) {
  const reasons = [];
  for (const hash of hashes) if (!MATCH_ROOM.test(hash)) reasons.push("a page is not in a match room: " + (hash || "(no room)"));
  const rooms = [...new Set(hashes)];
  if (rooms.length > 1) reasons.push("the pages split into " + rooms.length + " match rooms: " + rooms.join(" "));
  return { ok: reasons.length === 0, reasons };
}

export async function startRelay(port = 0) {
  const relay = new EphemeralRelay({ WebSocketServer, port });
  await once(relay.wss, "listening");
  return relay;
}

export async function runRace(context, { relayUrl, ns, pages = 3, gapMs = 1000, settleMs = 30000 }) {
  const url = "http://127.0.0.1:8853/js/client/play.html?ci=1&ice=" + encodeURIComponent("[]") + "&relay=" + encodeURIComponent(relayUrl) + "&ns=" + ns;
  const opened = [];
  for (let n = 0; n < pages; n += 1) {
    if (n > 0) await new Promise((resolve) => setTimeout(resolve, gapMs));
    const page = await context.newPage();
    opened.push(page);
    await page.goto(url, { waitUntil: "domcontentloaded", timeout: 50000 });
  }
  await new Promise((resolve) => setTimeout(resolve, settleMs));
  const hashes = opened.map((page) => new URL(page.url()).hash);
  for (const page of opened) await page.close();
  return { ns, hashes, ...raceVerdict(hashes) };
}
