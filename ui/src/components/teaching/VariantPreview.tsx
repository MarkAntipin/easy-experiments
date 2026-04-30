import { variantColor } from '@/lib/variantColors';

export interface VariantPreviewProps {
  variantKey: string;
  isControl: boolean;
  configRaw: string;
  variantIndex: number;
}

type ParsedConfig =
  | { kind: 'object'; entries: Array<[string, unknown]> }
  | { kind: 'invalid' }
  | { kind: 'empty' };

function parseConfig(raw: string): ParsedConfig {
  const trimmed = (raw ?? '').trim();
  if (trimmed.length === 0 || trimmed === '{}') return { kind: 'empty' };
  try {
    const parsed = JSON.parse(trimmed);
    if (
      parsed &&
      typeof parsed === 'object' &&
      !Array.isArray(parsed) &&
      Object.keys(parsed).length > 0
    ) {
      return { kind: 'object', entries: Object.entries(parsed) };
    }
    if (
      parsed &&
      typeof parsed === 'object' &&
      !Array.isArray(parsed) &&
      Object.keys(parsed).length === 0
    ) {
      return { kind: 'empty' };
    }
    return { kind: 'invalid' };
  } catch {
    return { kind: 'invalid' };
  }
}

function formatValue(v: unknown): string {
  if (v === null) return 'null';
  if (typeof v === 'string') return `"${v}"`;
  if (typeof v === 'number' || typeof v === 'boolean') return String(v);
  try {
    return JSON.stringify(v);
  } catch {
    return String(v);
  }
}

export function VariantPreview({
  variantKey,
  isControl,
  configRaw,
  variantIndex,
}: VariantPreviewProps) {
  const color = variantColor(variantIndex);
  const parsed = parseConfig(configRaw);

  // 8-digit hex (#RRGGBBAA) for soft fills and borders that match the variant.
  const softBorder = `${color.hex}33`;
  const softTint = `${color.hex}10`;
  const labelTint = `${color.hex}1f`;

  return (
    <div
      className="overflow-hidden rounded-md border bg-white"
      style={{ borderColor: softBorder }}
    >
      <div
        className="flex flex-wrap items-center gap-2 border-b px-3 py-1.5"
        style={{ borderColor: softBorder, backgroundColor: softTint }}
      >
        <span
          className="inline-block h-2.5 w-2.5 rounded-sm"
          style={{ backgroundColor: color.hex }}
        />
        <span className="text-xs font-semibold text-ink-900">
          {variantKey || (
            <span className="italic text-slate-400">unnamed variant</span>
          )}
        </span>
        <span
          className="inline-flex items-center rounded-full px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wide"
          style={{ backgroundColor: labelTint, color: color.hex }}
        >
          {isControl ? 'Control' : 'Variant'}
        </span>
        <span className="ml-auto text-[10px] text-slate-400">
          A user in this variant sees:
        </span>
      </div>

      <div className="p-3">
        {parsed.kind === 'object' ? (
          <div className="grid grid-cols-1 gap-1.5 sm:grid-cols-2">
            {parsed.entries.map(([k, v]) => (
              <div
                key={k}
                className="flex min-w-0 items-baseline gap-2 rounded-md bg-slate-50 px-2 py-1.5"
              >
                <span className="font-mono text-[10px] text-slate-500">{k}</span>
                <span className="truncate text-xs font-medium text-ink-900">
                  {formatValue(v)}
                </span>
              </div>
            ))}
          </div>
        ) : parsed.kind === 'empty' ? (
          <p className="text-xs italic text-slate-400">
            No payload &mdash; your code branches purely on which variant the
            user was assigned.
          </p>
        ) : (
          <p className="text-xs text-amber-700">
            Preview appears once the config is valid JSON.
          </p>
        )}
      </div>
    </div>
  );
}
