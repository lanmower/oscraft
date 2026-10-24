// Hero voice lines, played through WebAudio. A pack ships voices/ with a manifest.json written by the
// voice pipeline; the page reads it once and decodes each line on first use.
//
// Nothing here is required to run a match: with no manifest, play() does nothing. That keeps the pack
// playable while the voices are being generated. With no AudioContext, or one that cannot start, play()
// resolves false and nothing throws.
//
// Every line runs through one gain node, made once. Mute and volume change that node's level, so a change
// never recreates nodes. Mute is stored in localStorage, so it survives a reload.

const MUTE_KEY = "oscraft.voicesMuted";
const LEVEL_FADE_S = 0.015; // a short ramp, so toggling mid-line does not click

// The line to play for one hero event. A hero may have several lines per event, named "<event>_0",
// "<event>_1", ... in the manifest: different lines, not different takes, so one is picked at random
// and a hero does not say the same thing twice. A hero with a single line keeps that shape too.
function lineFor(hero, event) {
  const direct = hero[event];
  if (direct) return direct;
  const variants = [];
  for (const key of Object.keys(hero)) {
    if (key.startsWith(`${event}_`) && /^\d+$/.test(key.slice(event.length + 1))) variants.push(key);
  }
  if (variants.length === 0) return null;
  return hero[variants[Math.floor(Math.random() * variants.length)]];
}

function readMuted() {
  try {
    return globalThis.localStorage?.getItem(MUTE_KEY) === "1";
  } catch {
    return false;
  }
}

function writeMuted(on) {
  try {
    globalThis.localStorage?.setItem(MUTE_KEY, on ? "1" : "0");
  } catch {
    // Storage is unavailable: the choice lasts for this page only.
  }
}

// `url` resolves a path inside the pack's voices directory: the caller owns that root, so this module
// never names one and a pack without voices is never probed.
export function createVoices({ url, audioContext = null }) {
  let manifest = null;
  let ctx = audioContext;
  let gain = null;
  let volume = 1;
  let muted = readMuted();
  const buffers = new Map(); // path -> AudioBuffer
  const failed = new Set();
  const active = new Map(); // hero -> the source playing its line

  async function loadManifest() {
    if (manifest !== null) return manifest;
    try {
      const res = await fetch(url("manifest.json"));
      if (!res.ok) throw new Error(`manifest HTTP ${res.status}`);
      manifest = await res.json();
    } catch {
      manifest = false; // no voices in this pack
    }
    return manifest;
  }

  function level() {
    return muted ? 0 : volume;
  }

  function applyLevel() {
    if (!gain) return;
    try {
      gain.gain.setTargetAtTime(level(), ctx.currentTime, LEVEL_FADE_S);
    } catch {
      gain.gain.value = level();
    }
  }

  // The context and its gain node, each created once. Returns null when WebAudio is missing or will not start.
  function context() {
    if (!ctx) {
      const Ctor = globalThis.AudioContext || globalThis.webkitAudioContext;
      if (!Ctor) return null;
      try {
        ctx = new Ctor();
      } catch {
        return null;
      }
    }
    if (!gain) {
      try {
        gain = ctx.createGain();
        gain.gain.value = level();
        gain.connect(ctx.destination);
      } catch {
        gain = null;
        return null;
      }
    }
    return ctx;
  }

  async function bufferFor(rel) {
    if (buffers.has(rel)) return buffers.get(rel);
    if (failed.has(rel)) return null;
    const ac = context();
    if (!ac) return null;
    try {
      const res = await fetch(url(rel));
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const buf = await ac.decodeAudioData(await res.arrayBuffer());
      buffers.set(rel, buf);
      return buf;
    } catch {
      failed.add(rel);
      return null;
    }
  }

  // Plays one hero's line for one event (spawn, attack, ability, death). Resolves true when a line was
  // started. Overlapping calls for the same hero cut the previous one, so a burst of attacks does not
  // stack into noise. While muted, nothing is fetched or played.
  async function play(hero, event) {
    if (muted) return false;
    try {
      const m = await loadManifest();
      if (!m || !m[hero]) return false;
      const rel = lineFor(m[hero], event);
      if (!rel) return false;
      const ac = context();
      if (!ac) return false;
      if (ac.state === "suspended") await ac.resume().catch(() => {});
      const buf = await bufferFor(rel);
      if (!buf || muted) return false;
      const previous = active.get(hero);
      if (previous) {
        try { previous.stop(); } catch { /* already ended */ }
      }
      const source = ac.createBufferSource();
      source.buffer = buf;
      source.connect(gain);
      active.set(hero, source);
      source.onended = () => { if (active.get(hero) === source) active.delete(hero); };
      source.start();
      return true;
    } catch {
      return false;
    }
  }

  // Mute is one switch: setMuted(true) silences every line, toggleMute() flips it. The choice is stored.
  function setMuted(on) {
    muted = Boolean(on);
    writeMuted(muted);
    applyLevel();
    return muted;
  }

  // Volume is a 0..1 level on the shared gain node. It applies to lines already playing.
  function setVolume(value) {
    const v = Number(value);
    if (Number.isFinite(v)) volume = Math.min(1, Math.max(0, v));
    applyLevel();
    return volume;
  }

  return {
    play,
    // The heroes that have voices, for the debug surface.
    async heroes() {
      const m = await loadManifest();
      return m ? Object.keys(m) : [];
    },
    isMuted: () => muted,
    setMuted,
    toggleMute: () => setMuted(!muted),
    volume: () => volume,
    setVolume,
  };
}
