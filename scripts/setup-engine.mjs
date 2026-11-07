import { createHash } from 'node:crypto';
import { createReadStream, createWriteStream } from 'node:fs';
import { chmod, mkdir, mkdtemp, rename, rmdir, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { execFileSync, spawn } from 'node:child_process';

// Recreate the project's tiny placeholder icons without storing binary files in Git.
await mkdir('src-tauri/icons', { recursive: true });
await writeFile('src-tauri/icons/icon.png', Buffer.from('iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAAL0lEQVR4nO3OIQEAAAgDMApTg7hUgBg3E/Ornr2kEhAQEBAQEBAQEBAQEBAQSAceQveApgBmoEcAAAAASUVORK5CYII=', 'base64'));
await writeFile('src-tauri/icons/icon.ico', Buffer.from('AAABAAEAICAAAAEAIABoAAAAFgAAAIlQTkcNChoKAAAADUlIRFIAAAAgAAAAIAgGAAAAc3p69AAAAC9JREFUeJztziEBAAAIAzAKU4O4VIAYNxPzq569pBIQEBAQEBAQEBAQEBAQEEgHHkL3gKYAZqBHAAAAAElFTkSuQmCC', 'base64'));

const version = '1.14.4';
const builds = {
  'linux-x64': ['linux', 'amd64', '699a1b510aa4396b2176d32d1181920399ebf1e267835da6f3b598d0f7c24968'],
  'linux-arm64': ['linux', 'arm64', '436441f76ee2f7fe21b5b2d426d3cb261a480455a704865789ad533e4273ec26'],
  'win32-x64': ['windows', 'amd64', '99185d75d314a07dbba4a130c8ad986cbeb629a07927975636b341ca61f991df'],
  'win32-arm64': ['windows', 'arm64', '8cd0a4d0cd8cd480be5731ac31f17038969a42d12add9feb6e031fd43d85bbae'],
};
const build = builds[`${process.platform}-${process.arch}`];
if (!build) throw new Error(`Unsupported platform: ${process.platform}/${process.arch}; supported: Linux and Windows x64/arm64`);
const [os, arch, expectedHash] = build;
const triple = execFileSync('rustc', ['-vV'], { encoding: 'utf8' }).match(/^host: (.+)$/m)?.[1];
if (!triple || !(os === 'linux' ? triple.includes('linux') : triple.includes('windows')) || !(arch === 'amd64' ? triple.startsWith('x86_64-') : triple.startsWith('aarch64-'))) throw new Error(`Rust host target does not match this platform: ${triple ?? 'unknown'}`);
if (os === 'windows' && arch === 'amd64' && triple !== 'x86_64-pc-windows-msvc') throw new Error('Windows x64 requires the x86_64-pc-windows-msvc Rust toolchain; install and select the MSVC toolchain first');
const name = `sing-box-${version}-${os}-${arch}`;
const extension = os === 'windows' ? '.zip' : '.tar.gz';
const member = `${name}/sing-box${os === 'windows' ? '.exe' : ''}`;
const sidecarDir = resolve('src-tauri/binaries');
const destination = join(sidecarDir, `sing-box-${triple}${os === 'windows' ? '.exe' : ''}`);
const tempDir = await mkdtemp(join(tmpdir(), 'split-tunnel-engine-'));
const archive = join(tempDir, name + extension);
const tempBinary = join(tempDir, 'sing-box' + (os === 'windows' ? '.exe' : ''));
try {
  const url = `https://github.com/SagerNet/sing-box/releases/download/v${version}/${name}${extension}`;
  console.log(`Downloading ${name}${extension}…`);
  const response = await fetch(url, { signal: AbortSignal.timeout(180_000), headers: { 'user-agent': 'split-tunnel-engine-setup' } });
  if (!response.ok || !response.body) throw new Error(`Download failed: HTTP ${response.status}`);
  await pipeline(Readable.fromWeb(response.body), createWriteStream(archive, { flags: 'wx' }));
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(archive)) hash.update(chunk);
  if (hash.digest('hex') !== expectedHash) throw new Error('SHA-256 checksum mismatch; refusing to install the engine');
  // Modern Windows includes bsdtar, which reads both ZIP and tar.gz archives.
  const tar = spawn('tar', ['-xOf', archive, member], { stdio: ['ignore', 'pipe', 'pipe'] });
  let errorOutput = '';
  tar.stderr.on('data', chunk => { errorOutput += chunk.toString(); });
  const exit = new Promise((resolveExit, reject) => { tar.once('error', reject); tar.once('close', code => code === 0 ? resolveExit() : reject(new Error(`Could not extract engine: ${errorOutput.trim()}`))); });
  await Promise.all([pipeline(tar.stdout, createWriteStream(tempBinary, { flags: 'wx' })), exit]);
  if (os === 'linux') await chmod(tempBinary, 0o755);
  await mkdir(sidecarDir, { recursive: true });
  // Copy across filesystems before renaming atomically in the sidecar directory.
  const staging = `${destination}.${process.pid}.tmp`;
  try { await pipeline(createReadStream(tempBinary), createWriteStream(staging, { flags: 'wx', mode: 0o755 })); await rename(staging, destination); }
  finally { await unlink(staging).catch(() => {}); }
  console.log(`Engine installed: ${destination}`);
} finally {
  await unlink(archive).catch(() => {});
  await unlink(tempBinary).catch(() => {});
  await rmdir(tempDir).catch(() => {});
}
