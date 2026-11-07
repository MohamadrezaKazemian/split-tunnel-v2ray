import type { Outbound } from './links';
import { ruleRegex, matchesRule, type AppRule, type RunningApp } from './processes';
export type SelectedApp = AppRule;
export interface ConfigOptions { proxy: Outbound; apps: AppRule[]; mode?: 'inclusive' | 'exclusive'; running?: RunningApp[]; logLevel?: 'trace' | 'debug' | 'info' | 'warn' | 'error' | 'fatal' | 'panic' }
export function generateConfig({ proxy, apps, mode = 'inclusive', logLevel = 'info', running = [] }: ConfigOptions): Record<string, unknown> {
  if (!apps.length) throw new Error('Select at least one valid application');
  const matchers: Record<string, string[]>[] = apps.flatMap(rule => {
    const paths = { process_path_regex: [ruleRegex(rule)] };
    const names = rule.kind === 'term' ? [...new Set(running.filter(app => matchesRule(rule, app)).map(app => app.name).filter(Boolean))] : rule.kind === 'path' && !/[\\/]/.test(rule.pattern) ? [rule.pattern] : [];
    return names.length ? [paths, { process_name: names }] : [paths];
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
