// @vitest-environment jsdom
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import { RunningAppsPicker } from '../src/RunningAppsPicker';
(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
it('groups processes and confirms short search terms', () => {
 const element=document.createElement('div'); document.body.appendChild(element); const root=createRoot(element); const add=vi.fn(); const confirm=vi.spyOn(window,'confirm').mockReturnValue(false);
 const apps=[{name:'Discord',path:'/opt/discord/Discord'},{name:'Discord Helper',path:'/opt/discord/helper'},{name:'systemd',path:'/usr/lib/systemd/systemd'}];
 act(()=>root.render(<RunningAppsPicker apps={apps} onClose={vi.fn()} onSelect={add}/>));
 expect(element.textContent).toContain('2 processes');
 const input=element.querySelector('input')!; const set=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')!.set!;
 act(()=>{set.call(input,'di');input.dispatchEvent(new Event('input',{bubbles:true}));});
 const termButton=Array.from(element.querySelectorAll('button')).find(b=>b.textContent?.includes('Add all matching'))!;
 expect(termButton.textContent).toContain('(2)'); act(()=>termButton.dispatchEvent(new MouseEvent('click',{bubbles:true})));
 expect(confirm).toHaveBeenCalled(); expect(add).not.toHaveBeenCalled(); confirm.mockRestore(); act(()=>root.unmount());element.remove();
});
