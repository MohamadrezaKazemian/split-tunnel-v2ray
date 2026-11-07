// @vitest-environment jsdom
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import { InstalledAppsPicker } from '../src/InstalledAppsPicker';
(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
it('adds an app root and avoids shared launchers', () => {
 const element=document.createElement('div'); document.body.appendChild(element); const root=createRoot(element); const add=vi.fn();
 const apps=[{name:'Firefox',path:'/opt/firefox/firefox',matchDir:'/opt/firefox'},{name:'Flatpak',path:'/usr/bin/flatpak',sandbox:'Flatpak'}];
 act(()=>root.render(<InstalledAppsPicker apps={apps} selected={[]} onClose={vi.fn()} onSelect={add}/>));
 const buttons=element.querySelectorAll('li button'); expect(buttons[1].hasAttribute('disabled')).toBe(true);
 act(()=>buttons[0].dispatchEvent(new MouseEvent('click',{bubbles:true})));
 expect(add).toHaveBeenCalledWith({kind:'app',label:'Firefox',pattern:'/opt/firefox'});
 act(()=>root.unmount()); element.remove();
});
