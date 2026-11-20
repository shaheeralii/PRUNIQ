import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";

const iconsDir = path.resolve("src-tauri", "icons");
fs.mkdirSync(iconsDir, { recursive: true });

function crc32(buf) {
  let crc = 0xffffffff;
  for (const byte of buf) {
    crc ^= byte;
    for (let i = 0; i < 8; i++) {
      crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function pngChunk(type, data) {
  const typeBuf = Buffer.from(type, "ascii");
  const lenBuf = Buffer.alloc(4);
  lenBuf.writeUInt32BE(data.length, 0);
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([lenBuf, typeBuf, data, crcBuf]);
}

function createSolidPng(size, r, g, b, a) {
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;

  const stride = 1 + size * 4;
  const raw = Buffer.alloc(stride * size);
  for (let y = 0; y < size; y++) {
    const rowStart = y * stride;
    raw[rowStart] = 0;
    for (let x = 0; x < size; x++) {
      const px = rowStart + 1 + x * 4;
      raw[px] = r;
      raw[px + 1] = g;
      raw[px + 2] = b;
      raw[px + 3] = a;
    }
  }

  const idat = zlib.deflateSync(raw);
  return Buffer.concat([
    signature,
    pngChunk("IHDR", ihdr),
    pngChunk("IDAT", idat),
    pngChunk("IEND", Buffer.alloc(0)),
  ]);
}

function createSolidIco32(r, g, b, a) {
  const size = 32;
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(1, 4);

  const xorSize = size * size * 4;
  const andSize = size * 4;
  const bmpHeaderSize = 40;
  const imageSize = bmpHeaderSize + xorSize + andSize;

  const entry = Buffer.alloc(16);
  entry[0] = size;
  entry[1] = size;
  entry[2] = 0;
  entry[3] = 0;
  entry.writeUInt16LE(1, 4);
  entry.writeUInt16LE(32, 6);
  entry.writeUInt32LE(imageSize, 8);
  entry.writeUInt32LE(22, 12);

  const dib = Buffer.alloc(bmpHeaderSize);
  dib.writeUInt32LE(bmpHeaderSize, 0);
  dib.writeInt32LE(size, 4);
  dib.writeInt32LE(size * 2, 8);
  dib.writeUInt16LE(1, 12);
  dib.writeUInt16LE(32, 14);
  dib.writeUInt32LE(0, 16);
  dib.writeUInt32LE(xorSize + andSize, 20);

  const xorMask = Buffer.alloc(xorSize);
  for (let i = 0; i < size * size; i++) {
    const o = i * 4;
    xorMask[o] = b;
    xorMask[o + 1] = g;
    xorMask[o + 2] = r;
    xorMask[o + 3] = a;
  }
  const andMask = Buffer.alloc(andSize, 0);

  return Buffer.concat([header, entry, dib, xorMask, andMask]);
}

fs.writeFileSync(path.join(iconsDir, "32x32.png"), createSolidPng(32, 46, 56, 68, 255));
fs.writeFileSync(path.join(iconsDir, "128x128.png"), createSolidPng(128, 46, 56, 68, 255));
fs.writeFileSync(path.join(iconsDir, "icon.png"), createSolidPng(128, 46, 56, 68, 255));
fs.writeFileSync(path.join(iconsDir, "icon.ico"), createSolidIco32(46, 56, 68, 255));
console.log("Generated placeholder Tauri icons in src-tauri/icons/");
