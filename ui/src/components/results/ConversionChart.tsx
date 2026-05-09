import { useEffect, useMemo, useRef, useState } from 'react';
import type { VariantResult } from '@/api/types';
import { variantColorByKey } from '@/lib/variantColors';

export interface ConversionChartProps {
  variants: VariantResult[];
  /** Stable order to keep colors consistent with the rest of the UI. */
  variantKeyOrder: readonly string[];
}

const W_DEFAULT = 1000;
const W_MIN = 320;
const ROW_H = 64;
const BAR_H = 22;
const LEFT_PAD = 168; // room for the variant label on the left
const RIGHT_PAD = 80; // room for the rate label on the right
const TOP_PAD = 20;
const BOTTOM_PAD = 28;

function pickAxisMax(variants: VariantResult[]): number {
  // Choose the upper bound a little above the largest CI high or rate, with a
  // floor of 1% so a low-rate experiment doesn't render as one giant bar.
  let raw = 0;
  for (const v of variants) {
    if (v.ci95) raw = Math.max(raw, v.ci95[1]);
    else if (v.conversionRate !== null) raw = Math.max(raw, v.conversionRate);
  }
  if (raw <= 0) return 0.01;
  // Round up to a nice number: pick from a ladder.
  const ladder = [
    0.005, 0.01, 0.02, 0.05, 0.1, 0.15, 0.2, 0.3, 0.4, 0.5, 0.75, 1.0,
  ];
  for (const v of ladder) {
    if (raw * 1.15 <= v) return v;
  }
  return 1.0;
}

function formatPct(n: number, digits = 2): string {
  return `${(n * 100).toFixed(digits)}%`;
}

