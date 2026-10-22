// Hero voice lines, played through WebAudio. A pack ships voices/ with a manifest.json written by the
// voice pipeline; the page reads it once and decodes each line on first use.
//
// Nothing here is required to run a match: with no manifest, play() does nothing. That keeps the pack
// playable while the voices are being generated.

export function createVoices({ url, audioContext = null }) {
  let manifest = null;
  let ctx = audioContext;
  const buffers = new Map(); // path -> AudioBuffer
  const failed = new Set();

  async function loadManifest() {
    if (manifest !== null) return manifest;
    try {
      const res = await fetch(url("voices/manifest.json"));
      if (!res.ok) throw new Error(`manifest HTTP ${res.status}`);
      manifest = await res.json();
    } catch {
      manifest = false; // no voices in this pack
    }
    return manifest;
  }

  function context() {
    if (ctx) return ctx;
    const Ctor = window.AudioContext || window.webkitAudioContext;
    if (!Ctor) return null;
    ctx = new Ctor();
    return ctx;
  }

  async function bufferFor(rel) {
    if (buffers.has(rel)) return buffers.get(rel);
    if (failed.has(rel)) return null;
    try {
      const ac = context();
      if (!ac) throw new Error("no AudioContext");
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
  // stack into noise.
  const active = new Map();

  async function play(hero, event) {
    const m = await loadManifest();
    if (!m || !m[hero]) return false;
    const rel = m[hero]?.[event];
    if (!rel) return false;
    const ac = context();
    if (!ac) return false;
    if (ac.state === "suspended") await ac.resume().catch(() => {});
    const buf = await bufferFor(`voices/${rel}`);
    if (!buf) return false;
    const previous = active.get(hero);
    if (previous) { try { previous.stop(); } catch { /* already ended */ } }
    const source = ac.createBufferSource();
    source.buffer = buf;
    source.connect(ac.destination);
    active.set(hero, source);
    source.onended = () => { if (active.get(hero) === source) active.delete(hero); };
    source.start();
    return true;
  }

  return {
    play,
    // The heroes that have voices, for the debug surface.
    async heroes() {
      const m = await loadManifest();
      return m ? Object.keys(m) : [];
    },
  };
}
