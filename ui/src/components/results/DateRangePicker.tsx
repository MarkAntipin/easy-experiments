import { SegmentedControl } from '@/components/SegmentedControl';

export type RangePreset = '24h' | '7d' | '30d' | 'all';

const PRESETS: ReadonlyArray<{ value: RangePreset; label: string }> = [
  { value: '24h', label: '24h' },
  { value: '7d', label: '7d' },
  { value: '30d', label: '30d' },
  { value: 'all', label: 'All' },
];

const DAY_MS = 24 * 60 * 60 * 1000;

export interface ResolvedRange {
  /** undefined = let the server decide (started_at). */
  start: number | undefined;
  /** undefined = let the server decide (now or stopped_at + 7d). */
  end: number | undefined;
}

export function presetToRange(preset: RangePreset, now: number = Date.now()): ResolvedRange {
  switch (preset) {
    case '24h':
      return { start: now - DAY_MS, end: now };
    case '7d':
      return { start: now - 7 * DAY_MS, end: now };
    case '30d':
      return { start: now - 30 * DAY_MS, end: now };
    case 'all':
    default:
      return { start: undefined, end: undefined };
  }
}

export function DateRangePicker({
  value,
  onChange,
}: {
  value: RangePreset;
  onChange: (next: RangePreset) => void;
}) {
  return (
    <SegmentedControl<RangePreset>
      ariaLabel="Window"
      options={PRESETS}
      value={value}
      onChange={onChange}
      size="sm"
    />
  );
}