export function ConversionChart({ variants, variantKeyOrder }: ConversionChartProps) {
  // Animate bar widths in on mount.
  const [mounted, setMounted] = useState(false);
  useEffect(() => {
    const id = requestAnimationFrame(() => setMounted(true));
    return () => cancelAnimationFrame(id);
  }, []);

  const containerRef = useRef<HTMLDivElement>(null);
  const [W, setW] = useState(W_DEFAULT);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const w = entry.contentRect.width;
        if (w > 0) setW(Math.max(W_MIN, w));
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const axisMax = useMemo(() => pickAxisMax(variants), [variants]);
  const height = TOP_PAD + variants.length * ROW_H + BOTTOM_PAD;
  const barAreaW = W - LEFT_PAD - RIGHT_PAD;

  // X-axis tick positions: 0, 25%, 50%, 75%, 100% of axisMax.
  const ticks = [0, 0.25, 0.5, 0.75, 1.0].map((t) => ({
    frac: t,
    value: t * axisMax,
  }));

  return (
    <div className="overflow-hidden rounded-lg border border-slate-200 bg-white">
      <div className="border-b border-slate-200 px-5 py-3.5">
        <h2 className="text-base font-semibold text-slate-900">
          Conversion rate by variant
        </h2>
        <p className="mt-0.5 text-sm text-slate-500">
          Bars show the observed rate. The horizontal whisker marks the Wilson
          95% confidence interval — overlapping whiskers mean the difference
          might just be noise.
        </p>
      </div>
      <div ref={containerRef} className="px-3 pb-2 pt-3">
        <svg
          viewBox={`0 0 ${W} ${height}`}
          width="100%"
          height={height}
          role="img"
          aria-label="Conversion rate by variant"
          preserveAspectRatio="xMidYMid meet"
        >
          {/* Grid + tick labels */}
          {ticks.map((t) => {
            const x = LEFT_PAD + barAreaW * t.frac;
            return (
              <g key={t.frac}>
                <line
                  x1={x}
                  x2={x}
                  y1={TOP_PAD - 6}
                  y2={height - BOTTOM_PAD + 4}
                  stroke="#e2e8f0"
                  strokeWidth={1}
                  strokeDasharray={t.frac === 0 ? undefined : '3 3'}
                />
                <text
                  x={x}
                  y={height - 8}
                  textAnchor="middle"
                  className="fill-slate-500"
                  style={{ fontSize: 11 }}
                >
                  {formatPct(t.value, t.value < 0.01 ? 2 : 1)}
                </text>
              </g>
            );
          })}

          {variants.map((v, i) => {
            const color = variantColorByKey(variantKeyOrder, v.variantKey);
            const rowY = TOP_PAD + i * ROW_H;
            const barCenterY = rowY + ROW_H / 2;

            const rate = v.conversionRate ?? 0;
            const ratePx =
              LEFT_PAD + barAreaW * Math.min(1, rate / axisMax);

            const ciLow = v.ci95?.[0] ?? rate;
            const ciHigh = v.ci95?.[1] ?? rate;
            const ciLowPx =
              LEFT_PAD + barAreaW * Math.min(1, ciLow / axisMax);
            const ciHighPx =
              LEFT_PAD + barAreaW * Math.min(1, ciHigh / axisMax);

            const noData = v.exposures === 0;

            return (
              <g key={v.variantKey}>
                {/* Row label */}
                <text
                  x={LEFT_PAD - 12}
                  y={barCenterY - 4}
                  textAnchor="end"
                  className="fill-slate-900"
                  style={{ fontSize: 13, fontWeight: 600 }}
                >
                  {v.variantKey}
                </text>
                <text
                  x={LEFT_PAD - 12}
                  y={barCenterY + 12}
                  textAnchor="end"
                  className="fill-slate-500"
                  style={{ fontSize: 11 }}
                >
                  {v.isControl ? 'control · ' : ''}
                  {v.exposures.toLocaleString()} exposures
                </text>

                {/* Track */}
                <rect
                  x={LEFT_PAD}
                  y={barCenterY - BAR_H / 2}
                  width={barAreaW}
                  height={BAR_H}
                  rx={4}
                  fill="#f1f5f9"
                />

                {!noData ? (
                  <>
                    {/* Bar (animated width via transform). */}
                    <rect
                      x={LEFT_PAD}
                      y={barCenterY - BAR_H / 2}
                      width={mounted ? Math.max(2, ratePx - LEFT_PAD) : 2}
                      height={BAR_H}
                      rx={4}
                      fill={color.hex}
                      style={{
                        transition: 'width 700ms cubic-bezier(0.22, 1, 0.36, 1)',
                      }}
                    />
                    {/* CI whisker */}
                    {v.ci95 ? (
                      <g
                        opacity={mounted ? 1 : 0}
                        style={{ transition: 'opacity 600ms ease 350ms' }}
                      >
                        <line
                          x1={ciLowPx}
                          x2={ciHighPx}
                          y1={barCenterY}
                          y2={barCenterY}
                          stroke="#0f172a"
                          strokeWidth={1.5}
                        />
                        <line
                          x1={ciLowPx}
                          x2={ciLowPx}
                          y1={barCenterY - 8}
                          y2={barCenterY + 8}
                          stroke="#0f172a"
                          strokeWidth={1.5}
                        />
                        <line
                          x1={ciHighPx}
                          x2={ciHighPx}
                          y1={barCenterY - 8}
                          y2={barCenterY + 8}
                          stroke="#0f172a"
                          strokeWidth={1.5}
                        />
                        <circle
                          cx={ratePx}
                          cy={barCenterY}
                          r={4}
                          fill="white"
                          stroke="#0f172a"
                          strokeWidth={1.5}
                        />
                      </g>
                    ) : null}

                    {/* Right-side rate label */}
                    <text
                      x={W - RIGHT_PAD + 8}
                      y={barCenterY + 4}
                      className="fill-slate-900"
                      style={{ fontSize: 13, fontWeight: 600 }}
                    >
                      {formatPct(rate)}
                    </text>
                  </>
                ) : (
                  <text
                    x={LEFT_PAD + 12}
                    y={barCenterY + 4}
                    className="fill-slate-400"
                    style={{ fontSize: 12, fontStyle: 'italic' }}
                  >
                    No exposures yet
                  </text>
                )}
              </g>
            );
          })}
        </svg>
      </div>
    </div>
  );
}
