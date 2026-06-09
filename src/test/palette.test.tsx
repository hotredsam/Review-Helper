import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { parseHex, toHex, mixHex, contrastRatio, readableTextHex } from "../palette/color";
import { composeIcon, derivedTokens, randomPalette, DEFAULT_PALETTE } from "../palette/generator";
import { THEMES, isThemeId } from "../theme/themes";
import { PalettePane } from "../components/PalettePane";
import { usePaletteStore } from "../store/paletteStore";

describe("palette color utils", () => {
  it("parses and roundtrips hex (3- and 6-digit)", () => {
    expect(toHex(parseHex("#fff")!)).toBe("#ffffff");
    expect(toHex(parseHex("1b2440")!)).toBe("#1b2440");
    expect(parseHex("nope")).toBeNull();
  });

  it("mixes and computes WCAG contrast", () => {
    expect(mixHex("#000000", "#ffffff", 0.5)).toBe("#808080");
    const black = parseHex("#000000")!;
    const white = parseHex("#ffffff")!;
    expect(Math.round(contrastRatio(black, white))).toBe(21);
  });

  it("picks a readable foreground for a background", () => {
    expect(readableTextHex("#000000")).toBe("#ffffff");
    expect(readableTextHex("#ffffff")).not.toBe("#ffffff"); // near-black
  });
});

describe("palette generator", () => {
  it("composeIcon is deterministic per seed", () => {
    const a = composeIcon(DEFAULT_PALETTE, 42, "u");
    const b = composeIcon(DEFAULT_PALETTE, 42, "u");
    expect(JSON.stringify(a)).toBe(JSON.stringify(b));
  });

  it("varies the composition across seeds", () => {
    const shapes = new Set(
      [1, 2, 3, 4, 5, 6, 7, 8].map((s) => JSON.stringify(composeIcon(DEFAULT_PALETTE, s, "u"))),
    );
    expect(shapes.size).toBeGreaterThan(1);
  });

  it("always emits a full-bleed base shape first", () => {
    const comp = composeIcon(DEFAULT_PALETTE, 7, "u");
    expect(comp.shapes.length).toBeGreaterThan(1);
    expect(comp.shapes[0].kind).toBe("rect");
  });

  it("randomPalette yields five valid hex roles", () => {
    const p = randomPalette(123);
    const vals = Object.values(p);
    expect(vals).toHaveLength(5);
    for (const v of vals) expect(/^#[0-9a-f]{6}$/.test(v)).toBe(true);
  });

  it("derivedTokens covers the app token set", () => {
    const t = derivedTokens(DEFAULT_PALETTE);
    for (const k of ["--bg", "--surface", "--fg", "--fg-subtle", "--accent", "--accent-fg", "--ring", "--scrim"]) {
      expect(t[k]).toBeTruthy();
    }
  });
});

describe("theme registry", () => {
  it("includes the four new themes", () => {
    for (const id of ["nord", "forest", "rose", "grape"]) {
      expect(THEMES.some((t) => t.id === id)).toBe(true);
      expect(isThemeId(id)).toBe(true);
    }
  });
});

describe("PalettePane", () => {
  beforeEach(() => {
    usePaletteStore.setState({ palette: DEFAULT_PALETTE, seed: 1, applied: false });
  });

  it("renders the role editors and regenerate changes the seed", () => {
    render(<PalettePane />);
    expect(screen.getByLabelText("Background color")).toBeTruthy();
    expect(screen.getByLabelText("Primary hex value")).toBeTruthy();

    const before = usePaletteStore.getState().seed;
    fireEvent.click(screen.getByRole("button", { name: /Regenerate/i }));
    expect(usePaletteStore.getState().seed).not.toBe(before);
  });
});
