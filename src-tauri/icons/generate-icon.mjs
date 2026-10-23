/**
 * App-icon placeholder generator.
 *
 * `tauri-build` requires a Windows `.ico` resource, so the repository needs one file
 * before any Rust gate can run. The final artwork is a design deliverable; this script
 * only produces a flat, single-colour square so the build has a valid resource.
 *
 * Node core only: zlib plus a CRC table, no dependency. Run `npm run icons`, which then
 * calls `tauri icon` to emit every size the bundler expects.
 */
import { deflateSync } from 'node:zlib';
import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SIZE = 1024;
/** `--primary` from DESIGN.md §2.1, converted to sRGB for a raster asset. */
const RGB = [43, 43, 43];

const crcTable = Array.from({ length: 256 }, (_, index) => {
  let value = index;
  for (let bit = 0; bit < 8; bit += 1) {
    value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1;
  }
  return value >>> 0;
});

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) {
    crc = crcTable[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([length, body, crc]);
}

const header = Buffer.alloc(13);
header.writeUInt32BE(SIZE, 0);
header.writeUInt32BE(SIZE, 4);
header[8] = 8; // bit depth
header[9] = 6; // RGBA
header[10] = 0;
header[11] = 0;
header[12] = 0;

const scanline = Buffer.alloc(1 + SIZE * 4);
for (let x = 0; x < SIZE; x += 1) {
  const offset = 1 + x * 4;
  scanline[offset] = RGB[0];
  scanline[offset + 1] = RGB[1];
  scanline[offset + 2] = RGB[2];
  scanline[offset + 3] = 255;
}
const raw = Buffer.concat(Array.from({ length: SIZE }, () => scanline));

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk('IHDR', header),
  chunk('IDAT', deflateSync(raw, { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
]);

const here = dirname(fileURLToPath(import.meta.url));
const target = join(here, 'app-icon.png');
writeFileSync(target, png);
console.log(`wrote ${target}`);
