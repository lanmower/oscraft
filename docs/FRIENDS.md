# Hosting a game with friends

This guide takes you from the release archive to a game your friends can join from a browser.

## 1. Install the release archive

Download `oscraft-<version>.tar.gz` from the project's release page and unpack it into an empty directory:

```
mkdir oscraft && tar -C oscraft -xzf oscraft-<version>.tar.gz
cd oscraft
```

The archive holds `bin/oscraftd` (the host), `bin/oscraft-lsp` (the language server), `data/` (the game data), `site/` (the browser page), `js/host/` and `js/wire/` (the Node host), `js/client/` (the peer client page) and `MANIFEST.sha256`, which lists every file with its checksum.

## 2. Start the host with a room name

Option A, the archive host serves the page itself:

```
./bin/oscraftd --addr 0.0.0.0:8080 --room <name> --data data --site site
```

- `--room <name>`: the room friends join. Choose a short name. `--room` may be repeated to host several rooms.
- `--data data`: the game data directory from the archive.
- `--site site`: the browser page directory from the archive.
- `--addr 0.0.0.0:8080`: listen on all interfaces so other machines can reach it. Use `127.0.0.1:8080` to keep it on your own machine.

Option B, the peer-to-peer host over a relay. It needs two files the archive does not ship: the wasip1 guest `oscraft_wasip1.wasm` and an OSDT image, both built from the source repository (`rust/crates/wasip1` and `rust/crates/data`). Install the host's dependencies once, then start it with the room name:

```
(cd js/host && npm install)
node js/host/server.js --wasm <oscraft_wasip1.wasm> --data <image.osdt or its directory> --room <name>
```

Its flags and environment variables are in `docs/HOST.md`.

## 3. Open the page and share the room link

- Option A: friends open `http://<host-address>:8080/site/`, where `<host-address>` is your LAN address or the public address you forwarded. Share that address and the room name from `--room`.
- Option B: serve `js/client/` with any static server, for example `python3 -m http.server 8090 --directory js/client`, and share `http://<host-address>:8090/play.html#<room>`. The page takes the room from the part after `#`. Add `?relay=<wss-url>` to use one relay instead of the defaults.

## 4. When a peer does not connect

- Check that the address and port are reachable from the friend's network. A friend on another network needs a port forward on your router, or a VPN both of you join.
- Check that the host is still running. For Option A, a page that cannot load `/state` on the host address is a network problem, not a game problem.
- Relay: the host and the page must use the same relay URL and namespace. `OSCRAFT_RELAYS` and `OSCRAFT_NAMESPACE` set them on the host; `?relay=` and `?ns=` set them on the page. A mismatch means the two never meet.
- ICE (STUN/TURN): with no `?relay=`, the page lets the connection library choose its ICE servers. With `?relay=`, the page passes no ICE servers, so the two peers must reach each other directly. If a friend behind a strict NAT cannot connect, open the page without `?relay=`.
