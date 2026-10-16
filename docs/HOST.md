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

The host opens one guest instance at start and keeps it for the life of the process:

- The guest runs in a child node process (`process.execPath -e`) that calls `node:wasi` `start()`. Its stdin and stdout are anonymous pipes created by `spawn`, which behave the same on Linux, macOS and Windows, so no FIFO or named pipe is needed.
- The host writes the u32 LE image length and the image bytes once, then one command per line. The child fd 0 blocks in read until the next line arrives, so the instance is never restarted.
- Each command gets one reply line on stdout, in command order. The host pipelines a batch of commands and resolves them in order.
- Each tick runs `tick 1`, `state-hash` and `unit-count`, then `unit-position`, `unit-player` and `unit-hp` for each unit, all in one pipelined batch (`readTick` in `js/host/guest.js`). A player order arriving on a peer channel is sent as `order <unit> <x> <y>`. Tick cost depends on the sim state, not on the number of past commands: there is no journal and no replay.

The guest protocol is unchanged (`rust/crates/wasip1/src/lib.rs`). The host runs on Linux, macOS and Windows.

## Frames

Peer frames use the wire codec in `js/wire/frame.js`, imported by `server.js`; the local `js/host/frame.js` is gone. An order frame carries `unitId`, `x` and `y`, and the host turns it into an `order` command. A state frame carries `tick`, the low 32 bits of the guest state hash, and one unit record `{id, kind, player, x, y, hp}` per unit, read from the guest each tick. `kind` is the catalog unit type index from the guest command `unit-kind` (the id `catalog-unit-name` takes), or -1 when the unit has none.

## Orders

Each peer gets a player id when its channel opens (`assignPlayer` in `js/host/server.js`): the lowest id no other peer holds, up to 256. An order frame is applied by `applyOrder` only when the unit's owner, read with the guest command `unit-player <id>`, equals the sending peer's player. Otherwise the host prints `ev=order_refused reason=not_owner peer=<first 8 chars> unit=<id>` and sends nothing to the guest. The `order-peer-ownership` job in `.github/workflows/ci.yml` runs the gate against a real guest and prints `order ownership ok refused=1 moved=false`.

## CI

The `net-host-persistent` job in `.github/workflows/ci.yml` runs on ubuntu-latest, ubuntu-24.04-arm, macos-latest and windows-latest. Each builds the wasip1 guest and the OSDT image, opens one guest through `js/host/guest.js`, sends one order, and runs 50 ticks with `readTick`. It asserts that the tick count advanced by 50 and that the last 10 tick times sum to at most 3 times the first 10. It prints `net host persistent ok ticks=50 ratio=<r>`.

## Signalling

`createDataSession({ fsm, xstate, relayPool, auth, namespace: 'oscraft', createPeerConnection })` uses node-datachannel through the polyfill, following the wireweave README. Auth storage is in memory.
