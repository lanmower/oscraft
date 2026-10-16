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

One line per event on stdout: `ev=host_started`, `ev=peer_open peer=<pubkey>`, `ev=peer_close peer=<pubkey>`, `ev=tick n=<tick> units=<count>`, `ev=tick_failed n=<tick> error=<text>`, `ev=order_failed error=<text>`. SIGINT or SIGTERM stops the tick loop, closes the guest and exits 0.

## Persistence

The host opens one guest instance at start and keeps it for the life of the process. The image is written once, then commands go in one line at a time:

- Stdin is a POSIX FIFO created in a temp directory. The host writes the u32 LE image length, the image bytes, and then one command per line. The FIFO stays open, so the guest never sees EOF while the host runs.
- Stdout is a second FIFO. The guest writes one reply line per command, and the host reads replies in command order.
- The guest runs on a worker thread, because `node:wasi` `start()` blocks on its stdin read.
- Each tick sends `tick 1`, `unit-count` and `state-hash`. A player order arriving on a peer channel is sent as `order <unit> <x> <y>` at once. Tick cost depends on the sim state, not on the number of past commands, so there is no journal and no replay.

The guest protocol is unchanged (`rust/crates/wasip1/src/lib.rs`). Windows is not supported yet: the transport needs POSIX FIFOs, and `openGuest` throws on `win32`.

## Frames

Peer frames use the wire codec in `js/wire/frame.js`, imported by `server.js` (the local `js/host/frame.js` is gone). An order frame carries `unitId`, `x` and `y`; the host turns it into an `order` command. The state frame carries `tick` and the low 32 bits of the guest state hash. It sends `units: []`: the wire state frame lists full unit records, and the host does not yet query them each tick, so the unit count is reported on stdout only.

## CI

The `net-host-persistent` job in `.github/workflows/ci.yml` builds the wasip1 guest and the OSDT image, opens one guest through `js/host/guest.js`, sends 50 `tick 1` commands and one order, and asserts that the tick count advanced by 50 and that the mean of the last 10 tick times is at most 3 times the first 10. It prints `net host persistent ok ticks=50 ratio=<r>`.

## Signalling

`createDataSession({ fsm, xstate, relayPool, auth, namespace: 'oscraft', createPeerConnection })` uses node-datachannel through the polyfill, following the wireweave README. Auth storage is in memory.
