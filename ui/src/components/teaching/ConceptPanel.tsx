import { useEffect, useMemo, useReducer, useRef, useState } from 'react';
import { Play } from 'lucide-react';

interface DotSpec {
  id: number;
  spawnAt: number;
  lane: 'A' | 'B';
  converted: boolean;
  jitterY: number;
}

const TOTAL_MS = 12_000;
const DOT_LIFETIME_MS = 2_400;
const SPAWN_GAP_MS = 320;
const DOT_COUNT = 30;
// Make B clearly win over A, so the animation tells a story.
const CONVERT_RATE_A = 0.32;
const CONVERT_RATE_B = 0.6;

const VIEW_W = 720;
const VIEW_H = 220;
const SOURCE_X = 60;
const PIPE_END_X = 220;
const VARIANT_X = 420;
const VARIANT_W = 120;
const VARIANT_H = 56;
const LANE_Y_A = 70;
const LANE_Y_B = 150;
const RESULT_X = 600;

function buildDots(): DotSpec[] {
  // Deterministic pseudo-random so animation is identical on every replay.
  let seed = 7;
  const rand = () => {
    seed = (seed * 9301 + 49297) % 233280;
    return seed / 233280;
  };
  const dots: DotSpec[] = [];
  for (let i = 0; i < DOT_COUNT; i++) {
    const lane: 'A' | 'B' = rand() > 0.5 ? 'A' : 'B';
    const convertProb = lane === 'A' ? CONVERT_RATE_A : CONVERT_RATE_B;
    dots.push({
      id: i,
      spawnAt: 400 + i * SPAWN_GAP_MS,
      lane,
      converted: rand() < convertProb,
      jitterY: (rand() - 0.5) * 18,
    });
  }
  return dots;
}

interface DotPosition {
  x: number;
  y: number;
  opacity: number;
  visible: boolean;
  arrived: boolean;
}

function dotPosition(dot: DotSpec, t: number): DotPosition {
  const local = t - dot.spawnAt;
  if (local < 0) return { x: 0, y: 0, opacity: 0, visible: false, arrived: false };
  if (local > DOT_LIFETIME_MS) {
    return { x: 0, y: 0, opacity: 0, visible: false, arrived: true };
  }
  const p = local / DOT_LIFETIME_MS; // 0..1

  // Phase split:
  //  0.00 - 0.18 : drift inside source
  //  0.18 - 0.45 : travel through common pipe
  //  0.45 - 0.85 : branch into lane
  //  0.85 - 1.00 : fade into variant box
  const targetY = dot.lane === 'A' ? LANE_Y_A : LANE_Y_B;
  const sourceY = (LANE_Y_A + LANE_Y_B) / 2 + dot.jitterY;

  let x: number;
  let y: number;
  let opacity = 1;

  if (p < 0.18) {
    const k = p / 0.18;
    x = SOURCE_X + k * 30;
    y = sourceY;
    opacity = k;
  } else if (p < 0.45) {
    const k = (p - 0.18) / 0.27;
    x = SOURCE_X + 30 + k * (PIPE_END_X - (SOURCE_X + 30));
    y = sourceY;
  } else if (p < 0.85) {
    const k = (p - 0.45) / 0.4;
    // Smooth ease for the branch.
    const ease = k < 0.5 ? 2 * k * k : 1 - Math.pow(-2 * k + 2, 2) / 2;
    x = PIPE_END_X + ease * (VARIANT_X - PIPE_END_X);
    y = sourceY + ease * (targetY - sourceY);
  } else {
    const k = (p - 0.85) / 0.15;
    x = VARIANT_X + k * 30;
    y = targetY;
    opacity = 1 - k;
  }

  return { x, y, opacity, visible: true, arrived: false };
}

