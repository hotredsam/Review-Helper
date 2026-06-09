import { useId } from "react";
import { composeIcon, type LinearGradient, type Palette, type Shape } from "../palette/generator";

interface Props {
  palette: Palette;
  seed: number;
  size?: number;
  /** Accessible label; defaults to a generic one. */
  label?: string;
}

/** Convert a gradient angle (deg) into userSpace endpoint coords on a 0–100 box. */
function gradientCoords(angle: number) {
  const rad = (angle * Math.PI) / 180;
  const x = Math.cos(rad);
  const y = Math.sin(rad);
  return { x1: 50 - x * 50, y1: 50 - y * 50, x2: 50 + x * 50, y2: 50 + y * 50 };
}

function Def({ g }: { g: LinearGradient }) {
  const { x1, y1, x2, y2 } = gradientCoords(g.angle);
  return (
    <linearGradient id={g.id} gradientUnits="userSpaceOnUse" x1={x1} y1={y1} x2={x2} y2={y2}>
      {g.stops.map((s, i) => (
        <stop key={i} offset={s.offset} stopColor={s.color} />
      ))}
    </linearGradient>
  );
}

function ShapeEl({ s }: { s: Shape }) {
  if (s.kind === "rect") {
    return <rect x={s.x} y={s.y} width={s.w} height={s.h} rx={s.rx ?? 0} fill={s.fill} />;
  }
  if (s.kind === "circle") {
    return <circle cx={s.cx} cy={s.cy} r={s.r} fill={s.fill} />;
  }
  return <polygon points={s.points.map(([x, y]) => `${x},${y}`).join(" ")} fill={s.fill} />;
}

/**
 * The "generative UI box": a simplistic, app-icon-style composition that
 * distributes the palette colors across a seeded template. Rendered as a
 * rounded-square (squircle-ish) SVG with a subtle glossy highlight, so you can
 * judge how the colors sit together — Apple-app-logo level of detail, not a
 * full mockup.
 */
export function PaletteIcon({ palette, seed, size = 200, label }: Props) {
  const uid = useId().replace(/:/g, "");
  const comp = composeIcon(palette, seed, uid);
  const clipId = `${uid}-clip`;
  const sheenId = `${uid}-sheen`;
  const rx = 24; // ~Apple app-icon corner radius on a 100-unit box

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 100 100"
      role="img"
      aria-label={label ?? "Generated palette icon"}
      className="block drop-shadow-lg"
    >
      <defs>
        <clipPath id={clipId}>
          <rect x="0" y="0" width="100" height="100" rx={rx} ry={rx} />
        </clipPath>
        {comp.defs.map((g) => (
          <Def key={g.id} g={g} />
        ))}
        <linearGradient id={sheenId} gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2="100">
          <stop offset="0" stopColor="#ffffff" stopOpacity="0.22" />
          <stop offset="0.45" stopColor="#ffffff" stopOpacity="0" />
        </linearGradient>
      </defs>
      <g clipPath={`url(#${clipId})`}>
        {comp.shapes.map((s, i) => (
          <ShapeEl key={i} s={s} />
        ))}
        {/* glossy app-icon sheen */}
        <rect x="0" y="0" width="100" height="100" fill={`url(#${sheenId})`} />
        {/* hairline inner border for definition in every theme */}
        <rect
          x="0.5"
          y="0.5"
          width="99"
          height="99"
          rx={rx}
          ry={rx}
          fill="none"
          stroke="#000000"
          strokeOpacity="0.12"
        />
      </g>
    </svg>
  );
}
