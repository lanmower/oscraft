# Hosting a game with friends

The game runs in the browser. You host a room from the game page, and friends join the same room from their own browsers. Nothing to install and no server process to run.

## 1. Open the game page

Open the project's GitHub Pages address in a browser:

```
https://<owner>.github.io/<repo>/play.html
```

The room panel at the top right lists the rooms the page has seen announced. The room you are in is shown next to "room".

## 2. Choose a room

- Pick a room from the list, or type a short name (letters, digits, `-` or `_`, up to 64 characters) and press join.
- The page reloads with that room in its address, for example `play.html#<room>`.

## 3. Share the room link

Send your friends the same address with the room name after `#`:

```
https://<owner>.github.io/<repo>/play.html#<room>
```

Each friend opens the link and joins the same room. The page connects peers directly through WebRTC. Nostr relays carry the signalling that sets each connection up.

## 4. When a peer does not connect

- Relays: the page uses its default Nostr relays. To use specific relays, add `relay=<wss-url>` (comma-separated for several) to the address. Both players must use the same relays and the same namespace. The namespace is `ns=<name>` and defaults to `oscraft`.
- ICE (STUN/TURN): with no `relay=` parameter, the connection library chooses its ICE servers, which covers most home and mobile networks. With `relay=`, the page passes no ICE servers, so the two browsers must reach each other directly. If a friend behind a strict NAT or a corporate network cannot connect, open the link without `relay=`.
- Check that both players have the same room name in the address, and that neither has blocked WebRTC or pop-ups for the page.
- Reload the page once; the page retries a dropped connection up to six times with backoff before it gives up.
