import { describe, expect, it } from 'vitest';
import { filterRunningApps } from '../src/core/processes';

describe('running app search', () => {
  const apps = [
    { name: 'Firefox', path: '/usr/lib/firefox/firefox' },
    { name: 'code', path: '/snap/code/current/usr/share/code/code' },
    { name: 'Example.exe', path: 'C:\\Program Files\\Example\\Example.exe' },
  ];
  it('filters names case-insensitively and ignores surrounding spaces', () => {
    expect(filterRunningApps(apps, '  FIREfox  ')).toEqual([apps[0]]);
  });
  it('matches executable filenames and searches the complete path when given a path', () => {
    expect(filterRunningApps(apps, 'example.exe')).toEqual([apps[2]]);
    expect(filterRunningApps(apps, '/snap/code')).toEqual([apps[1]]);
  });
  it('does not match unrelated directory names for a plain app name', () => {
    expect(filterRunningApps(apps, 'snap')).toEqual([]);
    expect(filterRunningApps(apps, 'missing')).toEqual([]);
    expect(filterRunningApps(apps, ' ')).toEqual(apps);
  });
});
