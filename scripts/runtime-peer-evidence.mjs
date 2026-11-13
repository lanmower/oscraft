import fs from "node:fs";
import path from "node:path";

export function extractionEvidence(paths) {
  const sources = [
    ["wscheppity", path.join(paths.wscheppityRoot, ".local", "extraction-manifest.json")],
    ["extra", path.join(paths.extraRoot, ".local", "extraction-manifest.json")],
    ["extraCdnManifest", path.join(paths.extraRoot, ".local", "cdn", "manifest-all.json")],
  ];
  return sources.map(([peer, file]) => {
    if (!fs.existsSync(file)) return { peer, path: file.replace(/^\/workspace\//, ""), present: false, informational: true };
    let json = null;
    try { json = JSON.parse(fs.readFileSync(file, "utf8")); } catch {}
    const summary = json?.summary ?? json?.counts ?? {};
    return {
      peer,
      path: file.replace(/^\/workspace\//, ""),
      present: true,
      classes: summary.classes ?? null,
      complete: summary.complete ?? null,
      blocked: summary.blocked ?? null,
      informational: true,
    };
  });
}
