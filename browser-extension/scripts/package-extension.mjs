import { copyFileSync, cpSync, existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const extensionDir = resolve(scriptDir, '..');
const repoRoot = resolve(extensionDir, '..');
const manifest = JSON.parse(readFileSync(join(extensionDir, 'manifest.json'), 'utf8'));
const packageJson = JSON.parse(readFileSync(join(extensionDir, 'package.json'), 'utf8'));

const slug = packageJson.name.replace(/^@/, '').replace(/[^a-z0-9._-]+/gi, '-');
const version = manifest.version || packageJson.version;
const packageName = `${slug}-${version}`;
const distDir = join(extensionDir, 'dist');
const unpackedDir = join(distDir, packageName);
const downloadsDir = join(repoRoot, 'frontend', 'public', 'downloads');
const zipPath = join(downloadsDir, `${packageName}.zip`);

const requiredPaths = [
  'manifest.json',
  'background.js',
  'content-script.js',
  'sidepanel.html',
  'sidepanel.css',
  'sidepanel.js',
  'icons',
  'vendor/web-llm/index.js',
];

if (!existsSync(join(extensionDir, 'vendor', 'web-llm', 'index.js'))) {
  throw new Error('Missing vendor/web-llm/index.js. Run `npm run build:webllm` before packaging.');
}

rmSync(distDir, { recursive: true, force: true });
mkdirSync(unpackedDir, { recursive: true });
mkdirSync(downloadsDir, { recursive: true });

for (const relativePath of requiredPaths) {
  const source = join(extensionDir, relativePath);
  const target = join(unpackedDir, relativePath);
  if (!existsSync(source)) {
    throw new Error(`Missing required extension asset: ${relativePath}`);
  }

  mkdirSync(dirname(target), { recursive: true });
  cpSync(source, target, { recursive: true });
}

copyFileSync(join(extensionDir, 'README.md'), join(unpackedDir, 'README.md'));
copyFileSync(join(extensionDir, 'README.zh-TW.md'), join(unpackedDir, 'README.zh-TW.md'));

rmSync(zipPath, { force: true });

const zip = spawnSync('zip', ['-qr', zipPath, packageName], {
  cwd: distDir,
  stdio: 'inherit',
});

if (zip.error) {
  throw zip.error;
}

if (zip.status !== 0) {
  throw new Error(`zip exited with status ${zip.status}`);
}

console.log(`Packaged extension: ${zipPath}`);
