import { useEffect, useRef, useState } from 'react';
import { filterRunningApps } from './core/processes';
import type { SelectedApp } from './core/config';

export interface InstalledApp { name: string; path: string; sandbox?: string | null; matchDir?: string | null }
export function InstalledAppsPicker({apps, selected, onClose, onSelect}: {apps:InstalledApp[]; selected:SelectedApp[]; onClose:()=>void; onSelect:(app:InstalledApp)=>void}) {
  const [query,setQuery] = useState(''); const results = useRef<HTMLUListElement>(null);
  const filtered = filterRunningApps(apps, query) as InstalledApp[];
  useEffect(() => { if (results.current) results.current.scrollTop = 0; }, [query]);
  return <div className="overlay"><div className="modal" role="dialog" aria-modal="true" aria-labelledby="installed-apps-title">
    <div className="section-title"><h2 id="installed-apps-title">Installed applications</h2><button onClick={onClose}>Close</button></div>
    <input autoFocus aria-label="Search installed applications" placeholder="Search by name" value={query} onChange={e=>setQuery(e.target.value)}/>
    <p className="search-count" aria-live="polite">{filtered.length === 0 ? 'No matching applications' : `${filtered.length} matching application${filtered.length === 1 ? '' : 's'}`}</p>
    <p className="search-count">{navigator.userAgent.includes('Windows') ? 'UWP and Microsoft Store apps may not be matched reliably.' : 'Flatpak and Snap apps may use different process paths, so matching can be unreliable.'}</p>
    <ul ref={results} className="process-results">{filtered.map(app => {
      const added = selected.some(a => a.path === app.path);
      const sharedWrapper = !!app.sandbox && /[\\/](flatpak|snap)$/i.test(app.path);
      return <li key={app.path}><span>{app.name}{app.sandbox && <small>{app.sandbox}{sharedWrapper ? ' · shared launcher; choose its executable directly' : ' · sandboxed'}</small>}{app.matchDir && <small>Includes executables in {app.matchDir}</small>}<small>{app.path}</small></span><button disabled={added || sharedWrapper} onClick={()=>onSelect(app)}>{added ? 'Added' : sharedWrapper ? 'Unavailable' : 'Add'}</button></li>;
    })}</ul>
  </div></div>;
}