export function ConceptPanel() {
  const dots = useMemo(buildDots, []);
  const [phase, setPhase] = useState<'playing' | 'paused'>('playing');
  // Coalesce all rAF ticks into a single re-render per frame.
  const [, force] = useReducer((n: number) => n + 1, 0);
  const tRef = useRef(0);
  const startRef = useRef(0);
  const rafRef = useRef<number | null>(null);

  useEffect(() => {
    if (phase !== 'playing') return;
    startRef.current = performance.now() - tRef.current;
    const loop = (now: number) => {
      const elapsed = now - startRef.current;
      tRef.current = elapsed;
      force();
      if (elapsed >= TOTAL_MS) {
        // Settle on final frame, then pause.
        tRef.current = TOTAL_MS;
        force();
        setPhase('paused');
        return;
      }
      rafRef.current = requestAnimationFrame(loop);
    };
    rafRef.current = requestAnimationFrame(loop);
    return () => {
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
    };
  }, [phase]);

  const replay = () => {
    tRef.current = 0;
    setPhase('playing');
  };

  const t = tRef.current;
  const positions = dots.map((d) => ({ dot: d, pos: dotPosition(d, t) }));

  const totalA = dots.filter((d) => d.lane === 'A').length;
  const totalB = dots.filter((d) => d.lane === 'B').length;
  const arrivedA = positions.filter((p) => p.dot.lane === 'A' && p.pos.arrived).length;
  const arrivedB = positions.filter((p) => p.dot.lane === 'B' && p.pos.arrived).length;
  const convA = positions.filter(
    (p) => p.dot.lane === 'A' && p.pos.arrived && p.dot.converted,
  ).length;
  const convB = positions.filter(
    (p) => p.dot.lane === 'B' && p.pos.arrived && p.dot.converted,
  ).length;

  const rateA = arrivedA === 0 ? 0 : convA / arrivedA;
  const rateB = arrivedB === 0 ? 0 : convB / arrivedB;
  const winnerKnown = arrivedA >= 4 && arrivedB >= 4 && t > TOTAL_MS * 0.7;
  const winner = winnerKnown ? (rateB > rateA ? 'B' : 'A') : null;

  const recentA = positions.some(
    (p) =>
      p.dot.lane === 'A' &&
      p.pos.visible &&
      p.pos.x > VARIANT_X - 24 &&
      p.pos.x < VARIANT_X + 30,
  );
  const recentB = positions.some(
    (p) =>
      p.dot.lane === 'B' &&
      p.pos.visible &&
      p.pos.x > VARIANT_X - 24 &&
      p.pos.x < VARIANT_X + 30,
  );

  return (
    <section
      aria-label="How an experiment works"
      className="overflow-hidden rounded-xl border border-brand-100 bg-gradient-to-br from-brand-50/70 via-white to-accent-50/40 shadow-sm"
    >
      <div className="flex flex-col gap-1 px-5 pt-5 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-sm font-semibold text-ink-900">
            How an experiment works
          </h2>
          <p className="mt-0.5 max-w-2xl text-xs leading-relaxed text-slate-600">
            Show different versions of your feature to different users, then
            measure which one performs better.
          </p>
        </div>
        <button
          type="button"
          onClick={replay}
          disabled={phase === 'playing'}
          className="inline-flex h-7 items-center gap-1 self-start rounded-full border border-slate-200 bg-white px-3 text-xs font-medium text-slate-600 transition hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-50 sm:self-auto"
        >
          <Play className="h-3 w-3" />
          {phase === 'playing' ? 'Playing…' : 'Replay'}
        </button>
      </div>

      <div className="px-5 pb-5 pt-3">
        <svg
          viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
          className="block h-auto w-full"
          role="img"
          aria-hidden
        >
          <defs>
            <linearGradient id="cp-pipe" x1="0" x2="1" y1="0" y2="0">
              <stop offset="0" stopColor="#cbd5e1" stopOpacity="0.25" />
              <stop offset="1" stopColor="#cbd5e1" stopOpacity="0.6" />
            </linearGradient>
            <radialGradient id="cp-source" cx="0.5" cy="0.5" r="0.5">
              <stop offset="0" stopColor="#dbeafe" />
              <stop offset="1" stopColor="#dbeafe" stopOpacity="0" />
            </radialGradient>
          </defs>

          {/* Source halo */}
          <circle cx={SOURCE_X + 15} cy={(LANE_Y_A + LANE_Y_B) / 2} r="48" fill="url(#cp-source)" />
          <text
            x={SOURCE_X + 15}
            y={(LANE_Y_A + LANE_Y_B) / 2 - 56}
            textAnchor="middle"
            className="fill-slate-500 text-[11px] font-medium"
          >
            Your users
          </text>

          {/* Common pipe */}
          <line
            x1={SOURCE_X + 30}
            y1={(LANE_Y_A + LANE_Y_B) / 2}
            x2={PIPE_END_X}
            y2={(LANE_Y_A + LANE_Y_B) / 2}
            stroke="url(#cp-pipe)"
            strokeWidth="22"
            strokeLinecap="round"
          />

          {/* Lane connectors */}
          <path
            d={`M ${PIPE_END_X} ${(LANE_Y_A + LANE_Y_B) / 2} C ${PIPE_END_X + 80} ${(LANE_Y_A + LANE_Y_B) / 2}, ${VARIANT_X - 80} ${LANE_Y_A}, ${VARIANT_X} ${LANE_Y_A}`}
            stroke="#e2e8f0"
            strokeWidth="2"
            fill="none"
          />
          <path
            d={`M ${PIPE_END_X} ${(LANE_Y_A + LANE_Y_B) / 2} C ${PIPE_END_X + 80} ${(LANE_Y_A + LANE_Y_B) / 2}, ${VARIANT_X - 80} ${LANE_Y_B}, ${VARIANT_X} ${LANE_Y_B}`}
            stroke="#e2e8f0"
            strokeWidth="2"
            fill="none"
          />

          {/* Variant boxes */}
          <VariantBox
            x={VARIANT_X}
            y={LANE_Y_A - VARIANT_H / 2}
            w={VARIANT_W}
            h={VARIANT_H}
            color="#2a4eff"
            label="Variant A"
            sublabel="control"
            pulsing={recentA}
          />
          <VariantBox
            x={VARIANT_X}
            y={LANE_Y_B - VARIANT_H / 2}
            w={VARIANT_W}
            h={VARIANT_H}
            color="#6e26d9"
            label="Variant B"
            sublabel="new version"
            pulsing={recentB}
          />

          {/* Result panel */}
          <ResultBars
            x={RESULT_X}
            yA={LANE_Y_A}
            yB={LANE_Y_B}
            rateA={rateA}
            rateB={rateB}
            convA={convA}
            convB={convB}
            arrivedA={arrivedA}
            arrivedB={arrivedB}
            totalA={totalA}
            totalB={totalB}
            winner={winner}
          />

          {/* Flowing dots */}
          {positions.map(({ dot, pos }) =>
            pos.visible ? (
              <circle
                key={dot.id}
                cx={pos.x}
                cy={pos.y}
                r="4.5"
                fill={dot.lane === 'A' ? '#2a4eff' : '#6e26d9'}
                opacity={pos.opacity}
              />
            ) : null,
          )}
        </svg>

        <div className="mt-3 grid grid-cols-1 gap-2 text-xs text-slate-600 sm:grid-cols-3">
          <Step number={1} title="Split">
            Each user is bucketed into one variant.
          </Step>
          <Step number={2} title="Show">
            They see that variant&rsquo;s version of your feature.
          </Step>
          <Step number={3} title="Measure">
            Conversion rates tell you which version wins.
          </Step>
        </div>
      </div>
    </section>
  );
}

