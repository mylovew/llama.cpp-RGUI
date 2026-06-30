// 使用 Node.js 标准库生成 PNG 图标（无需外部依赖）
const zlib = require("zlib");
const fs = require("fs");
const path = require("path");

const iconDir = path.join(
  "E:",
  "java",
  "testProduct",
  "llama.cpp-RGUI",
  "src-tauri",
  "icons"
);
if (!fs.existsSync(iconDir)) fs.mkdirSync(iconDir, { recursive: true });

function crc32(buf) {
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) {
    crc ^= buf[i];
    for (let j = 0; j < 8; j++) {
      crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function makeChunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const crcData = Buffer.concat([typeBuf, data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(crcData), 0);
  return Buffer.concat([length, typeBuf, data, crc]);
}

function createPNG(width, height, drawFn) {
  // 构建扫描行（每行前加 filter byte 0）
  const rowLen = width * 4 + 1;
  const scanlines = Buffer.alloc(height * rowLen);
  for (let y = 0; y < height; y++) {
    scanlines[y * rowLen] = 0; // filter: none
    for (let x = 0; x < width; x++) {
      const [r, g, b, a] = drawFn(x, y);
      const off = y * rowLen + 1 + x * 4;
      scanlines[off] = r;
      scanlines[off + 1] = g;
      scanlines[off + 2] = b;
      scanlines[off + 3] = a;
    }
  }
  const compressed = zlib.deflateSync(scanlines, { level: 9 });
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // RGBA
  return Buffer.concat([
    signature,
    makeChunk("IHDR", ihdr),
    makeChunk("IDAT", compressed),
    makeChunk("IEND", Buffer.alloc(0)),
  ]);
}

function drawPixel(x, y, size) {
  const indigo = [99, 102, 241, 255];
  const white = [255, 255, 255, 255];

  // 圆角矩形白色背景
  const margin = size * 0.08;
  const radius = size * 0.16;
  if (x < margin || x > size - margin || y < margin || y > size - margin) {
    return indigo;
  }
  // 四个角的圆角检测
  const inCornerArea =
    (x < margin + radius || x > size - margin - radius) &&
    (y < margin + radius || y > size - margin - radius);
  if (inCornerArea) {
    let cx, cy;
    if (x < margin + radius) cx = margin + radius;
    else cx = size - margin - radius;
    if (y < margin + radius) cy = margin + radius;
    else cy = size - margin - radius;
    const dx = x - cx;
    const dy = y - cy;
    if (dx * dx + dy * dy > radius * radius) return indigo;
  }

  // L 字母（竖线 + 底部横线）
  const letterSize = size * 0.46;
  const lx = (size - letterSize) / 2;
  const ly = (size - letterSize) / 2 - size * 0.02;
  const stroke = letterSize * 0.22;
  // 竖线
  if (x >= lx && x <= lx + stroke && y >= ly && y <= ly + letterSize) {
    return indigo;
  }
  // 底部横线
  if (
    x >= lx &&
    x <= lx + letterSize &&
    y >= ly + letterSize - stroke &&
    y <= ly + letterSize
  ) {
    return indigo;
  }
  return white;
}

// 生成各尺寸 PNG
for (const size of [32, 128, 256, 512]) {
  const png = createPNG(size, size, (x, y) => drawPixel(x, y, size));
  const name = `${size}x${size}.png`;
  fs.writeFileSync(path.join(iconDir, name), png);
  console.log(`Generated ${name} (${png.length} bytes)`);
}
// @2x 图标
fs.copyFileSync(
  path.join(iconDir, "256x256.png"),
  path.join(iconDir, "[email protected]")
);
console.log("Generated [email protected]");

// 生成 ICO（封装 32x32 PNG）
const png32 = fs.readFileSync(path.join(iconDir, "32x32.png"));
const icoHeader = Buffer.alloc(6);
icoHeader[2] = 1; // type: icon
icoHeader[4] = 1; // count: 1
const icoEntry = Buffer.alloc(16);
icoEntry[0] = 32;
icoEntry[1] = 32;
icoEntry[4] = 1;
icoEntry[6] = 32;
icoEntry.writeUInt32BE(png32.length, 8);
icoEntry.writeUInt32BE(22, 12); // offset
fs.writeFileSync(
  path.join(iconDir, "icon.ico"),
  Buffer.concat([icoHeader, icoEntry, png32])
);
console.log("Generated icon.ico");

// ICNS 占位（macOS 打包时用 tauri icon 重新生成）
fs.copyFileSync(
  path.join(iconDir, "256x256.png"),
  path.join(iconDir, "icon.icns")
);
console.log("Generated icon.icns (placeholder)");

console.log("\nAll icons generated in:", iconDir);
