export const LINK_STATES = Object.freeze({
  good: Object.freeze({ glyph: "●", colour: "#7bd88f", word: "good" }),
  fair: Object.freeze({ glyph: "▲", colour: "#f2c14e", word: "fair" }),
  poor: Object.freeze({ glyph: "■", colour: "#ff6b6b", word: "poor" }),
  unknown: Object.freeze({ glyph: "○", colour: "#9aa4b2", word: "no data" }),
});

const FPS_FAIR = 45;
const FPS_POOR = 20;
const RTT_FAIR_MS = 120;
const RTT_POOR_MS = 250;

export function framesPerSecond(timer) {
  const { meanMs } = timer.summary();
  return meanMs > 0 ? 1000 / meanMs : 0;
}

export function selectedRoundTripMs(report) {
  let best = null;
  report.forEach((stat) => {
    if (stat.type !== "candidate-pair" || stat.state !== "succeeded") return;
    if (!(stat.nominated || stat.selected)) return;
    if (typeof stat.currentRoundTripTime !== "number") return;
    const ms = stat.currentRoundTripTime * 1000;
    if (best === null || ms < best) best = ms;
  });
  return best;
}

export async function measureRoundTripMs(peerConnections) {
  let best = null;
  for (const pc of peerConnections) {
    let report;
    try {
      report = await pc.getStats();
    } catch {
      continue;
    }
    const ms = selectedRoundTripMs(report);
    if (ms !== null && (best === null || ms < best)) best = ms;
  }
  return best;
}

export function linkState(fps, rttMs) {
  if (!fps && rttMs === null) return "unknown";
  if ((fps && fps < FPS_POOR) || (rttMs !== null && rttMs > RTT_POOR_MS)) return "poor";
  if ((fps && fps < FPS_FAIR) || (rttMs !== null && rttMs > RTT_FAIR_MS)) return "fair";
  return "good";
}

export function describeReadout(fps, rttMs) {
  const state = linkState(fps, rttMs);
  const fpsText = fps ? `${Math.round(fps)} FPS` : "-- FPS";
  const rttText = rttMs === null ? "-- ms" : `${Math.round(rttMs)} ms`;
  const text = `${fpsText}  ${rttText}`;
  return {
    state,
    glyph: LINK_STATES[state].glyph,
    colour: LINK_STATES[state].colour,
    text,
    label: `Connection ${LINK_STATES[state].word}: ${text}`,
  };
}

export function createNetReadout({ doc, timer, peerConnections, refreshMs = 1000 }) {
  const element = doc.createElement("div");
  element.id = "net-readout";
  element.setAttribute("role", "status");
  element.hidden = true;
  Object.assign(element.style, {
    position: "fixed",
    top: "8px",
    right: "8px",
    zIndex: "1000",
    padding: "2px 8px",
    borderRadius: "4px",
    background: "rgba(0,0,0,0.6)",
    font: "12px monospace",
    pointerEvents: "none",
    whiteSpace: "pre",
  });
  const mark = doc.createElement("span");
  const body = doc.createElement("span");
  element.append(mark, body);
  doc.body.append(element);

  let rttMs = null;
  let timerId = null;
  let shown = false;

  function paint() {
    const readout = describeReadout(framesPerSecond(timer), rttMs);
    mark.textContent = `${readout.glyph} `;
    mark.style.color = readout.colour;
    body.textContent = readout.text;
    element.style.color = readout.colour;
    element.title = readout.label;
    element.setAttribute("aria-label", readout.label);
  }

  async function refresh() {
    rttMs = await measureRoundTripMs(peerConnections);
    if (shown) paint();
  }

  function show() {
    shown = true;
    element.hidden = false;
    paint();
    refresh();
    timerId = doc.defaultView.setInterval(refresh, refreshMs);
  }

  function hide() {
    shown = false;
    element.hidden = true;
    if (timerId !== null) doc.defaultView.clearInterval(timerId);
    timerId = null;
  }

  function toggle() {
    if (shown) hide();
    else show();
    return shown;
  }

  return { toggle, show, hide, paint, refresh, isShown: () => shown, element };
}
