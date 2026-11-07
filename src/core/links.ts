export type Outbound = Record<string, unknown> & { type: string; tag: 'proxy'; server: string; server_port: number };
export interface Profile { name: string; outbound: Outbound }

function decode64(value: string): string {
  try {
    const normalized = value.replace(/-/g, '+').replace(/_/g, '/');
    if (!/^[A-Za-z0-9+/]*={0,2}$/.test(normalized) || normalized.length % 4 === 1) throw new Error();
    return new TextDecoder('utf-8', { fatal: true }).decode(Uint8Array.from(atob(normalized), c => c.charCodeAt(0)));
  } catch { throw new Error('Invalid base64 data'); }
}
function required(value: unknown, name: string): string {
  if (typeof value !== 'string' || !value.trim()) throw new Error(`Missing ${name}`);
  return value;
}
function port(value: unknown): number {
  const n = Number(value);
  if (!Number.isInteger(n) || n < 1 || n > 65535) throw new Error('Invalid server port');
  return n;
}
function base(type: string, server: string, serverPort: unknown): Outbound {
  return { type, tag: 'proxy', server: required(server, 'server').replace(/^\[|\]$/g, ''), server_port: port(serverPort) };
}
function transport(type: string, path?: string | null, host?: string | null, service?: string | null): Record<string, unknown> | undefined {
  if (!type || type === 'tcp') return undefined;
  if (type === 'ws') return { type: 'ws', ...(path ? { path } : {}), ...(host ? { headers: { Host: host } } : {}) };
  if (type === 'grpc') return { type: 'grpc', ...(service ? { service_name: service } : {}) };
  if (type === 'http' || type === 'h2') return { type: 'http', ...(path ? { path } : {}), ...(host ? { host: host.split(',') } : {}) };
  throw new Error(`Unsupported transport: ${type}`);
}
function tls(security: string, serverName?: string | null, fingerprint?: string | null, publicKey?: string | null, shortId?: string | null): Record<string, unknown> | undefined {
  if (!security || security === 'none') return undefined;
  if (security !== 'tls' && security !== 'reality') throw new Error(`Unsupported security: ${security}`);
  if (security === 'reality' && !publicKey) throw new Error('Missing Reality public key');
  return { enabled: true, ...(serverName ? { server_name: serverName } : {}), ...(fingerprint ? { utls: { enabled: true, fingerprint } } : {}), ...(security === 'reality' ? { reality: { enabled: true, public_key: publicKey, ...(shortId ? { short_id: shortId } : {}) } } : {}) };
}
function parseVmess(input: string): Profile {
  let v: Record<string, unknown>;
  try { v = JSON.parse(decode64(input.slice(8))) as Record<string, unknown>; } catch { throw new Error('Invalid VMess link'); }
  const outbound = base('vmess', required(v.add, 'server'), v.port);
  outbound.uuid = required(v.id, 'UUID');
  outbound.security = typeof v.scy === 'string' && v.scy ? v.scy : 'auto';
  const alterId = Number(v.aid || 0);
  if (!Number.isInteger(alterId) || alterId < 0) throw new Error('Invalid alter ID');
  if (alterId) outbound.alter_id = alterId;
  const t = transport(String(v.net || 'tcp'), String(v.path || ''), String(v.host || ''), String(v.path || ''));
  const s = tls(String(v.tls || ''), String(v.sni || v.host || ''), String(v.fp || ''));
  if (t) outbound.transport = t;
  if (s) outbound.tls = s;
  return { name: String(v.ps || v.add), outbound };
}
function parseShadowsocks(input: string): Profile {
  let raw = input.slice(5);
  const hash = raw.indexOf('#');
  const name = hash >= 0 ? decodeURIComponent(raw.slice(hash + 1)) : '';
  if (hash >= 0) raw = raw.slice(0, hash);
  const query = raw.indexOf('?');
  if (query >= 0) {
    const params = new URLSearchParams(raw.slice(query + 1));
    if (params.has('plugin')) throw new Error('Shadowsocks plugins are not supported');
    raw = raw.slice(0, query);
  }
  if (!raw.includes('@')) raw = decode64(raw);
  const at = raw.lastIndexOf('@');
  if (at < 0) throw new Error('Invalid Shadowsocks link');
  let credentials = raw.slice(0, at);
  if (!credentials.includes(':')) credentials = decode64(credentials);
  else credentials = decodeURIComponent(credentials);
  const colon = credentials.indexOf(':');
  if (colon < 1 || !credentials.slice(colon + 1)) throw new Error('Invalid Shadowsocks credentials');
  const endpoint = new URL(`http://${raw.slice(at + 1)}`);
  const outbound = base('shadowsocks', endpoint.hostname, endpoint.port);
  outbound.method = credentials.slice(0, colon); outbound.password = credentials.slice(colon + 1);
  return { name: name || outbound.server, outbound };
}
export function parseLink(input: string): Profile {
  input = input.trim();
  if (input.startsWith('vmess://')) return parseVmess(input);
  if (input.startsWith('ss://')) return parseShadowsocks(input);
  let u: URL;
  try { u = new URL(input); } catch { throw new Error('Invalid proxy link'); }
  const type = u.protocol.slice(0, -1);
  if (type !== 'vless' && type !== 'trojan') throw new Error(`Unsupported protocol: ${type}`);
  const outbound = base(type, u.hostname, u.port);
  const credential = required(decodeURIComponent(u.username), type === 'vless' ? 'UUID' : 'password');
  if (u.password) throw new Error('Unexpected password field');
  if (type === 'vless') { outbound.uuid = credential; const flow = u.searchParams.get('flow'); if (flow) outbound.flow = flow; }
  else outbound.password = credential;
  const p = u.searchParams;
  const t = transport(p.get('type') || 'tcp', p.get('path'), p.get('host'), p.get('serviceName'));
  const s = tls(p.get('security') || (type === 'trojan' ? 'tls' : ''), p.get('sni') || u.hostname, p.get('fp'), p.get('pbk'), p.get('sid'));
  if (t) outbound.transport = t;
  if (s) outbound.tls = s;
  return { name: u.hash ? decodeURIComponent(u.hash.slice(1)) : outbound.server, outbound };
}
export function parseSubscription(body: string): Profile[] {
  let content = body.trim().replace(/^\uFEFF/, '');
  if (!/^(vless|vmess|trojan|ss):\/\//m.test(content)) content = decode64(content);
  const lines = content.split(/\r?\n/).map(x => x.trim()).filter(x => x && !x.startsWith('#'));
  if (!lines.length) throw new Error('Subscription contains no links');
  return lines.map((line, i) => { try { return parseLink(line); } catch (e) { throw new Error(`Subscription line ${i + 1}: ${e instanceof Error ? e.message : String(e)}`); } });
}
