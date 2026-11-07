import { useEffect, useRef, useState } from 'react';
import { filterRunningApps, type RunningApp } from './core/processes';

interface Props {
  apps: RunningApp[];
  onClose: () => void;
  onSelect: (app: RunningApp) => void;
}

export function RunningAppsPicker({ apps, onClose, onSelect }: Props) {
  const [query, setQuery] = useState('');
  const results = useRef<HTMLUListElement>(null);
  const filtered = filterRunningApps(apps, query);

  useEffect(() => {
    if (results.current) results.current.scrollTop = 0;
  }, [query]);

  return <div className="overlay"><div className="modal" role="dialog" aria-modal="true" aria-labelledby="running-apps-title">
    <div className="section-title"><h2 id="running-apps-title">Running applications</h2><button onClick={onClose}>Close</button></div>
    <input autoFocus aria-label="Search running applications" placeholder="Search by name" value={query} onChange={e => setQuery(e.target.value)}/>
    <p className="search-count" aria-live="polite">{filtered.length === 0 ? 'No matching applications' : `${filtered.length} matching application${filtered.length === 1 ? '' : 's'}${filtered.length > 200 ? ' · showing first 200' : ''}`}</p>
    {navigator.userAgent.includes('Windows') && <p className="search-count">UWP and Microsoft Store apps may not be matched reliably.</p>}
    <ul ref={results} className="process-results">{filtered.slice(0, 200).map((app, index) =>
      <li key={`${app.name}\0${app.path}\0${index}`}><span>{app.name}{app.matchDir && <small>Includes executables in {app.matchDir}</small>}<small>{app.path}</small></span><button onClick={() => onSelect(app)}>Add</button></li>
    )}</ul>
  </div></div>;
}