function Step({
  number,
  title,
  children,
}: {
  number: number;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-start gap-2 rounded-md border border-slate-100 bg-white/70 p-2.5">
      <span className="mt-0.5 inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-brand-100 text-[10px] font-semibold text-brand-700">
        {number}
      </span>
      <div>
        <div className="text-xs font-semibold text-ink-900">{title}</div>
        <div className="text-[11px] leading-snug text-slate-500">{children}</div>
      </div>
    </div>
  );
}

function VariantBox({
  x,
  y,
  w,
  h,
  color,
  label,
  sublabel,
  pulsing,
}: {
  x: number;
  y: number;
  w: number;
  h: number;
  color: string;
  label: string;
  sublabel: string;
  pulsing: boolean;
}) {
  return (
    <g>
      {pulsing ? (
        <rect
          x={x - 4}
          y={y - 4}
          width={w + 8}
          height={h + 8}
          rx="14"
          fill={color}
          opacity="0.12"
        />
      ) : null}
      <rect
        x={x}
        y={y}
        width={w}
        height={h}
        rx="10"
        fill="white"
        stroke={color}
        strokeWidth="1.6"
      />
      <text
        x={x + 14}
        y={y + 22}
        className="text-[12px] font-semibold"
        fill={color}
      >
        {label}
      </text>
      <text x={x + 14} y={y + 40} className="text-[10px]" fill="#64748b">
        {sublabel}
      </text>
    </g>
  );
}

