const REPLAY_DB = "oscraft-replays";
const REPLAY_STORE = "replays";

function openReplayDb() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(REPLAY_DB, 1);
    request.onupgradeneeded = () => request.result.createObjectStore(REPLAY_STORE, { keyPath: "id" });
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

export async function indexReplay(entry) {
  const db = await openReplayDb();
  try {
    await new Promise((resolve, reject) => {
      const tx = db.transaction(REPLAY_STORE, "readwrite");
      tx.objectStore(REPLAY_STORE).put(entry);
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error);
    });
  } finally {
    db.close();
  }
}

export async function listReplays() {
  const db = await openReplayDb();
  try {
    return await new Promise((resolve, reject) => {
      const request = db.transaction(REPLAY_STORE, "readonly").objectStore(REPLAY_STORE).getAll();
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
  } finally {
    db.close();
  }
}

export function downloadReplay(name, text) {
  const url = URL.createObjectURL(new Blob([text], { type: "text/plain" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  document.body.appendChild(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export async function saveReplayFile(text, meta) {
  const savedAt = Date.now();
  const stamp = new Date(savedAt).toISOString().replace(/[:.]/g, "-");
  const name = `oscraft-replay-${stamp}.replay`;
  downloadReplay(name, text);
  await indexReplay({
    id: `${savedAt}-${Math.random().toString(16).slice(2, 10)}`,
    name,
    savedAt,
    tick: meta.tick,
    bytes: text.length,
    namespace: meta.namespace,
    room: meta.room,
  });
}
