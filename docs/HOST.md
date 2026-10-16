# Net host

`js/host/server.js` runs the oscraft wasip1 guest (`rust/crates/wasip1`, built as `oscraft_wasip1.wasm`) under Node's built-in `node:wasi` and bridges it to peers over wireweave `DataSession`.

## Run

```
node js/host/server.js --wasm <oscraft_wasip1.wasm> --data <image file or dir> --room <name> [--every <ticks>] [--tick-ms <ms>]
```

- `--data`: the OSDT image bytes. A directory is read from `image.osdt` inside it.
- `--room`: defaults to `OSCRAFT_ROOM`, then `lobby`.
- `--every`: broadcast a state frame every N ticks (default 1).
- `--tick-ms`: tick interval (default 100).
- `--help`: prints the usage line and exits 0 without loading dependencies.

Dependencies are pinned in `js/host/package.json` (wireweave by commit sha, the rest by exact version). Run `npm install` in `js/host` before starting the host.

## Events

One line per event on stdout: `ev=host_started`, `ev=peer_open peer=<pubkey>`, `ev=peer_close peer=<pubkey>`, `ev=tick n=<tick> units=<count>`. SIGINT or SIGTERM stops the tick loop and exits 0.

## Guest protocol

The guest protocol is unchanged. Each run gets stdin as a u32 LE image length, the image bytes, then text commands until EOF. The guest is one-shot per run, so the host keeps a command journal of player orders and `tick 1` entries and replays it into a fresh instance each tick. The replay cost grows with the journal; a snapshot-based guest run is the planned replacement.

## Frames

`js/host/frame.js` holds a local codec: an order frame is 13 bytes (kind 1, unit, x, y) and a state frame is kind 2 with tick, unit count and hash. The wire lane (`js/wire/`) will replace this file.

## Signalling

`createDataSession({ fsm, xstate, relayPool, auth, namespace: 'oscraft', createPeerConnection })` uses node-datachannel through the polyfill, following the wireweave README. Auth storage is in memory.
