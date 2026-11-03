# Wire format

The binary frames the browser peers exchange over WebRTC data channels. The codec is `js/wire/frame.js` (pure: `Uint8Array` in and out, no DOM or Node APIs). `js/client/wire/frame.js` is a hand-kept copy of it; the page imports the copy. `js/wire/test.js` round-trips every frame type and checks the reject rules.

## Header

Every frame starts with two bytes: the frame type (1 byte) and `WIRE_VERSION` (1 byte, currently `1`). Integers are little-endian: `u8`, `u16`, `u32` unsigned, `i32` two's complement. Every field is fixed-width or length-prefixed; there are no padding bytes.

A decoder rejects any version other than its own. There is no negotiation within a version: a layout change bumps `WIRE_VERSION`.

## Frame types

The type values are the `FRAME` table in `js/wire/frame.js`. Sizes are the body after the two header bytes.

| Value | Name | Body |
|---:|---|---|
| 1 | ORDER | unitId u32, x i32, y i32 |
| 2 | STATE | tick u32, hash u32, count u16 (at most 4096), then count unit records |
| 3 | HELLO | roomLen u8 (at most 64), room UTF-8 |
| 4 | ERROR | code u16, messageLen u16 (at most 256), message UTF-8 |
| 5 | DELTA | tick u32, hash u32, count u16 (at most 4096) unit records, removedCount u16 (at most 4096), removed unit ids u32 each |
| 6 | CAST | unitId u32, slot u8, target i32 (-1 for a point), x i32, y i32 |
| 7 | ATTACK | unitId u32, targetId i32 |
| 8 | ROSTER | you u8, team u8, playerCount u8 (at most 10), then per player: slot u8, team u8, connected u8, gold i32, kills u16, deaths u16 |
| 9 | JOIN | lastTick u32 (0 on a fresh join, the last applied tick on a reconnect) |
| 10 | START | seatCount u8 (at most 10), then per seat: slot u8, ownerLen u8 (at most 64), owner UTF-8, pickLen u8 (at most 32), pick UTF-8 |
| 11 | HOLD | unitId u32 |
| 12 | USE_ITEM | unitId u32, slot u8, target i32 (-1 for none), x i32, y i32 |
| 13 | ATTACK_MOVE | unitId u32, x i32, y i32 |
| 14 | SWAP | tick u32, hash u32, fileLen u8 (at most 64), file name UTF-8 |
| 15 | CLASS_OVERRIDE | unitId u32, class u8 (0..3, or 255 to clear) |
| 16 | HOLD_OVERRIDE | unitId u32, hold u8 (0 never holds, 1 always, 255 clears) |
| 17 | SWAP_NOT_READY | tick u32, hash u32 |
| 18 | SWAP_READY | tick u32, hash u32 |
| 19 | SWAP_CANCEL | tick u32, hash u32 |
| 20 | PACK_FILE | nameLen u16 (at most 256), name UTF-8, hash u32, size u32, offset u32, flags u8, chunkLen u16 (at most 16384), chunk bytes |
| 21 | PACK_WANT | hash u32 |
| 22 | ACTION | nameLen u16 (at most 64), name UTF-8, bodyLen u16 (at most 4096), body (JSON text) |
| 23 | UI | bodyLen u16 (at most 8192), body (JSON text) |

A unit record in STATE and DELTA is 19 bytes: id u32, kind u16, player u8, x i32, y i32, hp i32. The host writes x and y as the unit's tile position times 1000, rounded (`WIRE_SCALE` in `js/client/play.js`).

Notes on the fields:

- **ORDER** is a move order to the whole tile (x, y) a peer sends the host; the host applies it only when the unit belongs to the sender's player. The wasm `Game` converts the tile to Q16.16.
- The host sends one state frame per sim tick: **STATE** (every `FULL_EVERY` = 20 frames, the full unit list) or **DELTA** (the units whose record changed since the previous frame, and the ids that left). The encoder is `js/client/delta.js`.
- **SWAP** announces a data swap at a tick: `hash` is the 32-bit FNV-1a of the new image bytes. **SWAP_READY** and **SWAP_NOT_READY** are a peer's answer for that tick and hash, and **SWAP_CANCEL** is the host dropping it.
- **PACK_FILE** moves a pack file in chunks. `flags` bit 1 (`PACK_FLAG_REF`) marks a reference with no bytes; the peer then asks for the bytes by hash with **PACK_WANT**. Bit 2 (`PACK_FLAG_DELETE`) marks a removal.
- **ACTION** is a joined player's input for the pack's `on_action`; **UI** is a UI event the host's pack emitted, sent to every page. Both bodies are JSON text.
- The start gate (below in ARCHITECTURE.md) rides these two frames. A peer sends ACTION `load_ready` (body `{}`, the page handles it and the pack never sees it) once its heroes are loaded and its GPU is warm, and repeats it every 500 ms until the match is live. The host sends UI `{"kind":"load","data":{"phase":"loading","rows":[{slot,name,stage,ready,reason}]}}` every 500 ms while it waits, and `{"phase":"live"}` when it opens. A STATE or DELTA frame also ends a peer's wait, since the host only ticks once live.

## Reject rules

Decoders never drop a frame silently. Each rejection throws `WireError` with a `code`:

| Code | Condition |
|---|---|
| `UNSUPPORTED_VERSION` | the header version byte is not `WIRE_VERSION` (checked first) |
| `UNKNOWN_TYPE` | the type byte is not in the table above |
| `TRUNCATED` | the frame ends before a field is complete |
| `LENGTH_MISMATCH` | bytes remain after the last field |
| `COUNT_OUT_OF_RANGE` | a count, length or name exceeds its bound above |
| `BAD_UTF8` | a text field is not valid UTF-8 |

Encoders throw `WireError` with code `RANGE` for any value that does not fit its field, and `UNKNOWN_TYPE` for an unknown frame type. Values are never truncated or wrapped.

## Tests

`js/wire/test.js` runs in the `wire` job of `.github/workflows/ci.yml` (`node test.js` in `js/wire`). It round-trips every frame type, checks the reject rules, and imports `js/client/delta.js` and `js/client/swap_protocol.js` to check the state encoder and the swap gate. Its output line is `wire frames ok types=<n> roundtrip=<n>`.
