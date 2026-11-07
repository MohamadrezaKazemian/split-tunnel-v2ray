// @vitest-environment jsdom
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';
import { RunningAppsPicker } from '../src/RunningAppsPicker';

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const container = document.createElement('div');
document.body.appendChild(container);
const root = createRoot(container);
afterEach(() => { act(() => root.render(null)); });

it('updates the visible rows when processes share an executable path', () => {
  const apps = [
    { name: 'Alpha', path: '/usr/bin/shared' },
    { name: 'Telegram', path: '/usr/bin/shared' },
    { name: 'Telegram Helper', path: '/usr/bin/shared' },
    { name: 'Firefox', path: '/usr/bin/firefox' },
  ];
  act(() => root.render(<RunningAppsPicker apps={apps} onClose={vi.fn()} onSelect={vi.fn()} />));
  expect(container.querySelectorAll('.process-results li')).toHaveLength(4);
  const input = container.querySelector('input')!;
  const setValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!;
  act(() => { setValue.call(input, 'telegram'); input.dispatchEvent(new Event('input', { bubbles: true })); });
  expect(container.querySelector('.search-count')!.textContent).toContain('2 matching');
  const rows = Array.from(container.querySelectorAll('.process-results li')).map(row => row.textContent);
  expect(rows).toHaveLength(2);
  expect(rows.every(row => row?.includes('Telegram'))).toBe(true);
  expect(container.textContent).not.toContain('Firefox');
  expect(container.textContent).not.toContain('Alpha');
});
