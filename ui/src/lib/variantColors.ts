// Shared color palette for variant visualizations. Components that visualize
// the same variant by index (or by key relative to a stable list) get the same
// color, which is what makes the funnel, variant cards, and stacked bar feel
// connected.

export interface VariantColor {
  /** SVG / inline-style hex. */
  hex: string;
  /** Solid pill background. */
  bg: string;
  /** Soft pill background. */
  soft: string;
  /** Text color for use on `soft`. */
  softText: string;
  /** Border / ring on soft pill. */
  ring: string;
}

const PALETTE: VariantColor[] = [
  {
    hex: '#2a4eff',
    bg: 'bg-brand-600',
    soft: 'bg-brand-50',
    softText: 'text-brand-700',
    ring: 'ring-brand-200',
  },
  {
    hex: '#6e26d9',
    bg: 'bg-accent-600',
    soft: 'bg-accent-50',
    softText: 'text-accent-700',
    ring: 'ring-accent-200',
  },
  {
    hex: '#059669',
    bg: 'bg-emerald-600',
    soft: 'bg-emerald-50',
    softText: 'text-emerald-700',
    ring: 'ring-emerald-200',
  },
  {
    hex: '#d97706',
    bg: 'bg-amber-600',
    soft: 'bg-amber-50',
    softText: 'text-amber-800',
    ring: 'ring-amber-200',
  },
  {
    hex: '#e11d48',
    bg: 'bg-rose-600',
    soft: 'bg-rose-50',
    softText: 'text-rose-700',
    ring: 'ring-rose-200',
  },
  {
    hex: '#0891b2',
    bg: 'bg-cyan-600',
    soft: 'bg-cyan-50',
    softText: 'text-cyan-700',
    ring: 'ring-cyan-200',
  },
];

const FALLBACK = PALETTE[0]!;

export function variantColor(index: number): VariantColor {
  if (index < 0) return FALLBACK;
  return PALETTE[index % PALETTE.length] ?? FALLBACK;
}

export function variantColorByKey(
  variantKeys: readonly string[],
  key: string,
): VariantColor {
  const idx = variantKeys.indexOf(key);
  return variantColor(idx === -1 ? 0 : idx);
}
