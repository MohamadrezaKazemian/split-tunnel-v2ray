import { describe, expect, it } from 'vitest';
import { parseLink, parseSubscription } from '../src/core/links';
import { generateConfig } from '../src/core/config';
const b64 = (s: string) => Buffer.from(s).toString('base64url');
describe('link parser', () => {
  it('parses VLESS with Reality and websocket', () => {
    const p = parseLink('vless://abc@example.com:443?security=reality&sni=example.com&fp=chrome&pbk=key&sid=12&type=ws&path=%2Fws#Test');
    expect(p.name).toBe('Test'); expect(p.outbound).toMatchObject({type:'vless',uuid:'abc',server:'example.com',server_port:443,tls:{reality:{public_key:'key'}},transport:{type:'ws',path:'/ws'}});
  });
  it('parses VMess', () => { expect(parseLink('vmess://' + b64(JSON.stringify({add:'example.org',port:'443',id:'id',ps:'VM',net:'ws',path:'/x',tls:'tls'}))).outbound).toMatchObject({type:'vmess',uuid:'id',transport:{type:'ws'},tls:{enabled:true}}); });
  it('parses Trojan', () => { expect(parseLink('trojan://secret@example.com:443#Trojan').outbound).toMatchObject({type:'trojan',password:'secret',tls:{enabled:true}}); });
  it('parses both Shadowsocks forms', () => { for (const link of [`ss://${b64('aes-256-gcm:secret')}@example.com:8388`, `ss://${b64('aes-256-gcm:secret@example.com:8388')}`]) expect(parseLink(link).outbound).toMatchObject({type:'shadowsocks',method:'aes-256-gcm',password:'secret'}); });
  it('parses plain and base64 subscriptions', () => { const lines = 'trojan://a@example.com:443\nvless://b@example.org:443'; expect(parseSubscription(lines)).toHaveLength(2); expect(parseSubscription(b64(lines))).toHaveLength(2); });
  it('rejects invalid and unsupported inputs', () => { expect(() => parseLink('vless://x@example.com:99999')).toThrow(); expect(() => parseLink('ss://x@example.com:123?plugin=foo')).toThrow(); expect(() => parseLink('vless://x@example.com:443?type=quic')).toThrow(); expect(() => parseSubscription('garbage')).toThrow(); });
});
describe('config generator', () => {
  const proxy = parseLink('trojan://secret@example.com:443').outbound;
  it.each(['inclusive','exclusive'] as const)('generates %s path and name matchers', mode => {
    const c = generateConfig({proxy, mode, apps:[{kind:'term',label:'Telegram',pattern:'telegram'}], running:[{name:'Telegram Helper',path:'/opt/Telegram/helper'}]}) as any;
    expect(c.route.final).toBe(mode === 'inclusive' ? 'direct' : 'proxy');
    expect(c.route.rules[2]).toMatchObject({process_path_regex:['(?i)telegram'],outbound:mode === 'inclusive' ? 'proxy' : 'direct'});
    expect(c.route.rules[3]).toMatchObject({process_name:['Telegram Helper']});
    expect(c.dns.rules.some((r:any) => r.process_path_regex)).toBe(true);
  });
  it('rejects empty selections and shared app roots', () => {
    expect(() => generateConfig({proxy,apps:[]})).toThrow();
    expect(() => generateConfig({proxy,apps:[{kind:'app',label:'bad',pattern:'/usr/bin'}]})).toThrow();
  });
});
