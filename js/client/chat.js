const WC3_DEFAULT_DISTANCE = 1650;
const WC3_MIN_DISTANCE = 1500;
const WC3_MAX_DISTANCE = 3000;
const MULTIPLIER_MAX = 3;

export function parseChat(text) {
  const line = text.trim();
  if (!line.startsWith("-")) return { text: line };
  const space = line.indexOf(" ");
  const cmd = (space < 0 ? line : line.slice(0, space)).toLowerCase();
  const rest = space < 0 ? "" : line.slice(space + 1).trim();
  const n = Math.trunc(Number.parseFloat(rest));
  return { text: line, cmd, rest, n: Number.isFinite(n) ? n : 0 };
}

export function createChat({ send, notify, camera, clearLog }) {
  const box = document.createElement("input");
  box.dataset.ui = "chat";
  box.maxLength = 200;
  box.style.cssText = "position:fixed;left:8px;bottom:150px;width:min(40vw,420px);z-index:5;display:none;font:14px system-ui,sans-serif;padding:3px 8px";
  document.body.appendChild(box);

  function setDistance(wc3) {
    camera.set((camera.defaultDistance() * wc3) / WC3_DEFAULT_DISTANCE);
  }

  function camCommand(c) {
    if (c.text.length === 4) {
      notify(`Current camera hight ${Math.round((WC3_DEFAULT_DISTANCE * camera.get()) / camera.defaultDistance())}`);
      return;
    }
    const r = Number.parseFloat(c.rest);
    if (!(r > 0)) {
      setDistance(WC3_DEFAULT_DISTANCE);
      notify("Incorrect multiplier, camera changed to default.");
    } else if (r < MULTIPLIER_MAX) {
      setDistance(WC3_DEFAULT_DISTANCE * r);
    } else if (r >= WC3_MIN_DISTANCE && r <= WC3_MAX_DISTANCE) {
      setDistance(r);
    } else {
      notify("Incorrect camera distance.");
    }
  }

  function submit() {
    const c = parseChat(box.value);
    box.value = "";
    close();
    if (c.text === "") return;
    if (c.cmd === "-cam") return camCommand(c);
    if (c.cmd === "-clear" || c.cmd === "-c") return clearLog();
    send("chat", { text: c.text, ...(c.cmd ? { cmd: c.cmd, n: c.n } : {}) });
  }

  function open() {
    box.style.display = "block";
    box.focus();
  }

  function close() {
    box.style.display = "none";
    box.blur();
  }

  box.addEventListener("keydown", (e) => {
    e.stopPropagation();
    if (e.key === "Enter") { e.preventDefault(); submit(); } else if (e.key === "Escape") { box.value = ""; close(); }
  });
  function onKey(e) {
    if (e.key !== "Enter" || e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    e.preventDefault();
    e.stopPropagation();
    open();
  }
  window.addEventListener("keydown", onKey, true);

  return {
    destroy() {
      window.removeEventListener("keydown", onKey, true);
      box.remove();
    },
  };
}
