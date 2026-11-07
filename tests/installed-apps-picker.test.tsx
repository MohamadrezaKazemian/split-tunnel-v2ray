// @vitest-environment jsdom
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import { InstalledAppsPicker } from '../src/InstalledAppsPicker';
(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
it('filters apps, marks selections, and avoids shared sandbox launchers', () => {
  const element = document.createElement('div'); document.body.appendChild(element);
  const root = createRoot(element); const add = vi.fn();
  const apps = [{name:'Firefox',path:'/opt/firefox/firefox',matchDir:'/opt/firefox'}, {name:'Flatpak app',path:'/usr/bin/flatpak',sandbox:'Flatpak'}, {name:'Other',path:'/opt/other'}];
  act(() => root.render(<InstalledAppsPicker apps={apps} selected={[{path:'/opt/firefox/firefox'}]} onClose={vi.fn()} onSelect={add}/>));
  const rows = element.querySelectorAll('li');
  expect(rows[0].querySelector('button')?.textContent).toBe('Added');
  expect(rows[1].querySelector('button')?.disabled).toBe(true);
  expect(rows[1].textContent).toContain('shared launcher');
  const input = element.querySelector('input')!;
  const setValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')!.set!;
  act(() => {setValue.call(input,'other'); input.dispatchEvent(new Event('input',{bubbles:true}));});
  expect(element.querySelectorAll('li')).toHaveLength(1);
  act(() => element.querySelector('li button')!.dispatchEvent(new MouseEvent('click',{bubbles:true})));
  expect(add).toHaveBeenCalledWith(apps[2]);
  act(() => root.unmount()); element.remove();
});
