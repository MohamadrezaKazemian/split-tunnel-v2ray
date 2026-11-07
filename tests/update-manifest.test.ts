import { mkdtempSync, readFileSync, rmSync, rmdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { expect, it } from 'vitest';

it('creates release-specific updater metadata with the signatures for both platforms', () => {
  const dir = mkdtempSync(join(tmpdir(), 'split-tunnel-update-test-'));
  try {
    writeFileSync(join(dir, 'split-tunnel-v2ray_1.2.3-beta.1_linux_amd64.AppImage.sig'), 'linux-signature\n');
    writeFileSync(join(dir, 'split-tunnel-v2ray_1.2.3-beta.1_windows_x64-setup.exe.sig'), 'windows-signature\n');
    const result = spawnSync(process.execPath, ['scripts/generate-update-manifest.mjs', '1.2.3-beta.1', dir], { encoding:'utf8' });
    expect(result.status, result.stderr).toBe(0);
    const manifest = JSON.parse(readFileSync(join(dir, 'latest.json'), 'utf8'));
    expect(manifest.version).toBe('1.2.3-beta.1');
    expect(manifest.platforms['linux-x86_64']).toEqual({signature:'linux-signature', url:expect.stringContaining('/v1.2.3-beta.1/split-tunnel-v2ray_1.2.3-beta.1_linux_amd64.AppImage')});
    expect(manifest.platforms['windows-x86_64']).toEqual({signature:'windows-signature', url:expect.stringContaining('/v1.2.3-beta.1/split-tunnel-v2ray_1.2.3-beta.1_windows_x64-setup.exe')});
  } finally {
    for (const file of ['split-tunnel-v2ray_1.2.3-beta.1_linux_amd64.AppImage.sig', 'split-tunnel-v2ray_1.2.3-beta.1_windows_x64-setup.exe.sig', 'latest.json']) rmSync(join(dir, file), {force:true});
    rmdirSync(dir);
  }
});
