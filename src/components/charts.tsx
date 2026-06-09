//! Fixed visualization components — pure SVG/DOM driven by real data, themed via
//! CSS-var-backed tokens (no hardcoded colors), so they work in all four themes.
//! No runtime LLM-generated UI.

/** Score → semantic color token. 75/50 breakpoints match StatePane's `tint`
 *  so the same score never reads as two different colors across the app. */
function tintText(v: number): string {
  return v >= 75 ? "text-success" : v >= 50 ? "text-warning" : "text-danger";
}

const clamp = (v: number, lo = 0, hi = 100) => Math.max(lo, Math.min(hi, v));

/** Radar of N labeled 0–100 axes (e.g. the assessment dimensions). Pass
 *  showLabels=false when an adjacent list already labels the axes. */
export function RadarChart({
  axes,
  size = 220,
  showLabels = true,
  ariaLabel,
}: {
  axes: { label: string; value: number }[];
  size?: number;
  showLabels?: boolean;
  ariaLabel?: string;
}) {
  const cx = size / 2;
  const cy = size / 2;
  const r = Math.max(1, size / 2 - 34);
  const n = Math.max(axes.length, 1);
  const pt = (i: number, frac: number): [number, number] => {
    const ang = -Math.PI / 2 + (i * 2 * Math.PI) / n;
    return [cx + Math.cos(ang) * r * frac, cy + Math.sin(ang) * r * frac];
  };
  const ring = (f: number) => axes.map((_, j) => pt(j, f).join(",")).join(" ");
  const data = axes.map((a, i) => pt(i, clamp(a.value) / 100).join(",")).join(" ");

  return (
    <svg viewBox={`0 0 ${size} ${size}`} role="img" aria-label={ariaLabel ?? "Dimension scores"} className="text-accent">
      {[0.25, 0.5, 0.75, 1].map((f, i) => (
        <polygon key={i} points={ring(f)} className="fill-none stroke-border" strokeWidth={1} />
      ))}
      {axes.map((a, i) => {
        const [x, y] = pt(i, 1);
        const [lx, ly] = pt(i, 1.16);
        return (
          <g key={a.label}>
            <line x1={cx} y1={cy} x2={x} y2={y} className="stroke-border" strokeWidth={1} />
            {showLabels && (
              <text x={lx} y={ly} className="fill-fg-subtle" fontSize={8} textAnchor="middle" dominantBaseline="middle">
                {a.label}
              </text>
            )}
          </g>
        );
      })}
      <polygon points={data} fill="currentColor" fillOpacity={0.25} stroke="currentColor" strokeWidth={2} />
    </svg>
  );
}

/** Half-circle gauge for a single 0–100 score, tinted by value. */
export function Gauge({ value, label, size = 132 }: { value: number; label: string; size?: number }) {
  const v = Math.round(clamp(value));
  const cx = size / 2;
  const cy = size / 2;
  const r = Math.max(1, size / 2 - 12);
  // sweep flag 0 → the arc bulges UP (y < cy), fitting the half-height viewBox.
  const arc = `M ${cx - r} ${cy} A ${r} ${r} 0 0 0 ${cx + r} ${cy}`;
  const circ = Math.PI * r;
  return (
    <svg viewBox={`0 0 ${size} ${size / 2 + 22}`} role="img" aria-label={`${label}: ${v} of 100`} className={tintText(v)}>
      <path d={arc} className="fill-none stroke-surface-2" strokeWidth={10} strokeLinecap="round" />
      <path
        d={arc}
        fill="none"
        stroke="currentColor"
        strokeWidth={10}
        strokeLinecap="round"
        strokeDasharray={`${(v / 100) * circ} ${circ}`}
      />
      <text x={cx} y={cy - 2} className="fill-fg" fontSize={20} fontWeight={600} textAnchor="middle">
        {v}
      </text>
      <text x={cx} y={cy + 13} className="fill-fg-subtle" fontSize={9} textAnchor="middle">
        {label}
      </text>
    </svg>
  );
}

/** Full-circle score ring for a single 0–100 score, tinted by value. Used for
 *  the overall score so it reads as a complete circle (not a clipped half). */
export function ScoreRing({ value, label, size = 120 }: { value: number; label: string; size?: number }) {
  const v = Math.round(clamp(value));
  const cx = size / 2;
  const cy = size / 2;
  const r = Math.max(1, size / 2 - 10);
  const circ = 2 * Math.PI * r;
  return (
    <svg viewBox={`0 0 ${size} ${size}`} role="img" aria-label={`${label}: ${v} of 100`} className={tintText(v)}>
      <circle cx={cx} cy={cy} r={r} className="fill-none stroke-surface-2" strokeWidth={9} />
      <circle
        cx={cx}
        cy={cy}
        r={r}
        fill="none"
        stroke="currentColor"
        strokeWidth={9}
        strokeLinecap="round"
        strokeDasharray={`${(v / 100) * circ} ${circ}`}
        transform={`rotate(-90 ${cx} ${cy})`}
      />
      <text x={cx} y={cy - 3} className="fill-fg" fontSize={22} fontWeight={700} textAnchor="middle" dominantBaseline="middle">
        {v}
      </text>
      <text x={cx} y={cy + 16} className="fill-fg-subtle" fontSize={9} textAnchor="middle" dominantBaseline="middle">
        {label}
      </text>
    </svg>
  );
}

/** Labeled horizontal progress bar. */
export function ProgressBar({ value, max = 100, label }: { value: number; max?: number; label: string }) {
  const pct = max > 0 && Number.isFinite(value) ? Math.round(clamp((value / max) * 100)) : 0;
  return (
    <div>
      <div className="mb-0.5 flex justify-between text-xs text-fg-subtle">
        <span>{label}</span>
        <span>
          {value}/{max}
        </span>
      </div>
      <div
        className="h-2 overflow-hidden rounded-full bg-surface-2"
        role="progressbar"
        aria-label={label}
        aria-valuenow={pct}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div className="h-full rounded-full bg-accent transition-all" style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}

/** Donut ring showing value/max as a percentage. */
export function Donut({ value, max, label, size = 104 }: { value: number; max: number; label: string; size?: number }) {
  const frac = max > 0 && Number.isFinite(value) ? clamp(value / max, 0, 1) : 0;
  const cx = size / 2;
  const cy = size / 2;
  const r = Math.max(1, size / 2 - 9);
  const circ = 2 * Math.PI * r;
  return (
    <svg viewBox={`0 0 ${size} ${size}`} role="img" aria-label={`${label}: ${value} of ${max}`} className="text-accent">
      <circle cx={cx} cy={cy} r={r} className="fill-none stroke-surface-2" strokeWidth={8} />
      <circle
        cx={cx}
        cy={cy}
        r={r}
        fill="none"
        stroke="currentColor"
        strokeWidth={8}
        strokeLinecap="round"
        strokeDasharray={`${frac * circ} ${circ}`}
        transform={`rotate(-90 ${cx} ${cy})`}
      />
      <text x={cx} y={cy} className="fill-fg" fontSize={16} fontWeight={600} textAnchor="middle" dominantBaseline="middle">
        {Math.round(frac * 100)}%
      </text>
    </svg>
  );
}
