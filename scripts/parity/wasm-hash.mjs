import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join, resolve } from 'node:path';

const [pkgDir, dataDir, ticksArg, nativeLine] = process.argv.slice(2);
if (!nativeLine) {
  console.error('usage: wasm-hash.mjs <pkg-dir> <data-dir> <ticks> <native-hash-line>');
  process.exit(2);
}
const ticks = Number(ticksArg);
const { Game } = createRequire(import.meta.url)(resolve(pkgDir, 'web.js'));

const game = new Game(new Uint8Array(readFileSync(join(dataDir, 'image.osdt'))));
const rulesError = game.set_attr_rules(readFileSync(join(dataDir, 'rules.txt'), 'utf8'));
if (rulesError) throw new Error(`set_attr_rules: ${rulesError}`);
const loadError = game.load_mods(readFileSync(join(dataDir, 'mods.json'), 'utf8'));
if (loadError) throw new Error(`load_mods: ${loadError}`);
for (let tick = 1; tick <= ticks; tick++) {
  const error = game.tick();
  if (error) throw new Error(`tick ${tick}: ${error}`);
}

const wasmLine = `state_hash=${game.state_hash().toString(16).padStart(16, '0')} ticks=${ticks}`;
console.log(`native: ${nativeLine}`);
console.log(`wasm:   ${wasmLine}`);
if (wasmLine !== nativeLine.trim()) {
  console.error('state hash differs between the native build and the built wasm');
  process.exit(1);
}
