import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const [version, directory] = process.argv.slice(2);
if (!version || !directory || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) throw new Error('Expected a release version and asset directory');
const base = `https://github.com/MohamadrezaKazemian/split-tunnel-v2ray/releases/download/v${version}`;
const platform = name => ({ signature: readFileSync(join(directory, `${name}.sig`), 'utf8').trim(), url: `${base}/${name}` });
const notes = readFileSync('.github/RELEASE_NOTES.md', 'utf8').replaceAll('v0.1.0', `v${version}`).replaceAll('_0.1.0_', `_${version}_`);
const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    'linux-x86_64': platform(`split-tunnel-v2ray_${version}_linux_amd64.AppImage`),
    'windows-x86_64': platform(`split-tunnel-v2ray_${version}_windows_x64-setup.exe`),
  },
};
writeFileSync(join(directory, 'latest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
