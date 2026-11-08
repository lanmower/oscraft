export const LUA_BUNDLE = "bundle/moba.modules.json";

export function moduleTextReader(bundle, readText) {
  return (rel) => (bundle && Object.hasOwn(bundle, rel) ? Promise.resolve(bundle[rel]) : readText(rel));
}

export async function moduleClosure(manifest, readText) {
  const byStem = manifest.modules ?? {};
  const out = [];
  const seen = new Set();
  const reads = new Map();
  const read = (file) => {
    if (!reads.has(file)) {
      reads.set(file, readText(file).then((text) => {
        for (const [, stem] of text.matchAll(/require\("([^"]+)"\)/g)) if (byStem[stem] !== undefined) read(byStem[stem]);
        return text;
      }));
    }
    return reads.get(file);
  };
  for (const root of manifest.mods ?? []) read(root);
  const visit = async (file) => {
    if (seen.has(file)) return;
    seen.add(file);
    const text = await read(file);
    out.push({ name: file, text });
    for (const [, stem] of text.matchAll(/require\("([^"]+)"\)/g)) {
      if (byStem[stem] !== undefined) await visit(byStem[stem]);
    }
  };
  for (const root of manifest.mods ?? []) await visit(root);
  return out;
}
