# Hosting a game with friends

This guide takes you from the release archive to a running game that friends can open in a browser.

## 1. Install the release archive

Download `oscraft-<version>.tar.gz` from the project's release page and unpack it into an empty directory:

```
mkdir oscraft && tar -C oscraft -xzf oscraft-<version>.tar.gz
cd oscraft
```

The archive holds `bin/oscraftd` (the host), `bin/oscraft-lsp` (the language server), `data/` (the game data) and `site/` (the browser page). `MANIFEST.sha256` lists every file with its checksum.

## 2. Start the host

```
./bin/oscraftd --addr 0.0.0.0:8080 --room <name> --data data --site site
```

- `--room <name>`: the room friends join. Give it a short name. `--room` may be repeated to host several rooms.
- `--data data`: the game data directory from the archive.
- `--site site`: the browser page directory from the archive.
- `--addr 0.0.0.0:8080`: listen on all network interfaces, so other machines can reach the host. Use `127.0.0.1:8080` to keep it on your own machine.

Leave the host running while you play. Stop it with Ctrl+C.

## 3. Open the page and share the link

Open `http://localhost:8080/site/` in a browser on the host machine. Friends open `http://<host-address>:8080/site/`, where `<host-address>` is your machine's LAN address or the public address you forwarded. Send them that address and the room name you passed to `--room`.

## 4. When a peer does not connect

- Check that the address and port are reachable from the friend's network. A friend on another network needs a port forward on your router, or a VPN that both of you join.
- Check that the host is still running and that its log shows the room. A page that cannot reach `/state` on the host address is a network problem, not a game problem.
- The archive host serves the page and the game state over HTTP. It has no relay or ICE (STUN/TURN) settings. Peer-to-peer relay and ICE belong to the Node host in the source repository (`js/host`), which is not part of the release archive yet. Until it is, the archive host connects peers only through the address they can reach.
