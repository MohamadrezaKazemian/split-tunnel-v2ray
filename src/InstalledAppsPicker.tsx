import { useEffect, useRef, useState } from 'react';
import { appRule, filterRunningApps, type AppRule } from './core/processes';
export interface InstalledApp { name: string; path: string; sandbox?: string | null; matchDir?: string | null }
export function InstalledAppsPicker({apps, selected, onClose, onSelect}: {apps:InstalledApp[]; selected:AppRule[]; onClose:()=>void; onSelect:(app:AppRule)=>void}) {
  const [query,setQuery] = useState(''); const results = useRef<HTMLUListElement>(null);
  const filtered = filterRunningApps(apps, query) as InstalledApp[];
  useEffect(() => { if (results.current) results.current.scrollTop = 0; }, [query]);
  return <div className="overlay"><div className="modal" role="dialog" aria-modal="true" aria-labelledby="installed-apps-title">
    <div className="section-title"><h2 id="installed-apps-title">Installed applications</h2><button onClick={onClose}>Close</button></div>
    <input autoFocus aria-label="Search installed applications" placeholder="Search by name or path" value={query} onChange={e=>setQuery(e.target.value)}/>
    <p className="search-count" aria-live="polite">{filtered.length} matching applications</p>
    <p className="search-count">Shared launchers and sandboxed apps may not expose an app-specific install directory.</p>
    <ul ref={results} className="process-results">{filtered.map(app => {
      const rule = appRule(app); const added = !!rule && selected.some(a => a.kind === rule.kind && a.pattern.toLowerCase() === rule.pattern.toLowerCase());
      return <li key={`${app.name}\0${app.path}`}><span>{app.name}{app.sandbox && <small>{app.sandbox}</small>}<small>{rule?.pattern ?? `App directory unavailable: ${app.path}`}</small></span><button disabled={added || !rule} onClick={()=>{if (rule) onSelect(rule);}}>{added ? 'Added' : rule ? 'Add whole app' : 'Unavailable'}</button></li>;
    })}</ul>
  </div></div>;
}
