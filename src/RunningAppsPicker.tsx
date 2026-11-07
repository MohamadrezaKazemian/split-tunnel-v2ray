import { useEffect, useRef, useState } from 'react';
import { appRule, filterRunningApps, groupRunningApps, isSystemProcess, pathRule, type AppRule, type RunningApp } from './core/processes';
interface Props { apps: RunningApp[]; onClose: () => void; onSelect: (rule: AppRule) => void }
export function RunningAppsPicker({ apps, onClose, onSelect }: Props) {
  const [query, setQuery] = useState('');
  const results = useRef<HTMLUListElement>(null);
  const filtered = filterRunningApps(apps.filter(app => !app.protected), query);
  const groups = groupRunningApps(filtered);
  useEffect(() => { if (results.current) results.current.scrollTop = 0; }, [query]);
  const addTerm = () => {
    const term = query.trim();
    const reasons = [term.length < 3 ? 'This term has fewer than 3 characters.' : '', (filtered.some(isSystemProcess) || ['svchost', 'systemd', 'system', 'services', 'lsass', 'winlogon'].some(name => name.includes(term.toLowerCase()))) ? 'This term matches system processes.' : ''].filter(Boolean);
    if (reasons.length && !window.confirm(`${reasons.join(' ')} Add this broad rule?`)) return;
    onSelect({ kind:'term', label:term, pattern:term });
  };
  return <div className="overlay"><div className="modal" role="dialog" aria-modal="true" aria-labelledby="running-apps-title">
    <div className="section-title"><h2 id="running-apps-title">Running applications</h2><button onClick={onClose}>Close</button></div>
    <input autoFocus aria-label="Search running applications" placeholder="Search by name or path" value={query} onChange={e => setQuery(e.target.value)}/>
    <p className="search-count" aria-live="polite">{filtered.length} matching process{filtered.length === 1 ? '' : 'es'} in {groups.length} group{groups.length === 1 ? '' : 's'}</p>
    {query.trim() && <button onClick={addTerm}>Add all matching '{query.trim()}' ({filtered.length})</button>}
    {navigator.userAgent.includes('Windows') && <p className="search-count">UWP and Microsoft Store apps may not be matched reliably.</p>}
    <ul ref={results} className="process-results">{groups.slice(0, 200).map(group => <li key={group.root ?? group.apps[0].path} className="process-group"><div><strong>{group.label}</strong> · {group.apps.length} process{group.apps.length === 1 ? '' : 'es'}<small>{group.root ?? 'Shared directory; select individual executable'}</small>
      <details><summary>Individual processes</summary><ul>{group.apps.map((app,index) => <li key={`${app.path}\0${index}`}><span>{app.name}<small>{app.path}</small></span><button onClick={() => onSelect(pathRule(app))}>Add process</button></li>)}</ul></details></div>
      <button disabled={!group.root} onClick={() => { const rule = appRule(group.apps[0]); if (rule) onSelect(rule); }}>Add whole app</button></li>)}</ul>
  </div></div>;
}
