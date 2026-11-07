export type PingMeasurement = { kind: 'ms'; ms: number } | { kind: 'timeout' };
export type PingResult = { id: string; tcp: PingMeasurement; real: PingMeasurement };

export function pingText(measurement: PingMeasurement | undefined): string {
  if (!measurement) return 'Not tested';
  return measurement.kind === 'ms' ? `${measurement.ms} ms` : 'Timeout';
}

export function pingTone(measurement: PingMeasurement | undefined): 'idle' | 'good' | 'warn' | 'bad' {
  if (!measurement) return 'idle';
  if (measurement.kind === 'timeout' || measurement.ms >= 800) return 'bad';
  return measurement.ms < 300 ? 'good' : 'warn';
}

export function pingRank(measurement: PingMeasurement | undefined): number {
  return measurement?.kind === 'ms' ? measurement.ms : Number.POSITIVE_INFINITY;
}
