import { describe, expect, it } from 'vitest';
import { pingRank, pingText, pingTone } from '../src/core/ping';

describe('ping display helpers', () => {
  it('formats and colors live delay thresholds', () => {
    expect(pingText({kind:'ms',ms:42})).toBe('42 ms');
    expect(pingText({kind:'timeout'})).toBe('Timeout');
    expect(pingTone({kind:'ms',ms:299})).toBe('good');
    expect(pingTone({kind:'ms',ms:300})).toBe('warn');
    expect(pingTone({kind:'ms',ms:800})).toBe('bad');
    expect(pingTone({kind:'timeout'})).toBe('bad');
  });
  it('sorts successful pings ahead of timeouts', () => {
    expect(pingRank({kind:'ms',ms:50})).toBeLessThan(pingRank({kind:'timeout'}));
    expect(pingRank(undefined)).toBe(Infinity);
  });
});
