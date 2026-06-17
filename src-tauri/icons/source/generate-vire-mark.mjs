// Generates the TEMPORARY placeholder Vire app icon (src-tauri/icons/source/vire-icon.png).
// This is a stand-in mark, not the branded asset — brand owns the final icon (artifacts/brand/).
// To replace: drop a branded >=1024x1024 PNG at src-tauri/icons/source/vire-icon.png and re-run
// `npx tauri icon src-tauri/icons/source/vire-icon.png`, then rebuild. No code change required.
//
// SAFE AREA (TASK-027 E3): the mark must occupy ~80% of a transparent 1024x1024 canvas (≈10% margin
// per side) so macOS renders it at Dock parity with other apps. The branded replacement asset MUST
// keep this same ~80% safe area — a full-bleed PNG will render oversized in the Dock.
//
// Dependency-free: encodes a PNG with Node's built-in zlib so it runs anywhere Node is present
// (no ImageMagick/rsvg/PIL needed). Run: `node src-tauri/icons/source/generate-vire-mark.mjs`.

import { deflateSync } from 'node:zlib';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const N = 1024;
const BG = [79, 70, 229]; // indigo (#4F46E5)
const FG = [255, 255, 255]; // white mark
const AA = 1.5; // anti-alias edge width in px

// Safe-area inset (TASK-027 E3): the rounded-square mark occupies ~80% of the canvas with a
// transparent margin so macOS renders it at Dock parity with other apps instead of oversized. The
// branded asset that replaces this placeholder MUST keep the same ~80% safe area (see icons README).
const SAFE = 0.8;
const BOX = SAFE * N; // content square side (~819px)
const ORIGIN = (N - BOX) / 2; // top-left of the inset content square (~102px)
const CENTER = N / 2;
const CORNER_R = 0.215 * BOX; // rounded-square corner radius, relative to the inset box

// V geometry as fractions of the inset content box (not the full canvas)
const frac = (f) => ORIGIN + f * BOX;
const ty = frac(0.3);
const by = frac(0.72);
const lx = frac(0.3);
const rx = frac(0.7);
const apex = [frac(0.5), by];
const strokeHalf = 0.058 * BOX;

function smoothEdge(d, edge) {
  // returns coverage 1 inside (d<0), 0 outside, linear ramp over `edge` px
  const t = 0.5 - d / edge;
  return Math.max(0, Math.min(1, t));
}

function distToSegment(px, py, ax, ay, bx, by) {
  const dx = bx - ax;
  const dy = by - ay;
  const len2 = dx * dx + dy * dy;
  let t = len2 === 0 ? 0 : ((px - ax) * dx + (py - ay) * dy) / len2;
  t = Math.max(0, Math.min(1, t));
  const cx = ax + t * dx;
  const cy = ay + t * dy;
  return Math.hypot(px - cx, py - cy);
}

function roundedRectSDF(px, py) {
  // signed distance to the inset rounded square (~80% of canvas, centered); <0 inside
  const half = BOX / 2;
  const qx = Math.abs(px - CENTER) - (half - CORNER_R);
  const qy = Math.abs(py - CENTER) - (half - CORNER_R);
  const ax = Math.max(qx, 0);
  const ay = Math.max(qy, 0);
  const outside = Math.hypot(ax, ay);
  const inside = Math.min(Math.max(qx, qy), 0);
  return outside + inside - CORNER_R;
}

function blend(dst, src, a) {
  for (let i = 0; i < 3; i++) dst[i] = Math.round(src[i] * a + dst[i] * (1 - a));
  dst[3] = Math.round(255 * Math.max(a, dst[3] / 255));
}

const raw = Buffer.alloc(N * (N * 4 + 1)); // +1 filter byte per row

for (let y = 0; y < N; y++) {
  const rowStart = y * (N * 4 + 1);
  raw[rowStart] = 0; // filter type: none
  for (let x = 0; x < N; x++) {
    const px = x + 0.5;
    const py = y + 0.5;
    const px4 = rowStart + 1 + x * 4;

    const bgCov = smoothEdge(roundedRectSDF(px, py), AA);
    const pixel = [0, 0, 0, 0];
    if (bgCov > 0) blend(pixel, BG, bgCov);

    const dV = Math.min(
      distToSegment(px, py, lx, ty, apex[0], apex[1]),
      distToSegment(px, py, rx, ty, apex[0], apex[1]),
    );
    const fgCov = smoothEdge(dV - strokeHalf, AA) * bgCov;
    if (fgCov > 0) blend(pixel, FG, fgCov);

    raw[px4] = pixel[0];
    raw[px4 + 1] = pixel[1];
    raw[px4 + 2] = pixel[2];
    raw[px4 + 3] = pixel[3];
  }
}

// ---- minimal PNG encoder ----
const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, 'ascii');
  const body = Buffer.concat([typeBuf, data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body), 0);
  return Buffer.concat([len, body, crc]);
}

const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(N, 0);
ihdr.writeUInt32BE(N, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type RGBA
ihdr[10] = 0; // compression
ihdr[11] = 0; // filter
ihdr[12] = 0; // interlace
const idat = deflateSync(raw, { level: 9 });

const png = Buffer.concat([
  sig,
  chunk('IHDR', ihdr),
  chunk('IDAT', idat),
  chunk('IEND', Buffer.alloc(0)),
]);

const out = join(dirname(fileURLToPath(import.meta.url)), 'vire-icon.png');
writeFileSync(out, png);
console.log(`wrote ${out} (${N}x${N}, ${png.length} bytes)`);
