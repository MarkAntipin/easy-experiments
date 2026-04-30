export function formatTimestamp(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) return '—';
  try {
    return new Date(ms).toLocaleString();
  } catch {
    return String(ms);
  }
}

export function formatRelative(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) return '—';
  const delta = ms - Date.now();
  const absSec = Math.round(Math.abs(delta) / 1000);
  const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });
  const thresholds: Array<[Intl.RelativeTimeFormatUnit, number]> = [
    ['year', 60 * 60 * 24 * 365],
    ['month', 60 * 60 * 24 * 30],
    ['week', 60 * 60 * 24 * 7],
    ['day', 60 * 60 * 24],
    ['hour', 60 * 60],
    ['minute', 60],
  ];
  for (const [unit, size] of thresholds) {
    if (absSec >= size) {
      return rtf.format(Math.round(delta / (size * 1000)), unit);
    }
  }
  return rtf.format(Math.round(delta / 1000), 'second');
}
