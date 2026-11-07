import React, { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open, save } from '@tauri-apps/plugin-dialog';
import { parseLink, parseSubscription, type Profile } from './core/links';
import { generateConfig, type SelectedApp, type TunnelMode } from './core/config';
import { appRule, pathRule, migrateRule, matchesRule, ruleRegex, isSystemProcess, type RunningApp } from './core/processes';
import { RunningAppsPicker } from './RunningAppsPicker';
import { InstalledAppsPicker, type InstalledApp } from './InstalledAppsPicker';
import { pingRank, pingText, pingTone, type PingMeasurement, type PingResult } from './core/ping';
import './style.css';

type Data = { profiles: Profile[]; active: number; apps: SelectedApp[]; binary: string; mode: TunnelMode; level: 'trace'|'debug'|'info'|'warn'|'error'|'fatal'|'panic'; autoUpdateCheck: boolean; theme: 'classic'|'modern' };
type AvailableUpdate = {version:string; notes:string; kind:string; command:string|null};
type Event = {status:string; logs:string[]};
const initial: Data = { profiles: [], active: 0, apps: [], binary: '', mode:'inclusive', level:'info', autoUpdateCheck:true, theme:'classic' };
function App() {
  const [data, setData] = useState<Data>(initial), [loaded, setLoaded] = useState(false);
  const [tab, setTab] = useState<'connection'|'settings'|'logs'>('connection');
  const [status, setStatus] = useState('disconnected'), [error, setError] = useState(''), [link, setLink] = useState('');
  const [installed, setInstalled] = useState<InstalledApp[]|null>(null), [loadingInstalled, setLoadingInstalled] = useState(false);
  const [logs, setLogs] = useState<string[]>([]), [running, setRunning] = useState<RunningApp[]>([]), [showRunning, setShowRunning] = useState(false), [includeSensitive, setIncludeSensitive] = useState(false);
  const [pings, setPings] = useState<Record<number, PingResult>>({}), [pinging, setPinging] = useState<Set<number>>(new Set()), [pingProgress, setPingProgress] = useState<{done:number;total:number}|null>(null), [livePing, setLivePing] = useState<PingMeasurement>();
  const [availableUpdate, setAvailableUpdate] = useState<AvailableUpdate|null>(null), [checkingUpdate, setCheckingUpdate] = useState(false), [installingUpdate, setInstallingUpdate] = useState(false), [updateMessage, setUpdateMessage] = useState('');
  const last = useRef(Promise.resolve());
  const pingRun = useRef(0);
  useEffect(() => {
    invoke<Data|null>('load_data').then(d => { if (d) setData({...initial, ...d, apps: (Array.isArray(d.apps) ? d.apps : []).map(migrateRule).filter((a): a is SelectedApp => a !== null)}); }).catch(e => setError(String(e))).finally(() => setLoaded(true));
    invoke<string>('status').then(setStatus).catch(e => setError(String(e)));
    const off = listen<Event>('tunnel-status', e => { setStatus(e.payload.status); if (e.payload.logs.length) setLogs(l => [...l, ...e.payload.logs].slice(-1000)); });
    return () => { void off.then(f => f()); };
  }, []);
  useEffect(() => { let active = true; const refresh = () => { void invoke<RunningApp[]>('running_apps', {binary:data.binary || null}).then(apps => { if (active) setRunning(apps); }).catch(() => {}); }; refresh(); const timer = window.setInterval(refresh, 5000); return () => { active = false; window.clearInterval(timer); }; }, [data.binary]);
  useEffect(() => {
    let active = true; let timer: number | undefined;
    if (status !== 'connected') setLivePing(undefined);
    const refresh = () => { if (active && status === 'connected' && !document.hidden) void invoke<PingMeasurement>('active_ping').then(ping => { if (active) setLivePing(ping); }).catch(() => {}); };
    const schedule = () => { window.clearInterval(timer); refresh(); if (!document.hidden && status === 'connected') timer = window.setInterval(refresh, 5000); };
    document.addEventListener('visibilitychange', schedule); schedule();
    return () => { active = false; window.clearInterval(timer); document.removeEventListener('visibilitychange', schedule); };
  }, [status]);
  useEffect(() => { if (loaded) { last.current = last.current.then(() => invoke<void>('save_data', {data})).catch(e => {setError(String(e));}); } }, [data, loaded]);
  useEffect(() => { document.documentElement.dataset.theme = data.theme === 'modern' ? 'modern' : 'classic'; }, [data.theme]);
  const checkedOnStartup = useRef(false);
  const checkUpdates = async (manual = false) => { setCheckingUpdate(true); setUpdateMessage(''); try {
    const release = await invoke<AvailableUpdate|null>('check_updates'); setAvailableUpdate(release);
    if (manual && !release) setUpdateMessage('You are up to date.');
  } catch (e) { if (manual) setUpdateMessage(`Update check failed: ${String(e)}`); } finally { setCheckingUpdate(false); } };
  useEffect(() => { if (loaded && !checkedOnStartup.current) { checkedOnStartup.current = true; if (data.autoUpdateCheck) void checkUpdates(); } }, [loaded, data.autoUpdateCheck]);
  const installUpdate = async () => { if (!availableUpdate) return; setInstallingUpdate(true); setUpdateMessage(''); try {
    await invoke('install_update', {version:availableUpdate.version});
  } catch (e) { setUpdateMessage(`Update failed: ${String(e)}`); setInstallingUpdate(false); } };
  const update = (change:Partial<Data>) => setData(d => ({...d,...change}));
  const runPing = async (entries:{profile:Profile;index:number}[]) => {
    if (!entries.length) return;
    const run = ++pingRun.current; setPinging(new Set(entries.map(e => e.index))); setPingProgress({done:0,total:entries.length});
    try {
      const results = await invoke<PingResult[]>('ping_configs', {binary:data.binary || null, targets:entries.map(({profile,index}) => ({id:String(index),outbound:profile.outbound}))});
      if (run !== pingRun.current) return;
      setPings(previous => ({...previous, ...Object.fromEntries(results.map(result => [Number(result.id), result]))}));
      setPingProgress({done:entries.length,total:entries.length});
    } catch (e) { if (run === pingRun.current) setError(String(e)); }
    finally { if (run === pingRun.current) { setPinging(new Set()); window.setTimeout(() => { if (run === pingRun.current) setPingProgress(null); }, 700); } }
  };
  const cancelPing = () => { pingRun.current += 1; setPinging(new Set()); setPingProgress(null); };
  const pingAll = () => void runPing(data.profiles.map((profile,index) => ({profile,index})));
  const sortByPing = () => {
    const active = data.profiles[data.active];
    const sorted = data.profiles.map((profile,index) => ({profile,ping:pings[index]})).sort((a,b) => pingRank(a.ping?.real) - pingRank(b.ping?.real));
    setPings(Object.fromEntries(sorted.map(({ping},index) => [index,ping]).filter((entry): entry is [number,PingResult] => Boolean(entry[1]))));
    update({profiles:sorted.map(entry => entry.profile),active:Math.max(0,sorted.findIndex(entry => entry.profile === active))});
  };
  const addLink = async () => { try {
    const input = link.trim();
    const profiles = /^https?:\/\//i.test(input) ? parseSubscription(await invoke<string>('fetch_subscription', {url:input})) : [parseLink(input)];
    const first = data.profiles.length;
    setData(d => ({...d, profiles:[...d.profiles,...profiles], active:d.profiles.length}));
    void runPing(profiles.map((profile,offset) => ({profile,index:first + offset})));
    setLink(''); setError('');
  } catch(e) { setError(String(e)); } };
  const addApp = (app:SelectedApp) => { try { ruleRegex(app); if (!data.apps.some(a => a.kind === app.kind && a.pattern === app.pattern)) update({apps:[...data.apps,app]}); setError(''); } catch(e) { setError(String(e)); } };
  const editApp = (index:number) => { const rule = data.apps[index]; const value = window.prompt(`Edit ${rule.kind} rule pattern`, rule.pattern); if (value === null) return; const pattern = value.trim(); const next = {...rule, pattern}; try { ruleRegex(next); if (next.kind === 'term' && (pattern.length < 3 || running.some(a => isSystemProcess(a) && matchesRule(next,a))) && !window.confirm('This term is short or matches system processes. Save it?')) return; update({apps:data.apps.map((a,i) => i === index ? next : a)}); setError(''); } catch(e) { setError(String(e)); } };
  const browse = async () => { try { const selected = await open({multiple:false, directory:false}); if (typeof selected === 'string') { const app = await invoke<InstalledApp>('resolve_app', {path:selected}); if (app.sandbox && !app.matchDir && /[\\/](flatpak|snap)$/i.test(app.path)) throw new Error('This launcher uses a shared sandbox executable; choose the application executable directly.'); addApp(appRule(app) ?? pathRule(app)); } } catch(e) { setError(String(e)); } };
  const browseEngine = async () => { const selected = await open({multiple:false, directory:false}); if (typeof selected === 'string') update({binary:selected}); };
  const toggle = async () => { setError(''); try {
    if (status === 'connected') { await invoke('disconnect'); return; }
    const profile = data.profiles[data.active]; if (!profile) throw new Error('Add and select a configuration.');
    setLogs([]); setStatus('connecting');
    await invoke('connect', { binary:data.binary || null, config:generateConfig({proxy:profile.outbound, apps:data.apps, mode:data.mode, logLevel:data.level, running}) });
  } catch(e) { setStatus(`failed: ${String(e)}`); setError(String(e)); } };
  const showInstalled = async () => { setLoadingInstalled(true); try { setInstalled(await invoke<InstalledApp[]>('installed_apps')); } catch(e) { setError(String(e)); } finally { setLoadingInstalled(false); } };
  const showProcesses = async () => { try { setRunning(await invoke<RunningApp[]>('running_apps', {binary:data.binary || null})); setShowRunning(true); } catch(e) { setError(String(e)); } };
  const logData = () => ({mode:data.mode, ruleCount:data.apps.length, logs, includeSensitive, binary:data.binary || null});
  const exportLogs = async () => { try {
    const day = new Date().toISOString().slice(0, 10);
    const path = await save({defaultPath:`split-tunnel-log-${day}.txt`, filters:[{name:'Text', extensions:['txt']}]});
    if (path) await invoke('export_log', {path, data:logData()});
  } catch (e) { setError(`Could not export log: ${String(e)}`); } };
  const copyLogs = async () => { try {
    const text = await invoke<string>('prepare_log_export', {data:logData()});
    await navigator.clipboard.writeText(text);
  } catch (e) { setError(`Could not copy log: ${String(e)}`); } };
  useEffect(() => { const off = listen('export-log-request', () => { void exportLogs(); }); return () => { void off.then(f => f()); }; }, [data, logs, includeSensitive]);
  const rank: Record<string, number> = {trace:0,debug:1,info:2,warn:3,error:4,fatal:5,panic:6};
  const visible = logs.filter(line => { const level = line.match(/\b(TRACE|DEBUG|INFO|WARN|ERROR|FATAL|PANIC)\b/i)?.[1].toLowerCase(); return !level || rank[level] >= rank[data.level]; });
  return <main><div className="classic-title" aria-hidden="true"><strong>Split Tunnel</strong></div><header><div><h1>Split Tunnel</h1><p>Choose which apps use your proxy.</p></div></header>
    <nav className="tabs" aria-label="Application sections">{(['connection','settings','logs'] as const).map(name => <button key={name} className={tab === name ? 'selected' : ''} aria-current={tab === name ? 'page' : undefined} onClick={() => setTab(name)}>{name === 'connection' ? 'Connection' : name === 'settings' ? 'Settings' : `Logs (${visible.length})`}</button>)}</nav>
    {availableUpdate && <section className="update-banner" role="status"><div className="section-title"><h2>Version {availableUpdate.version} available</h2><button className="text" onClick={() => setAvailableUpdate(null)}>Dismiss</button></div><pre>{availableUpdate.notes || 'No release notes provided.'}</pre>{availableUpdate.kind === 'appimage' || availableUpdate.kind === 'nsis' ? <button onClick={() => void installUpdate()} disabled={installingUpdate || status !== 'disconnected'}>{installingUpdate ? 'Installing…' : 'Update'}</button> : availableUpdate.command ? <div><p>Update with your package manager:</p><pre>{availableUpdate.command}</pre><button onClick={() => void navigator.clipboard.writeText(availableUpdate.command!).catch(e => setUpdateMessage(`Could not copy command: ${String(e)}`))}>Copy command</button></div> : <p>Automatic updates are unavailable for this installation. Download the appropriate package from GitHub Releases.</p>}{status !== 'disconnected' && (availableUpdate.kind === 'appimage' || availableUpdate.kind === 'nsis') && <p>Disconnect the tunnel before updating.</p>}</section>}
    {updateMessage && <p role="status">{updateMessage}</p>}
    {tab === 'connection' && <><section><div className="section-title"><h2>Configuration</h2><div className="row">{data.profiles.length > 1 && <button className="text" onClick={pingAll} disabled={pinging.size > 0}>Ping all</button>}{Object.keys(pings).length > 1 && <button className="text" onClick={sortByPing}>Sort by ping</button>}{data.profiles.length > 0 && <button className="text" onClick={() => { update({profiles:data.profiles.filter((_,i)=>i!==data.active),active:0}); setPings({}); }}>Remove</button>}</div></div>
      <select value={data.profiles.length ? data.active : ''} onChange={e=>update({active:Number(e.target.value)})}><option value="" disabled>No configuration selected</option>{data.profiles.map((p,i)=><option value={i} key={i}>{p.name} · {p.outbound.type}</option>)}</select>
      {data.profiles.length > 0 && <ul className="config-list">{data.profiles.map((profile,index) => <li className={index === data.active ? 'active' : ''} key={`${profile.name}-${index}`}><button className="config-select" onClick={() => update({active:index})}>{profile.name}<small>{profile.outbound.type}</small></button><span className={'ping '+pingTone(pings[index]?.real)} title={`TCP: ${pingText(pings[index]?.tcp)} · Real delay: ${pingText(pings[index]?.real)}`}>{pinging.has(index) ? 'Testing…' : pingText(pings[index]?.real)}</span><button className="text" onClick={() => void runPing([{profile,index}])} disabled={pinging.has(index)}>Ping</button></li>)}</ul>}
      {pingProgress && <div className="ping-progress" role="status">Testing {pingProgress.done}/{pingProgress.total} configurations… <button className="text" onClick={cancelPing}>Cancel</button></div>}
      <div className="row"><input value={link} onChange={e=>setLink(e.target.value)} placeholder="Paste a proxy link or subscription URL"/><button onClick={addLink}>Add</button></div></section>
    {data.mode === 'whole' ? <section><div className="section-title"><div><h2>Whole-system tunnel</h2><p>All non-local traffic and DNS use the proxy. The proxy endpoint, this app, and the engine stay direct to avoid routing loops.</p></div></div></section> : <section><div className="section-title"><div><h2>Applications</h2><p>{data.mode === 'inclusive' ? 'Only these apps use the proxy.' : 'These apps bypass the proxy.'}</p></div><div className="row"><button onClick={showProcesses}>Running apps</button><button onClick={showInstalled} disabled={loadingInstalled}>{loadingInstalled ? 'Loading…' : 'Installed apps'}</button><button onClick={browse}>Browse</button></div></div>
      {data.apps.length === 0 ? <div className="empty">No applications selected</div> : <ul>{data.apps.map((a,i)=><li key={i}><span title={a.pattern}>{a.label}<small>{a.kind} · {running.filter(p => matchesRule(a,p)).length} live matches · {a.pattern}</small></span><div className="row"><button className="text" onClick={()=>editApp(i)}>Edit</button><button className="text" onClick={()=>update({apps:data.apps.filter((_,j)=>i!==j)})}>Remove</button></div></li>)}</ul>}
    </section>}</>}
    {error && <div className="error" role="alert">{error}</div>}
    <button className="connect" disabled={status==='connecting' || installingUpdate} onClick={toggle}>{status === 'connected' ? 'Disconnect' : status === 'connecting' ? 'Connecting…' : 'Connect'}</button>
    {tab === 'settings' && <section><h2>Settings</h2><div className="settings"><label>Theme<select value={data.theme} onChange={e=>update({theme:e.target.value as Data['theme']})}><option value="classic">Classic</option><option value="modern">Modern</option></select></label><label>Tunnel mode<select value={data.mode} onChange={e=>update({mode:e.target.value as TunnelMode})}><option value="inclusive">Selected apps only</option><option value="exclusive">All except selected</option><option value="whole">Whole system</option></select></label><label>Log level<select value={data.level} onChange={e=>update({level:e.target.value as Data['level']})}>{['trace','debug','info','warn','error','fatal','panic'].map(x=><option key={x}>{x}</option>)}</select></label><label className="check-setting"><input type="checkbox" checked={data.autoUpdateCheck} onChange={e=>update({autoUpdateCheck:e.target.checked})}/> Check for updates on startup</label><div><button onClick={() => void checkUpdates(true)} disabled={checkingUpdate}>{checkingUpdate ? 'Checking…' : 'Check for updates'}</button></div><details className="advanced"><summary>Advanced</summary><label>Engine override (optional)<div className="row"><input readOnly value={data.binary} placeholder="Automatic"/><button onClick={browseEngine}>Browse</button>{data.binary && <button onClick={() => update({binary:''})}>Clear</button>}</div></label></details></div></section>}
    {tab === 'logs' && <section><h2>Logs <span>{visible.length}</span></h2><div className="log-actions"><label><input type="checkbox" checked={includeSensitive} onChange={e=>setIncludeSensitive(e.target.checked)}/> Include sensitive details</label><span>UUIDs, passwords, server addresses, URLs, and home paths are redacted by default.</span><div className="row"><button onClick={() => void exportLogs()}>Export log</button><button onClick={() => void copyLogs()}>Copy log</button><button className="text" onClick={() => setLogs([])}>Clear log</button></div></div><pre>{visible.join('\n') || 'No logs yet.'}</pre></section>}
    <footer className="status-bar"><span className={'status '+(status === 'connected' ? 'on' : status.startsWith('failed') ? 'failed' : '')}><i/>{status.startsWith('failed') ? 'Failed' : status}</span><span>{status === 'connected' ? `Live ping: ${pingText(livePing)}` : 'Not connected'}</span></footer>
    {installed && <InstalledAppsPicker apps={installed} selected={data.apps} onClose={() => setInstalled(null)} onSelect={addApp}/>}
    {showRunning && <RunningAppsPicker apps={running} onClose={() => setShowRunning(false)} onSelect={app => { addApp(app); setShowRunning(false); }}/>}
  </main>;
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><App/></React.StrictMode>);
