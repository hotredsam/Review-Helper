/**
 * Small, dependency-free color utilities for the palette planner: hex parsing,
 * sRGB mixing, WCAG luminance/contrast, readable-foreground choice, and HSL→hex
 * (for generating harmonious random palettes). Pure functions, easy to test.
 */

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

const HEX_RE = /^#?([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;

export function isHex(s: string): boolean {
  return HEX_RE.test(s.trim());
}

export function parseHex(input: string): Rgb | null {
  const s = input.trim().replace(/^#/, "");
  if (s.length === 3) {
    return {
      r: parseInt(s[0] + s[0], 16),
      g: parseInt(s[1] + s[1], 16),
      b: parseInt(s[2] + s[2], 16),
    };
  }
  if (s.length === 6) {
    return {
      r: parseInt(s.slice(0, 2), 16),
      g: parseInt(s.slice(2, 4), 16),
      b: parseInt(s.slice(4, 6), 16),
    };
  }
  return null;
}

const clampByte = (n: number) => Math.max(0, Math.min(255, Math.round(n)));

export function toHex(c: Rgb): string {
  const h = (n: number) => clampByte(n).toString(16).padStart(2, "0");
  return `#${h(c.r)}${h(c.g)}${h(c.b)}`;
}

/** Linear sRGB mix; t=0 → a, t=1 → b (clamped to [0,1]). */
export function mix(a: Rgb, b: Rgb, t: number): Rgb {
  const k = Math.max(0, Math.min(1, t));
  return {
    r: a.r + (b.r - a.r) * k,
    g: a.g + (b.g - a.g) * k,
    b: a.b + (b.b - a.b) * k,
  };
}

/** Mix two hex colors; unparseable inputs fall back to black so this never throws. */
export function mixHex(a: string, b: string, t: number): string {
  const ca = parseHex(a) ?? { r: 0, g: 0, b: 0 };
  const cb = parseHex(b) ?? { r: 0, g: 0, b: 0 };
  return toHex(mix(ca, cb, t));
}

function channelLinear(byte: number): number {
  const c = byte / 255;
  return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

export function relativeLuminance(c: Rgb): number {
  return 0.2126 * channelLinear(c.r) + 0.7152 * channelLinear(c.g) + 0.0722 * channelLinear(c.b);
}

/** WCAG contrast ratio (1–21) between two colors. */
export function contrastRatio(a: Rgb, b: Rgb): number {
  const la = relativeLuminance(a);
  const lb = relativeLuminance(b);
  const hi = Math.max(la, lb);
  const lo = Math.min(la, lb);
  return (hi + 0.05) / (lo + 0.05);
}

const NEAR_WHITE: Rgb = { r: 255, g: 255, b: 255 };
const NEAR_BLACK: Rgb = { r: 17, g: 18, b: 23 };

/** Whichever of near-white / near-black reads best on `bgHex`, as a hex string. */
export function readableTextHex(bgHex: string): string {
  const bg = parseHex(bgHex) ?? NEAR_BLACK;
  return contrastRatio(bg, NEAR_WHITE) >= contrastRatio(bg, NEAR_BLACK)
    ? toHex(NEAR_WHITE)
    : toHex(NEAR_BLACK);
}

/** A rough "is this a light color" test (luminance over a mid threshold). */
export function isLight(hex: string): boolean {
  const c = parseHex(hex);
  return c ? relativeLuminance(c) > 0.45 : false;
}

/** HSL (h 0–360, s/l 0–1) → hex. Used to build harmonious random palettes. */
export function hslToHex(h: number, s: number, l: number): string {
  const hue = ((h % 360) + 360) % 360;
  const sat = Math.max(0, Math.min(1, s));
  const lum = Math.max(0, Math.min(1, l));
  const c = (1 - Math.abs(2 * lum - 1)) * sat;
  const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
  const m = lum - c / 2;
  let r = 0;
  let g = 0;
  let b = 0;
  if (hue < 60) [r, g, b] = [c, x, 0];
  else if (hue < 120) [r, g, b] = [x, c, 0];
  else if (hue < 180) [r, g, b] = [0, c, x];
  else if (hue < 240) [r, g, b] = [0, x, c];
  else if (hue < 300) [r, g, b] = [x, 0, c];
  else [r, g, b] = [c, 0, x];
  return toHex({ r: (r + m) * 255, g: (g + m) * 255, b: (b + m) * 255 });
}
