import { spawnSync } from 'node:child_process';
import { parseLink } from '../src/core/links';

const input = process.env.SPLIT_TUNNEL_PING_CONFIG?.trim();
if (!input) {
  console.error('Set SPLIT_TUNNEL_PING_CONFIG to a VLESS, VMess, Trojan, Shadowsocks link, or outbound JSON.');
  process.exit(2);
}

let outbound: unknown;
try {
  outbound = input.startsWith('{') ? JSON.parse(input) : parseLink(input).outbound;
} catch {
  console.error('SPLIT_TUNNEL_PING_CONFIG is not a supported proxy link or outbound JSON.');
  process.exit(2);
}

const env: NodeJS.ProcessEnv = { ...process.env, SPLIT_TUNNEL_PING_OUTBOUND: JSON.stringify(outbound) };
delete env.SPLIT_TUNNEL_PING_CONFIG;
const result = spawnSync(
  'cargo',
  ['run', '--quiet', '--manifest-path', 'src-tauri/Cargo.toml', '--bin', 'ping_config'],
  { env, stdio: 'inherit' },
);
if (result.error) {
  console.error(`Could not run the ping verifier: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
