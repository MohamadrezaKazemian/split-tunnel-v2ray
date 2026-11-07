export interface RunningApp { name: string; path: string; matchDir?: string | null; sandbox?: string | null; protected?: boolean }
export type RuleKind = 'app' | 'term' | 'path';
export interface AppRule { kind: RuleKind; label: string; pattern: string }
export interface LegacyApp { name?: string; path?: string; matchDir?: string | null; sandbox?: string | null }

export const isWindowsPath = (path: string) => /^[a-z]:[\\/]/i.test(path) || path.startsWith('\\\\');
export function escapeRegex(value: string): string { return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }
export function normalizePath(path: string): string {
  return isWindowsPath(path) ? path.replace(/\//g, '\\').replace(/\\+$/, '') : path.replace(/\/+$/, '') || '/';
}
const shared = new Set(['/', '/usr', '/usr/bin', '/usr/lib', '/usr/lib64', '/usr/share', '/usr/local', '/usr/local/bin', '/usr/lib/x86_64-linux-gnu', '/usr/lib/aarch64-linux-gnu', '/bin', '/opt', '/snap', '/snap/bin', '/var/lib/flatpak', 'c:', 'c:/program files', 'c:/program files (x86)', 'c:/windows', 'c:/windows/system32']);
export function safeAppRoot(path: string, suggested?: string | null): string | null {
  const windows = isWindowsPath(path);
  const sep = windows ? '\\' : '/';
  const clean = normalizePath(path);
  if (!windows && !clean.startsWith('/')) return null;
  let root = suggested ? normalizePath(suggested) : clean.slice(0, clean.lastIndexOf(sep));
  if (!root || isWindowsPath(root) !== windows) return null;
  // Version directories belong to the same installation across app updates.
  const version = root.match(windows ? /\\app-[0-9][^\\]*(?:\\.*)?$/i : /\/app-[0-9][^/]*(?:\/.*)?$/i);
  if (version?.index !== undefined) root = root.slice(0, version.index);
  if (windows) root = root.replace(/\\Application(?:\\[0-9][^\\]*)?$/i, '');
  // Snap revisions and Flatpak deployments resolve to versioned subdirectories.
  const snap = !windows && root.match(/^(\/snap\/[^/]+)(?:\/.*)?$/);
  if (snap) root = snap[1];
  const flatpak = !windows && root.match(/^(.*\/flatpak\/app\/[^/]+)(?:\/.*)?$/);
  if (flatpak) root = flatpak[1];
  const key = root.replace(/\\/g, '/').replace(/\/$/, '').toLowerCase() || '/';
  if (shared.has(key) || /^[a-z]:$/i.test(root) || root === '/') return null;
  return root;
}
export function appRule(app: RunningApp): AppRule | null {
  if (app.sandbox && !app.matchDir && /[\\/](?:flatpak|snap)$/i.test(app.path)) return null;
  const root = safeAppRoot(app.path, app.matchDir);
  return root ? { kind: 'app', label: app.name, pattern: root } : null;
}
export function pathRule(app: RunningApp): AppRule { return { kind: 'path', label: app.name, pattern: app.path }; }
export function migrateRule(value: unknown): AppRule | null {
  if (!value || typeof value !== 'object') return null;
  const a = value as Partial<AppRule & LegacyApp>;
  if ((a.kind === 'app' || a.kind === 'term' || a.kind === 'path') && typeof a.pattern === 'string' && a.pattern.trim())
    return { kind: a.kind, pattern: a.pattern.trim(), label: typeof a.label === 'string' && a.label.trim() ? a.label.trim() : a.pattern.trim() };
  if (typeof a.path === 'string' && a.path.trim()) {
    const path = a.path.trim();
    const app = { name: a.name?.trim() || path.split(/[\\/]/).pop() || path, path, matchDir: a.matchDir, sandbox: a.sandbox };
    // Preserve old directory matching; also upgrade versioned Discord installations.
    if (a.matchDir || /[\\/]discord(?:canary|ptb|development)?[\\/](?:app-[^\\/]+[\\/])?(?:discord[^\\/]*|update)\.exe$/i.test(path)) return appRule(app) ?? pathRule(app);
    return pathRule(app);
  }
  if (typeof a.name === 'string' && a.name.trim()) return { kind: 'path', label: a.name.trim(), pattern: a.name.trim() };
  return null;
}
export function ruleRegex(rule: AppRule): string {
  const pattern = rule.pattern.trim();
  if (!pattern) throw new Error('Rule pattern cannot be empty');
  if (rule.kind === 'term') return `(?i)${escapeRegex(pattern)}`;
  const path = normalizePath(pattern);
  const prefix = isWindowsPath(path) ? '(?i)' : '';
  if (rule.kind === 'path') {
    if (!/[\\/]/.test(path)) return `${/\.exe$/i.test(path) ? '(?i)' : ''}(?:^|[\\\\/])${escapeRegex(path)}$`;
    return `${prefix}^${escapeRegex(path)}$`;
  }
  const root = safeAppRoot(`${path}${isWindowsPath(path) ? '\\' : '/'}placeholder`, path);
  if (!root || normalizePath(root) !== path) throw new Error(`Invalid or shared application directory: ${pattern}`);
  return `${prefix}^${escapeRegex(path)}${isWindowsPath(path) ? '\\\\' : '/'}`;
}
export function matchesRule(rule: AppRule, app: RunningApp): boolean {
  if (app.protected) return false;
  if (rule.kind === 'term') { const term = rule.pattern.toLowerCase(); return app.name.toLowerCase().includes(term) || app.path.toLowerCase().includes(term); }
  const expression = ruleRegex(rule);
  const insensitive = expression.startsWith('(?i)');
  return new RegExp(insensitive ? expression.slice(4) : expression, insensitive ? 'i' : '').test(normalizePath(app.path));
}
export function filterRunningApps(apps: RunningApp[], query: string): RunningApp[] {
  const term = query.trim().toLowerCase();
  return term ? apps.filter(a => a.name.toLowerCase().includes(term) || a.path.toLowerCase().includes(term)) : apps;
}
export interface ProcessGroup { label: string; root: string | null; apps: RunningApp[] }
export function groupRunningApps(apps: RunningApp[]): ProcessGroup[] {
  const groups = new Map<string, ProcessGroup>();
  for (const app of apps) {
    const root = appRule(app)?.pattern ?? null;
    const key = root ? `app:${isWindowsPath(root) ? root.toLowerCase() : root}` : `path:${isWindowsPath(app.path) ? app.path.toLowerCase() : app.path}`;
    const group = groups.get(key);
    if (group) group.apps.push(app); else groups.set(key, { label: app.name, root, apps: [app] });
  }
  return [...groups.values()].sort((a,b) => a.label.localeCompare(b.label));
}
export function isSystemProcess(app: RunningApp): boolean {
  return /^(?:svchost(?:\.exe)?|systemd(?:-.+)?|system|registry|smss\.exe|csrss\.exe|wininit\.exe|services\.exe|lsass\.exe|winlogon\.exe)$/i.test(app.name) || /[\\/]windows[\\/]system32[\\/]/i.test(app.path);
}
