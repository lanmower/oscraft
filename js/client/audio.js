
const MUTE_KEY = "oscraft.voicesMuted";
const LEVEL_FADE_S = 0.015;

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
  }
}

let packPlayer = null;
export function packVoices() {
  return packPlayer;
}

export function createVoices({ url, audioContext = null }) {
  let manifest = null;
  let ctx = audioContext;
  let gain = null;
  let volume = 1;
  let muted = readMuted();
  const buffers = new Map();
  const failed = new Set();
  const active = new Map();

  async function loadManifest() {
    if (manifest !== null) return manifest;
    try {
      const res = await fetch(url("manifest.json"));
      if (!res.ok) throw new Error(`manifest HTTP ${res.status}`);
      manifest = await res.json();
    } catch {
      manifest = false;
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
        try { previous.stop(); } catch {  }
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

  function setMuted(on) {
    muted = Boolean(on);
    writeMuted(muted);
    applyLevel();
    return muted;
  }

  function setVolume(value) {
    const v = Number(value);
    if (Number.isFinite(v)) volume = Math.min(1, Math.max(0, v));
    applyLevel();
    return volume;
  }

  const voices = {
    play,
    playing: () => active.size,
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
  packPlayer = voices;
  return voices;
}

export function createSfx({ url, audioContext = null }) {
  let ctx = audioContext;
  const buffers = new Map();
  const failed = new Set();

  async function load(name) {
    if (buffers.has(name)) return buffers.get(name);
    if (failed.has(name)) return null;
    try {
      // Try .ogg first, then .mp3
      for (const ext of [".ogg", ".mp3"]) {
        const res = await fetch(url(`${name}${ext}`));
        if (res.ok) {
          const buffer = await ctx.decodeAudioData(await res.arrayBuffer());
          buffers.set(name, buffer);
          return buffer;
        }
      }
      throw new Error(`Could not load ${name} as .ogg or .mp3`);
    } catch {
      failed.add(name);
      return null;
    }
  }

  async function play(name) {
    if (readMuted()) return false;
    if (!ctx) {
      const Ctor = globalThis.AudioContext || globalThis.webkitAudioContext;
      if (!Ctor) return false;
      try { ctx = new Ctor(); } catch { return false; }
    }
    const buffer = await load(String(name));
    if (!buffer) return false;
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    source.connect(ctx.destination);
    source.start();
    return true;
  }

  return { play };
}
