import fs from "node:fs";
import zlib from "node:zlib";

const [dir, out, factorArg] = process.argv.slice(2);
if (!dir || !out) {
  console.error("usage: contact-sheet.mjs <frames-dir> <out.png> [factor]");
  process.exit(2);
}
const factor = Number(factorArg ?? 4);
const COLUMNS = 7;
const GAP = 4;
const names = fs.readdirSync(dir).filter((f) => /^tick-\d{4}\.png$/.test(f)).sort();
if (names.length === 0) {
  console.error(`contact-sheet: no tick-NNNN.png in ${dir}`);
  process.exit(1);
}
const CHANNELS = { 2: 3, 6: 4 };

function decode(file) {
  const buf = fs.readFileSync(file);
  let p = 8;
  let w = 0;
  let h = 0;
  let ch = 0;
  const idat = [];
  while (p < buf.length) {
    const len = buf.readUInt32BE(p);
    const type = buf.toString("latin1", p + 4, p + 8);
    const data = buf.subarray(p + 8, p + 8 + len);
    if (type === "IHDR") {
      w = data.readUInt32BE(0);
      h = data.readUInt32BE(4);
      if (data[8] !== 8 || CHANNELS[data[9]] === undefined) throw new Error(`${file}: unsupported PNG depth/colour`);
      ch = CHANNELS[data[9]];
    }
    if (type === "IDAT") idat.push(data);
    p += 12 + len;
  }
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const stride = w * ch;
  const rows = [];
  let prev = Buffer.alloc(stride);
  for (let y = 0; y < h; y++) {
    const filter = raw[y * (stride + 1)];
    const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    const cur = Buffer.alloc(stride);
    for (let i = 0; i < stride; i++) {
      const a = i >= ch ? cur[i - ch] : 0;
      const b = prev[i];
      const c = i >= ch ? prev[i - ch] : 0;
      let v = line[i];
      if (filter === 1) v += a;
      else if (filter === 2) v += b;
      else if (filter === 3) v += (a + b) >> 1;
      else if (filter === 4) {
        const pp = a + b - c;
        const pa = Math.abs(pp - a);
        const pb = Math.abs(pp - b);
        const pc = Math.abs(pp - c);
        v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      }
      cur[i] = v & 255;
    }
    rows.push(cur);
    prev = cur;
  }
  return { w, h, ch, rows };
}

const frames = names.map((n) => decode(`${dir}/${n}`));
const { w, h, ch } = frames[0];
const tw = Math.floor(w / factor);
const th = Math.floor(h / factor);
const cols = Math.min(COLUMNS, frames.length);
const rowsN = Math.ceil(frames.length / cols);
const sw = cols * tw + (cols - 1) * GAP;
const sh = rowsN * th + (rowsN - 1) * GAP;
const pixels = Buffer.alloc(sw * sh * 3, 40);

frames.forEach((frame, index) => {
  const ox = (index % cols) * (tw + GAP);
  const oy = Math.floor(index / cols) * (th + GAP);
  for (let ty = 0; ty < th; ty++) {
    for (let tx = 0; tx < tw; tx++) {
      const sum = [0, 0, 0];
      for (let dy = 0; dy < factor; dy++) {
        const row = frame.rows[ty * factor + dy];
        for (let dx = 0; dx < factor; dx++) {
          const base = (tx * factor + dx) * ch;
          for (let k = 0; k < 3; k++) sum[k] += row[base + k];
        }
      }
      const n = factor * factor;
      const o = ((oy + ty) * sw + (ox + tx)) * 3;
      for (let k = 0; k < 3; k++) pixels[o + k] = Math.round(sum[k] / n);
    }
  }
});

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "latin1"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(zlib.crc32(body) >>> 0);
  return Buffer.concat([len, body, crc]);
}

const header = Buffer.alloc(13);
header.writeUInt32BE(sw, 0);
header.writeUInt32BE(sh, 4);
header[8] = 8;
header[9] = 2;
header[10] = 0;
header[11] = 0;
header[12] = 0;
const scan = Buffer.alloc((sw * 3 + 1) * sh);
for (let y = 0; y < sh; y++) {
  scan[y * (sw * 3 + 1)] = 0;
  pixels.copy(scan, y * (sw * 3 + 1) + 1, y * sw * 3, (y + 1) * sw * 3);
}
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", header),
  chunk("IDAT", zlib.deflateSync(scan)),
  chunk("IEND", Buffer.alloc(0)),
]);
fs.writeFileSync(out, png);
console.log(`contact-sheet: ${frames.length} frames, ${sw}x${sh}, ${out}`);
