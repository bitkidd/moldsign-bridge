import { execFileSync } from 'node:child_process';
import { readFile, writeFile, mkdir, rm, mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import sharp from 'sharp';

const key = await readFile(
  new URL('../node_modules/@phosphor-icons/core/assets/bold/key-bold.svg', import.meta.url),
  'utf8',
);
const glyph = key.replace(/<svg[^>]*>/, '').replace('</svg>', '');
const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128"><defs><linearGradient id="background" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#8b7cf1"/><stop offset="1" stop-color="#4b40bd"/></linearGradient></defs><rect x="5" y="5" width="118" height="118" rx="28" fill="url(#background)"/><rect x="6" y="6" width="116" height="116" rx="27" fill="none" stroke="#fff" stroke-opacity=".2"/><g transform="translate(24 24) scale(.3125)" fill="#f5f1ff">${glyph}</g></svg>`;
await mkdir('ui/public', { recursive: true });
await writeFile('ui/public/key.svg', svg);
await writeFile(
  'src-tauri/icons/PHOSPHOR-LICENSE.txt',
  await readFile('node_modules/@phosphor-icons/core/LICENSE'),
);
const raster = (size) => sharp(Buffer.from(svg)).resize(size, size).png().toBuffer();
await writeFile('src-tauri/icons/icon.png', await raster(256));
const temporary = await mkdtemp(join(tmpdir(), 'moldsign-icons-'));
try {
  const iconset = join(temporary, 'key.iconset');
  await mkdir(iconset);
  for (const size of [16, 32, 128, 256, 512])
    for (const scale of [1, 2]) {
      await writeFile(
        join(iconset, `icon_${size}x${size}${scale === 2 ? '@2x' : ''}.png`),
        await raster(size * scale),
      );
    }
  execFileSync('/usr/bin/iconutil', ['-c', 'icns', iconset, '-o', 'src-tauri/icons/icon.icns']);
} finally {
  await rm(temporary, { recursive: true, force: true });
}
console.log('Generated app icons using Phosphor Key (bold).');
