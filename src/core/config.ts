import type { Outbound } from './links';
export interface SelectedApp { name?: string; path?: string; matchDir?: string | null; sandbox?: string | null }
export interface ConfigOptions { proxy: Outbound; apps: SelectedApp[]; mode?: 'inclusive' | 'exclusive'; logLevel?: 'trace' | 'debug' | 'info' | 'warn' | 'error' | 'fatal' | 'panic' }
/** Discord uses versioned app-* directories and a separate updater on Windows. */
function discordWindowsInstallRoot(path: string): string | undefined {
  const windowsPath = path.replace(/\//g, '\\');
  const match = windowsPath.match(/^([a-z]:\\users\\[^\\]+\\appdata\\local\\discord(?:canary|ptb|development)?)(?:\\app-[^\\]+)?\\discord(?:canary|ptb|development)?\.exe$/i);
  return match?.[1];
}
/** Candidate configuration: routing and DNS attribution require platform integration and verification. */
export function generateConfig({ proxy, apps, mode = 'inclusive', logLevel = 'info' }: ConfigOptions): Record<string, unknown> {
  if (!apps.length || apps.some(a => !a.name?.trim() && !a.path?.trim())) throw new Error('Select at least one valid application');
  const matchers = apps.flatMap<Record<string, string[]>>(app => {
    const path = app.path?.trim();
    if (!path) {
      const name = app.name!.trim();
      if (!/\.exe$/i.test(name)) return [{ process_name: [name] }];
      const escapedName = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      return [{ process_name: [name] }, { process_path_regex: [`(?i)(?:^|[\\\\/])${escapedName}$`] }];
    }
    const isWindowsPath = /^[A-Za-z]:[\\/]/.test(path) || path.startsWith('\\\\');
    const exact = { process_path: [path] };
    const exactMatchers: Record<string, string[]>[] = [exact];
    if (isWindowsPath) {
      const escapedPath = path.replace(/\//g, '\\').replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      exactMatchers.push({ process_path_regex: [`(?i)^${escapedPath}$`] });
    }
    const discordRoot = !app.sandbox && isWindowsPath ? discordWindowsInstallRoot(path) : undefined;
    if (discordRoot) {
      const escapedRoot = discordRoot.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      exactMatchers.push({ process_path_regex: [`(?i)^${escapedRoot}\\\\(?:app-[^\\\\]+\\\\)?(?:discord(?:canary|ptb|development)?|update)\\.exe$`] });
    }
    const dir = app.matchDir?.replace(/[\\/]+$/, '');
    const parent = path.replace(/[\\/][^\\/]+$/, '');
    const normalized = dir?.replace(/\\/g, '/').toLowerCase();
    const shared = ['/', '/usr', '/usr/bin', '/usr/lib', '/usr/lib64', '/usr/share', '/usr/local', '/usr/lib/x86_64-linux-gnu', '/usr/lib/aarch64-linux-gnu', '/usr/local/bin', '/bin', '/snap/bin', '/opt', 'c:', 'c:/windows', 'c:/windows/system32'];
    if (!dir || app.sandbox || normalized !== parent.replace(/\\/g, '/').toLowerCase() || shared.includes(normalized!)) return exactMatchers;
    const escaped = (isWindowsPath ? dir.replace(/\//g, '\\') : dir).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const separator = isWindowsPath ? '\\\\' : '/';
    const prefix = isWindowsPath ? '(?i)' : '';
    return [...exactMatchers, { process_path_regex: [`${prefix}^${escaped}${separator}`] }];
  });
  const selected = mode === 'inclusive' ? 'proxy' : 'direct';
  const final = mode === 'inclusive' ? 'direct' : 'proxy';
  return {
    log: { level: ['trace', 'debug'].includes(logLevel) ? logLevel : 'info', timestamp: true },
    dns: { servers: [{ type: 'local', tag: 'local' }, { type: 'https', tag: 'remote', server: '1.1.1.1', detour: 'proxy' }, { type: 'fakeip', tag: 'fakeip', inet4_range: '198.18.0.0/15', inet6_range: '2001:2::/48' }], rules: mode === 'inclusive' ? [...matchers.map(m => ({ ...m, query_type: ['A', 'AAAA'], action: 'route', server: 'fakeip' })), ...matchers.map(m => ({ ...m, action: 'route', server: 'remote' }))] : [...matchers.map(m => ({ ...m, action: 'route', server: 'local' })), { query_type: ['A', 'AAAA'], action: 'route', server: 'fakeip' }], final: mode === 'inclusive' ? 'local' : 'remote' },
    inbounds: [{ type: 'tun', tag: 'tun-in', address: ['172.19.0.1/30', 'fdfe:dcba:9876::1/126'], auto_route: true, strict_route: false }],
    outbounds: [proxy, { type: 'direct', tag: 'direct' }],
    route: { auto_detect_interface: true, default_domain_resolver: 'local', rules: [{ action: 'sniff' }, { protocol: 'dns', action: 'hijack-dns' }, ...matchers.map(m => ({ ...m, action: 'route', outbound: selected }))], final }
  };
}
