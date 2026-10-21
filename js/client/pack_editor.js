// The pack editor: a file tree, a text editor for .lua .osd .json .txt .tsv, a drop zone for asset files, Save and a
// download link. Opened from the host's menu. Only the host may edit (canEdit); a peer sees the same pack read-only.
// Save hands the changed files to onSave(changes) (see play.js, which sends them to every peer as PACK_FILE frames).
import { isText, normalizePath, readFileList, readZip, refusedPath } from "./pack_load.js";
import { Pack, writeZip } from "./pack.js";

// Builds the editor inside `root` for `pack` (a Pack). Returns { setPack, close }.
export function createPackEditor({ root, pack, canEdit = false, onSave = () => {}, onLoad = () => {} }) {
  let current = pack;
  let selected = null;
  const doc = root.ownerDocument;
  const el = (tag, props = {}, kids = []) => {
    const node = Object.assign(doc.createElement(tag), props);
    for (const k of kids) node.append(k);
    return node;
  };

  const tree = el("ul", { className: "pack-tree" });
  const editor = el("textarea", { className: "pack-text", spellcheck: false });
  const status = el("div", { className: "pack-status" });
  const dropZone = el("div", { className: "pack-drop", textContent: "drop asset files here" });
  const save = el("button", { type: "button", textContent: "save", disabled: !canEdit });
  const download = el("a", { textContent: "download pack (.zip)", download: "pack.zip", href: "#" });
  const fileInput = el("input", { type: "file", multiple: true, accept: ".zip" });
  const folderInput = el("input", { type: "file", webkitdirectory: true });
  const panel = el("div", { className: "pack-editor", hidden: true }, [
    el("div", { className: "pack-bar" }, [fileInput, folderInput, save, download]),
    el("div", { className: "pack-body" }, [el("div", { className: "pack-side" }, [tree, dropZone]), editor]),
    status,
  ]);
  root.append(panel);

  function setStatus(text) { status.textContent = text; }

  function refreshTree() {
    tree.replaceChildren();
    for (const path of current.paths()) {
      const li = el("li", { textContent: path, className: path === selected ? "sel" : "" });
      li.addEventListener("click", () => select(path));
      tree.append(li);
    }
  }

  function select(path) {
    selected = path;
    const value = current.get(path);
    editor.value = isText(path) ? value : `(binary asset, ${value.length} bytes)`;
    editor.readOnly = !canEdit || !isText(path);
    refreshTree();
  }

  function applyEdit() {
    if (!canEdit || selected === null || !isText(selected)) return;
    if (editor.value !== current.get(selected)) current.set(selected, editor.value);
  }

  function addAssets(list) {
    const added = [];
    const refused = [];
    return Promise.all([...list].map(async (file) => {
      const path = file.name;
      try {
        const clean = normalizePath(path);
        if (refusedPath(clean)) { refused.push(clean); return; }
        const bytes = new Uint8Array(await file.arrayBuffer());
        current.set(clean, isText(clean) ? new TextDecoder().decode(bytes) : bytes);
        added.push(clean);
      } catch (err) { refused.push(String(err.message)); }
    })).then(() => {
      refreshTree();
      setStatus(`added ${added.length}${refused.length ? `, refused ${refused.join("; ")}` : ""}`);
    });
  }

  async function loadFrom(promise) {
    try {
      const { files, skipped } = await promise;
      current = new Pack(files);
      current.readManifest();
      selected = null;
      editor.value = "";
      refreshTree();
      setStatus(`loaded ${files.size} files${skipped.length ? `, skipped ${skipped.join(", ")}` : ""}`);
      onLoad(current);
    } catch (err) {
      setStatus(`load failed: ${err.message}`);
    }
  }

  editor.addEventListener("input", applyEdit);
  editor.addEventListener("change", applyEdit);
  save.addEventListener("click", () => {
    applyEdit();
    const changes = current.changed();
    if (!changes.length) { setStatus("nothing to save"); return; }
    try {
      onSave(changes, current);
      current.markSaved();
      setStatus(`saved ${changes.length} file(s)`);
    } catch (err) {
      setStatus(`save failed: ${err.message}`);
    }
  });
  fileInput.addEventListener("change", () => {
    const file = fileInput.files[0];
    if (file) loadFrom(file.arrayBuffer().then((buf) => readZip(buf)));
  });
  folderInput.addEventListener("change", () => loadFrom(readFileList(folderInput.files)));
  dropZone.addEventListener("dragover", (e) => e.preventDefault());
  dropZone.addEventListener("drop", (e) => {
    e.preventDefault();
    if (canEdit) addAssets(e.dataTransfer.files);
  });
  download.addEventListener("click", (e) => {
    e.preventDefault();
    const bytes = writeZip(new Map(current.paths().map((p) => [p, current.get(p)])));
    const url = URL.createObjectURL(new Blob([bytes], { type: "application/zip" }));
    const a = el("a", { href: url, download: "pack.zip" });
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  });

  refreshTree();
  return {
    setPack(next) { current = next; selected = null; editor.value = ""; refreshTree(); },
    open() { panel.hidden = false; },
    dispose() { panel.remove(); },
    close() { panel.hidden = true; },
    get pack() { return current; },
  };
}
