import { useEffect, useRef, useState } from 'react';
import { appRule, filterRunningApps, pathRule, type AppRule } from './core/processes';
export interface InstalledApp { name: string; path: string; sandbox?: string | null; matchDir?: string | null }
export function InstalledAppsPicker({apps, selected, onClose, onSelect}: {apps:InstalledApp[]; selected:AppRule[]; onClose:()=>void; onSelect:(app:AppRule)=>void}) {
  const [query,setQuery] = useState(''); const results = useRef<HTMLUListElement>(null);
  const filtered = filterRunningApps(apps, query) as InstalledApp[];
  useEffect(() => { if (results.current) results.current.scrollTop = 0; }, [query]);
  return <div className="overlay"><div className="modal" role="dialog" aria-modal="true" aria-labelledby="installed-apps-title">
    <div className="section-title"><h2 id="installed-apps-title">Installed applications</h2><button onClick={onClose}>Close</button></div>
    <input autoFocus aria-label="Search installed applications" placeholder="Search by name or path" value={query} onChange={e=>setQuery(e.target.value)}/>
    <p className="search-count" aria-live="polite">{filtered.length} matching applications</p>
    <p className="search-count">Apps in shared directories use their exact executable path; helpers may need separate rules. Shared sandbox launchers remain unavailable.</p>
    <ul ref={results} className="process-results">{filtered.map(app => {
      const sharedSandbox = Boolean(app.sandbox && !app.matchDir && /[\\/](?:flatpak|snap)$/i.test(app.path));
      const rule = appRule(app) ?? (sharedSandbox ? null : pathRule(app));
      const added = !!rule && selected.some(a => a.kind === rule.kind && a.pattern.toLowerCase() === rule.pattern.toLowerCase());
      return <li key={`${app.name}\0${app.path}`}><span>{app.name}{app.sandbox && <small>{app.sandbox}</small>}<small>{rule?.pattern ?? `Shared sandbox launcher: ${app.path}`}</small></span><button disabled={added || !rule} onClick={()=>{if (rule) onSelect(rule);}}>{added ? 'Added' : !rule ? 'Unavailable' : rule.kind === 'app' ? 'Add whole app' : 'Add executable'}</button></li>;
    })}</ul>
  </div></div>;
}