function ResultBars({
  x,
  yA,
  yB,
  rateA,
  rateB,
  convA,
  convB,
  arrivedA,
  arrivedB,
  totalA,
  totalB,
  winner,
}: {
  x: number;
  yA: number;
  yB: number;
  rateA: number;
  rateB: number;
  convA: number;
  convB: number;
  arrivedA: number;
  arrivedB: number;
  totalA: number;
  totalB: number;
  winner: 'A' | 'B' | null;
}) {
  const maxRate = Math.max(rateA, rateB, 0.6);
  const barMaxW = 80;
  const wA = maxRate === 0 ? 0 : (rateA / maxRate) * barMaxW;
  const wB = maxRate === 0 ? 0 : (rateB / maxRate) * barMaxW;

  return (
    <g>
      <text
        x={x}
        y={yA - 28}
        className="fill-slate-500 text-[11px] font-medium"
      >
        Conversion rate
      </text>

      {/* A */}
      <rect x={x} y={yA - 8} width={barMaxW} height="8" rx="4" fill="#e2e8f0" />
      <rect
        x={x}
        y={yA - 8}
        width={wA}
        height="8"
        rx="4"
        fill="#2a4eff"
        style={{ transition: 'width 240ms ease-out' }}
      />
      <text x={x} y={yA + 14} className="fill-ink-900 text-[11px] font-semibold">
        {(rateA * 100).toFixed(0)}%
      </text>
      <text x={x + 36} y={yA + 14} className="fill-slate-500 text-[10px]">
        {convA}/{arrivedA} of {totalA}
      </text>

      {/* B */}
      <rect x={x} y={yB - 8} width={barMaxW} height="8" rx="4" fill="#e2e8f0" />
      <rect
        x={x}
        y={yB - 8}
        width={wB}
        height="8"
        rx="4"
        fill="#6e26d9"
        style={{ transition: 'width 240ms ease-out' }}
      />
      <text x={x} y={yB + 14} className="fill-ink-900 text-[11px] font-semibold">
        {(rateB * 100).toFixed(0)}%
      </text>
      <text x={x + 36} y={yB + 14} className="fill-slate-500 text-[10px]">
        {convB}/{arrivedB} of {totalB}
      </text>

      {/* Winner pill */}
      {winner ? (
        <g
          style={{
            opacity: 1,
            transition: 'opacity 400ms ease',
          }}
        >
          <rect
            x={x - 4}
            y={(winner === 'A' ? yA : yB) - 30}
            width="76"
            height="22"
            rx="11"
            fill="#10b981"
          />
          <text
            x={x + 34}
            y={(winner === 'A' ? yA : yB) - 15}
            textAnchor="middle"
            className="fill-white text-[10px] font-semibold"
          >
            ★ Winner
          </text>
        </g>
      ) : null}
    </g>
  );
}

