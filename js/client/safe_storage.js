const store = () => {
  try { return globalThis.localStorage ?? null; } catch { return null; }
};

export const readItem = (key) => {
  try { return store()?.getItem(key) ?? null; } catch { return null; }
};

export const writeItem = (key, value) => {
  try { store()?.setItem(key, value); return null; } catch (err) { return err; }
};

export const removeItem = (key) => {
  try { store()?.removeItem(key); return null; } catch (err) { return err; }
};
