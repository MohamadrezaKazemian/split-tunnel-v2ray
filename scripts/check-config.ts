import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { generateConfig } from '../src/core/config';
import { parseLink } from '../src/core/links';
const dir = mkdtempSync(join(tmpdir(), 'split-tunnel-check-'));
try {
  const file = join(dir, 'config.json');
  const proxy = parseLink('vless://00000000-0000-4000-8000-000000000000@example.com:443?security=tls').outbound;
  for (const mode of ['inclusive', 'exclusive', 'whole'] as const) {
    const apps = mode === 'whole' ? [] : [{ kind:'term' as const, label:'sample', pattern:'sample-app' }, { kind:'app' as const, label:'Browser', pattern:'/opt/Browser+X' }, { kind:'app' as const, label:'Chrome', pattern:'C:\\Program Files\\Chrome' }];
    writeFileSync(file, JSON.stringify(generateConfig({ proxy, apps, mode }), null, 2));
    const result = spawnSync(process.env.SING_BOX || 'sing-box', ['check', '-c', file], { stdio: 'inherit' });
    if (result.error) throw result.error;
    if (result.status !== 0) process.exitCode = result.status || 1;
    else console.log(`${mode} sample config passed sing-box check`);
  }
} finally { rmSync(dir, { recursive: true, force: true }); }
