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
  it('generates inclusive rules and direct fallback', () => { const c = generateConfig({proxy,apps:[{path:'/usr/bin/firefox'},{name:'app.exe'}]}) as any; expect(c.route.final).toBe('direct'); expect(c.route.rules[2]).toMatchObject({process_path:['/usr/bin/firefox'],outbound:'proxy'}); expect(c.dns.rules[1]).toMatchObject({process_name:['app.exe'],server:'fakeip'}); expect(c.dns.servers[1].detour).toBe('proxy'); });
  it('matches related executables only within an app-specific directory', () => {
    const c = generateConfig({proxy, apps:[{path:'/opt/Browser+X/browser', matchDir:'/opt/Browser+X'}]}) as any;
    const rule = c.route.rules.find((r:any) => r.process_path_regex);
    const regex = new RegExp(rule.process_path_regex[0]);
    expect(regex.test('/opt/Browser+X/helper')).toBe(true);
    expect(regex.test('/opt/Browser+X-other/helper')).toBe(false);
    expect(c.dns.rules.some((r:any) => r.process_path_regex)).toBe(true);
    const win = generateConfig({proxy, apps:[{path:'C:\\Program Files\\Chrome\\chrome.exe', matchDir:'C:\\Program Files\\Chrome'}]}) as any;
    const winRule = win.route.rules.find((r:any) => r.process_path_regex && new RegExp(r.process_path_regex[0].replace('(?i)', ''), 'i').test('c:\\program files\\chrome\\helper.exe'));
    expect(new RegExp(winRule.process_path_regex[0].replace('(?i)', ''), 'i').test('c:\\program files\\chrome\\helper.exe')).toBe(true);
    const shared = generateConfig({proxy,apps:[{path:'/usr/bin/firefox',matchDir:'/usr/bin'}]}) as any;
    expect(shared.route.rules.some((r:any) => r.process_path_regex)).toBe(false);
  });
  it('matches Discord updater and versioned executables for old and new selections', () => {
    const root = 'C:\\Users\\ExampleUser\\AppData\\Local\\Discord';
    for (const selected of [`${root}\\Discord.exe`, `${root}\\app-1.0.9267\\Discord.exe`]) {
      const c = generateConfig({proxy, apps:[{path:selected}]}) as any;
      const patterns = c.route.rules.flatMap((r:any) => r.process_path_regex ?? []);
      const matches = (candidate:string) => patterns.some((p:string) => new RegExp(p.replace('(?i)', ''), 'i').test(candidate));
      expect(matches(`${root}\\app-1.0.9267\\Discord.exe`)).toBe(true);
      expect(matches(`${root}\\app-1.0.9999\\Discord.exe`)).toBe(true);
      expect(matches(`${root}\\Update.exe`)).toBe(true);
      expect(matches(`${root}\\other.exe`)).toBe(false);
      expect(matches(`${root}2\\app-1.0.9267\\Discord.exe`)).toBe(false);
      expect(c.dns.rules.some((r:any) => r.process_path_regex?.some((p:string) => new RegExp(p.replace('(?i)', ''), 'i').test(`${root}\\Update.exe`)))).toBe(true);
    }
  });
  it('matches Windows names and full paths without case sensitivity', () => {
    const config = generateConfig({proxy, apps:[{name:'chrome.exe'}, {path:'C:\\Apps\\Browser\\Chrome.EXE'}]}) as any;
    const expressions = config.route.rules.flatMap((r:any) => r.process_path_regex ?? []);
    const matches = (path:string) => expressions.some((p:string) => new RegExp(p.replace('(?i)', ''), 'i').test(path));
    expect(matches('C:\\OTHER\\Chrome.EXE')).toBe(true);
    expect(matches('c:\\apps\\browser\\chrome.exe')).toBe(true);
    expect(matches('c:\\apps\\browser\\chrome.exe.old')).toBe(false);
    const forward = generateConfig({proxy,apps:[{path:'C:/Apps/Browser/Chrome.exe'}]}) as any;
    const forwardRegex = forward.route.rules.find((r:any) => r.process_path_regex).process_path_regex[0];
    expect(new RegExp(forwardRegex.replace('(?i)', ''), 'i').test('c:\\apps\\browser\\chrome.exe')).toBe(true);
  });
  it('keeps readiness logs enabled even for warning-level display', () => { const c = generateConfig({proxy,apps:[{name:'app'}],logLevel:'warn'}) as any; expect(c.log.level).toBe('info'); });
  it('generates exclusive fallback and rejects empty app lists', () => { const c = generateConfig({proxy,apps:[{name:'app'}],mode:'exclusive'}) as any; expect(c.route.final).toBe('proxy'); expect(c.route.rules[2].outbound).toBe('direct'); expect(c.dns.final).toBe('remote'); expect(() => generateConfig({proxy,apps:[]})).toThrow(); });
});
