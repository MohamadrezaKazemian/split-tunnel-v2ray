import React, { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { parseLink, parseSubscription, type Profile } from './core/links';
import { generateConfig, type SelectedApp } from './core/config';
import type { RunningApp } from './core/processes';
import { RunningAppsPicker } from './RunningAppsPicker';
import { InstalledAppsPicker, type InstalledApp } from './InstalledAppsPicker';
import './style.css';

type Data = { profiles: Profile[]; active: number; apps: SelectedApp[]; binary: string; mode: 'inclusive'|'exclusive'; level: 'trace'|'debug'|'info'|'warn'|'error'|'fatal'|'panic' };
type Event = {status:string; logs:string[]};
const initial: Data = { profiles: [], active: 0, apps: [], binary: '', mode:'inclusive', level:'info' };
function App() {
  const [data, setData] = useState<Data>(initial), [loaded, setLoaded] = useState(false);
  const [status, setStatus] = useState('disconnected'), [error, setError] = useState(''), [link, setLink] = useState('');
  const [installed, setInstalled] = useState<InstalledApp[]|null>(null), [loadingInstalled, setLoadingInstalled] = useState(false);
  const [logs, setLogs] = useState<string[]>([]), [running, setRunning] = useState<RunningApp[]|null>(null);
  const last = useRef(Promise.resolve());
  useEffect(() => {
    invoke<Data|null>('load_data').then(d => { if (d) setData({...initial, ...d}); }).catch(e => setError(String(e))).finally(() => setLoaded(true));
    invoke<string>('status').then(setStatus).catch(e => setError(String(e)));
    const off = listen<Event>('tunnel-status', e => { setStatus(e.payload.status); if (e.payload.logs.length) setLogs(l => [...l, ...e.payload.logs].slice(-1000)); });
    return () => { void off.then(f => f()); };
  }, []);
  useEffect(() => { if (loaded) { last.current = last.current.then(() => invoke<void>('save_data', {data})).catch(e => {setError(String(e));}); } }, [data, loaded]);
  const update = (change:Partial<Data>) => setData(d => ({...d,...change}));
  const addLink = async () => { try {
    const input = link.trim();
    const profiles = /^https?:\/\//i.test(input) ? parseSubscription(await invoke<string>('fetch_subscription', {url:input})) : [parseLink(input)];
    setData(d => ({...d, profiles:[...d.profiles,...profiles], active:d.profiles.length}));
    setLink(''); setError('');
  } catch(e) { setError(String(e)); } };
  const addApp = (app:SelectedApp) => { if (!data.apps.some(a => a.path === app.path)) update({apps:[...data.apps,app]}); };
  const browse = async () => { try { const selected = await open({multiple:false, directory:false}); if (typeof selected === 'string') { const app = await invoke<InstalledApp>('resolve_app', {path:selected}); if (app.sandbox && /[\\/](flatpak|snap)$/i.test(app.path)) throw new Error('This launcher uses a shared sandbox executable; choose the application executable directly.'); addApp(app); } } catch(e) { setError(String(e)); } };
  const browseEngine = async () => { const selected = await open({multiple:false, directory:false}); if (typeof selected === 'string') update({binary:selected}); };
  const toggle = async () => { setError(''); try {
    if (status === 'connected') { await invoke('disconnect'); return; }
    const profile = data.profiles[data.active]; if (!profile) throw new Error('Add and select a configuration.');
    setLogs([]); setStatus('connecting');
    await invoke('connect', { binary:data.binary || null, config:generateConfig({proxy:profile.outbound, apps:data.apps, mode:data.mode, logLevel:data.level}) });
  } catch(e) { setStatus(`failed: ${String(e)}`); setError(String(e)); } };
  const showInstalled = async () => { setLoadingInstalled(true); try { setInstalled(await invoke<InstalledApp[]>('installed_apps')); } catch(e) { setError(String(e)); } finally { setLoadingInstalled(false); } };
  const showProcesses = async () => { try { setRunning(await invoke<RunningApp[]>('running_apps')); } catch(e) { setError(String(e)); } };
  const rank: Record<string, number> = {trace:0,debug:1,info:2,warn:3,error:4,fatal:5,panic:6};
  const visible = logs.filter(line => { const level = line.match(/\b(TRACE|DEBUG|INFO|WARN|ERROR|FATAL|PANIC)\b/i)?.[1].toLowerCase(); return !level || rank[level] >= rank[data.level]; });
  return <main><header><div><h1>Split Tunnel</h1><p>Choose which apps use your proxy.</p></div><span className={'status '+(status === 'connected' ? 'on' : status.startsWith('failed') ? 'failed' : '')}><i/>{status.startsWith('failed') ? 'Failed' : status}</span></header>
    <section><div className="section-title"><h2>Configuration</h2>{data.profiles.length > 0 && <button className="text" onClick={() => update({profiles:data.profiles.filter((_,i)=>i!==data.active),active:0})}>Remove</button>}</div>
      <select value={data.profiles.length ? data.active : ''} onChange={e=>update({active:Number(e.target.value)})}><option value="" disabled>No configuration selected</option>{data.profiles.map((p,i)=><option value={i} key={i}>{p.name} · {p.outbound.type}</option>)}</select>
      <div className="row"><input value={link} onChange={e=>setLink(e.target.value)} placeholder="Paste a proxy link or subscription URL"/><button onClick={addLink}>Add</button></div></section>
    <section><div className="section-title"><div><h2>Applications</h2><p>{data.mode === 'inclusive' ? 'Only these apps use the proxy.' : 'These apps bypass the proxy.'}</p></div><div className="row"><button onClick={showProcesses}>Running apps</button><button onClick={showInstalled} disabled={loadingInstalled}>{loadingInstalled ? 'Loading…' : 'Installed apps'}</button><button onClick={browse}>Browse</button></div></div>
      {data.apps.length === 0 ? <div className="empty">No applications selected</div> : <ul>{data.apps.map((a,i)=><li key={i}><span title={a.path}>{a.name || a.path?.split(/[\\/]/).pop()}<small>{a.name && a.path ? a.path : ''}</small></span><button className="text" onClick={()=>update({apps:data.apps.filter((_,j)=>i!==j)})}>Remove</button></li>)}</ul>}
    </section>
    {error && <div className="error" role="alert">{error}</div>}
    <button className="connect" disabled={status==='connecting'} onClick={toggle}>{status === 'connected' ? 'Disconnect' : status === 'connecting' ? 'Connecting…' : 'Connect'}</button>
    <details><summary>Settings</summary><div className="settings"><label>Tunnel mode<select value={data.mode} onChange={e=>update({mode:e.target.value as Data['mode']})}><option value="inclusive">Inclusive</option><option value="exclusive">Exclusive</option></select></label><label>Log level<select value={data.level} onChange={e=>update({level:e.target.value as Data['level']})}>{['trace','debug','info','warn','error','fatal','panic'].map(x=><option key={x}>{x}</option>)}</select></label><details className="advanced"><summary>Advanced</summary><label>Engine override (optional)<div className="row"><input readOnly value={data.binary} placeholder="Automatic"/><button onClick={browseEngine}>Browse</button>{data.binary && <button onClick={() => update({binary:''})}>Clear</button>}</div></label></details></div></details>
    <details><summary>Logs <span>{visible.length}</span></summary><pre>{visible.join('\n') || 'No logs yet.'}</pre></details>
    {installed && <InstalledAppsPicker apps={installed} selected={data.apps} onClose={() => setInstalled(null)} onSelect={addApp}/>}
    {running && <RunningAppsPicker apps={running} onClose={() => setRunning(null)} onSelect={app => { addApp(app); setRunning(null); }}/>}
  </main>;
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><App/></React.StrictMode>);
