import { describe, expect, it } from 'vitest';
import { appRule, filterRunningApps, groupRunningApps, matchesRule, migrateRule, ruleRegex, safeAppRoot, type AppRule } from '../src/core/processes';
const discord = 'C:\\Users\\ExampleUser\\AppData\\Local\\Discord';
const fixtures = [
  { name:'Discord', path:`${discord}\\app-1.0.9267\\Discord.exe` },
  { name:'Discord Helper', path:`${discord}\\app-1.0.9267\\helper.exe` },
  { name:'Update', path:`${discord}\\Update.exe` },
  { name:'Chrome', path:'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe' },
  { name:'Chrome Helper', path:'C:\\Program Files\\Google\\Chrome\\Application\\helper.exe' },
  { name:'Telegram', path:'/opt/Telegram Desktop/Telegram' },
  { name:'Telegram Helper', path:'/opt/Telegram Desktop/helper' },
];
function testRegex(rule:AppRule, path:string) { const r=ruleRegex(rule); return new RegExp(r.replace(/^\(\?i\)/,''),r.startsWith('(?i)')?'i':'').test(path); }
describe('application rules', () => {
  it('groups Discord version folders with updater and future versions', () => {
    expect(safeAppRoot(fixtures[0].path)).toBe(discord);
    const rule=appRule(fixtures[0])!; expect(rule.pattern).toBe(discord);
    expect(testRegex(rule,`${discord}\\app-1.2.9999\\renderer.exe`)).toBe(true);
    expect(testRegex(rule,`${discord}\\Update.exe`)).toBe(true);
    expect(testRegex(rule,`${discord}2\\Update.exe`)).toBe(false);
    const groups=groupRunningApps(fixtures); expect(groups.find(g=>g.root===discord)?.apps).toHaveLength(3);
    expect(groups.find(g=>g.label==='Chrome')?.apps).toHaveLength(2);
    expect(groups.find(g=>g.label==='Chrome')?.root).toBe('C:\\Program Files\\Google\\Chrome');
    expect(groups.find(g=>g.label==='Telegram')?.apps).toHaveLength(2);
  });
  it('escapes special characters and spaces, preserving Linux case', () => {
    const rule:AppRule={kind:'app',label:'X',pattern:'/opt/Browser+X (beta)'};
    expect(testRegex(rule,'/opt/Browser+X (beta)/helper')).toBe(true);
    expect(testRegex(rule,'/opt/browser+X (beta)/helper')).toBe(false);
    expect(testRegex(rule,'/opt/BrowserXX (beta)/helper')).toBe(false);
  });
  it('matches Windows paths case-insensitively and normalizes slashes', () => {
    const rule:AppRule={kind:'app',label:'Chrome',pattern:'C:/Program Files/Google/Chrome'};
    expect(testRegex(rule,'c:\\program files\\google\\chrome\\Application\\chrome.exe')).toBe(true);
    expect(testRegex(rule,'C:\\Program Files\\Google\\ChromeOther\\chrome.exe')).toBe(false);
  });
  it('matches terms case-insensitively and exact path rules only', () => {
    expect(matchesRule({kind:'term',label:'tele',pattern:'TELE'},fixtures[5])).toBe(true);
    expect(matchesRule({kind:'term',label:'tele',pattern:'TELE'}, {...fixtures[5],protected:true})).toBe(false);
    expect(testRegex({kind:'path',label:'x',pattern:'C:\\Apps\\A+B.exe'},'c:\\apps\\a+b.EXE')).toBe(true);
    expect(testRegex({kind:'path',label:'x',pattern:'C:\\Apps\\A+B.exe'},'c:\\apps\\ab.EXE')).toBe(false);
  });
  it('migrates legacy selections without broadening ordinary paths', () => {
    expect(migrateRule({name:'Discord',path:fixtures[0].path})).toEqual({kind:'app',label:'Discord',pattern:discord});
    expect(migrateRule({name:'Editor',path:'/usr/bin/editor'})).toEqual({kind:'path',label:'Editor',pattern:'/usr/bin/editor'});
    expect(migrateRule({name:'Browser',path:'/opt/browser/browser',matchDir:'/opt/browser'})).toEqual({kind:'app',label:'Browser',pattern:'/opt/browser'});
    expect(migrateRule({name:'curl'})).toEqual({kind:'path',label:'curl',pattern:'curl'});
  });
  it('keeps shared directories out of whole-app rules and supports sandbox roots', () => {
    expect(appRule({name:'ls',path:'/usr/bin/ls'})).toBeNull();
    expect(safeAppRoot('/snap/code/123/usr/share/code/code')).toBe('/snap/code');
    expect(safeAppRoot('/var/lib/flatpak/app/org.example.App/x86_64/stable/hash/files/bin/app')).toBe('/var/lib/flatpak/app/org.example.App');
  });
  it('searches full paths case-insensitively', () => { expect(filterRunningApps(fixtures,'google')).toHaveLength(2); });
});
