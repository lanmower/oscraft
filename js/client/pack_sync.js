// Live pack propagation over the wire. The host turns its saved changes into PACK_FILE frames; a peer reassembles them
// into its own Pack, checks each file against its content hash, and asks for referenced (large) files with PACK_WANT.
// Applying a change is the caller's job (play.js): this module only moves bytes and keeps the pack in step.
import { FRAME, MAX_PACK_CHUNK_BYTES, PACK_FLAG_DELETE, PACK_FLAG_REF } from "./wire/frame.js";
import { contentHash, toBytes } from "./pack.js";

// Files up to this size travel as inline chunks; larger ones travel by reference (the peer asks for them by hash).
export const MAX_INLINE_FILE_BYTES = 256 * 1024;

// Inline chunk frames for one whole file (an empty file is one empty chunk).
export function chunkFrames(name, bytes, hash) {
  const frames = [];
  for (let offset = 0; ; offset += MAX_PACK_CHUNK_BYTES) {
    frames.push({ type: FRAME.PACK_FILE, name, hash, size: bytes.length, offset, flags: 0, bytes: bytes.subarray(offset, offset + MAX_PACK_CHUNK_BYTES) });
    if (offset + MAX_PACK_CHUNK_BYTES >= bytes.length) break;
  }
  return frames;
}

// The frames that announce one saved change. `blobs` (Map hash -> { name, bytes }) keeps referenced bytes for PACK_WANT answers.
export function changeFrames(change, blobs) {
  if (change.deleted) return [{ type: FRAME.PACK_FILE, name: change.path, hash: 0, size: 0, offset: 0, flags: PACK_FLAG_DELETE, bytes: new Uint8Array(0) }];
  const bytes = toBytes(change.bytes);
  const hash = contentHash(bytes);
  if (bytes.length > MAX_INLINE_FILE_BYTES) {
    blobs.set(hash, { name: change.path, bytes });
    return [{ type: FRAME.PACK_FILE, name: change.path, hash, size: bytes.length, offset: 0, flags: PACK_FLAG_REF, bytes: new Uint8Array(0) }];
  }
  return chunkFrames(change.path, bytes, hash);
}

// Every file of a pack as PACK_FILE frames, for a peer that joins: it holds the same bytes the host does.
export function packStateFrames(pack, blobs) {
  return pack.paths().flatMap((path) => changeFrames({ path, bytes: pack.get(path) }, blobs));
}

// The receiving side: `pack` is the peer's Pack. accept(frame) returns one of
//   { kind: "wait" }                 a chunk of a file not yet complete;
//   { kind: "want", hash, name }     a reference: send PACK_WANT with this hash to the host;
//   { kind: "applied", path, deleted, bytes } a whole file that was verified and stored in the pack;
//   { kind: "refused", reason }      a hash or size mismatch, or a path the pack may not hold.
export function createPackReceiver(pack) {
  const partial = new Map(); // name -> { hash, size, parts: Uint8Array, got }
  return {
    accept(frame) {
      if (frame.flags & PACK_FLAG_DELETE) {
        try { pack.delete(frame.name); } catch (err) { return { kind: "refused", reason: err.message }; }
        partial.delete(frame.name);
        return { kind: "applied", path: frame.name, deleted: true };
      }
      if (frame.flags & PACK_FLAG_REF) return { kind: "want", hash: frame.hash, name: frame.name };
      let p = partial.get(frame.name);
      if (!p || p.hash !== frame.hash || p.size !== frame.size) {
        p = { hash: frame.hash, size: frame.size, parts: new Uint8Array(frame.size), got: 0 };
        partial.set(frame.name, p);
      }
      if (frame.offset + frame.bytes.length > p.size) return { kind: "refused", reason: `chunk past the end of ${frame.name}` };
      p.parts.set(frame.bytes, frame.offset);
      p.got += frame.bytes.length;
      if (p.got < p.size) return { kind: "wait" };
      partial.delete(frame.name);
      return this.store(frame.name, p.parts, p.hash);
    },
    // Stores verified bytes (a whole file, from chunks or a PACK_WANT answer) under the pack path.
    store(name, bytes, hash) {
      if (contentHash(bytes) !== hash) return { kind: "refused", reason: `${name} fails its hash` };
      try {
        const isTextFile = /\.(lua|osd|json|txt|tsv)$/i.test(name);
        const value = isTextFile ? new TextDecoder().decode(bytes) : bytes;
        pack.set(name, value);
        pack.saved.set(name, hash);
        return { kind: "applied", path: name, deleted: false, bytes };
      } catch (err) {
        return { kind: "refused", reason: err.message };
      }
    },
  };
}
