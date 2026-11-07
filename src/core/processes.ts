export interface RunningApp { name: string; path: string; matchDir?: string | null }

/** Search names and executable filenames; search the full path when a path is entered. */
export function filterRunningApps(apps: RunningApp[], query: string): RunningApp[] {
  const term = query.trim().toLocaleLowerCase();
  if (!term) return apps;
  const isPath = /[\\/]/.test(term);
  return apps.filter(({ name, path }) => {
    const fileName = path.split(/[\\/]/).pop() ?? '';
    return name.toLocaleLowerCase().includes(term)
      || fileName.toLocaleLowerCase().includes(term)
      || (isPath && path.toLocaleLowerCase().includes(term));
  });
}
