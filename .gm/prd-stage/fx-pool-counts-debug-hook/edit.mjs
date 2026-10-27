import fs from 'node:fs';
const dir = '/workspace/oscraft/.gm/prd-stage/fx-pool-counts-debug-hook';
function rep(file, anchor, insert) {
  const src = fs.readFileSync(file, 'utf8');
  const n = src.split(anchor).length - 1;
  if (n !== 1) throw new Error(file + ': anchor matched ' + n + ' times');
  fs.writeFileSync(file, src.replace(anchor, anchor + insert));
  console.log('EDITED', file, '+' + insert.length + ' chars');
}
rep(dir + '/debug_api.js', '  "oscraft.debug.effects()         the last 64 display effects the page drained from the sim (read-only copy)",\n', '  "oscraft.debug.fx()              live pool counts per effect kind (instances, pool sizes, cost); read-only copy",\n');
rep(dir + '/debug_api.js', '    effects: () => hooks.effects(),\n', '    fx: () => hooks.fxCounts(),\n');
rep(dir + '/play.js', '  effects: () => (fx ? fx.recentEffects() : null),\n', '  fxCounts: () => (fx ? fx.counts() : null),\n');
