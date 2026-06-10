import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const store = vi.hoisted(() => ({
  config: { provider: "claude", local_endpoint: null, api_credit_overflow: false } as any,
  saved: [] as any[],
}));

vi.mock("../api/settings", () => ({
  getModelConfig: vi.fn(async () => ({ ...store.config })),
  setModelConfig: vi.fn(async (c: any) => {
    store.config = c;
    store.saved.push(c);
  }),
}));

import { ProviderSettings } from "../components/ProviderSettings";

beforeEach(() => {
  store.config = { provider: "claude", local_endpoint: null, api_credit_overflow: false };
  store.saved = [];
});

describe("ProviderSettings", () => {
  it("defaults to Claude and persists a switch to Local with the stub note", async () => {
    const user = userEvent.setup();
    render(<ProviderSettings />);

    const claude = await screen.findByRole("radio", { name: /Claude Code/i });
    expect(claude.getAttribute("aria-checked")).toBe("true");

    await user.click(screen.getByRole("radio", { name: /Local/i }));

    expect(store.saved[store.saved.length - 1].provider).toBe("local");
    await screen.findByText(/Stub in v1/i); // local endpoint + stub notice appears
  });

  it("persists the api-credit overflow toggle", async () => {
    const user = userEvent.setup();
    render(<ProviderSettings />);
    await screen.findByRole("radio", { name: /Claude Code/i });

    const checkbox = screen.getByRole("checkbox") as HTMLInputElement;
    expect(checkbox.checked).toBe(false);
    await user.click(checkbox);

    expect(store.saved[store.saved.length - 1].api_credit_overflow).toBe(true);
  });
});

describe("ProviderSettings stub notice (Phase 17)", () => {
  it("warns that Local is a stub when local is the active provider", async () => {
    const { getModelConfig } = await import("../api/settings");
    vi.mocked(getModelConfig).mockResolvedValueOnce({
      provider: "local",
      local_endpoint: null,
      api_credit_overflow: false,
    } as any);
    const { ProviderSettings } = await import("../components/ProviderSettings");
    const { render, screen } = await import("@testing-library/react");
    render(<ProviderSettings />);
    expect(await screen.findByRole("note")).toHaveTextContent(/local provider is a stub/i);
    expect(screen.getByRole("note")).toHaveTextContent(/nothing spends Claude credits/i);
  });
});

vi.mock("../api/profile", () => ({
  profileGet: vi.fn(async () => ({
    enabled: true,
    unreflected_events: 7,
    files: [
      { name: "learner-profile.md", content: "# How you learn\nstuff\n## Your notes (never auto-edited)\nmy note" },
      { name: "review-preferences.md", content: "# Reviews\nstuff\n## Your notes (never auto-edited)\n" },
    ],
  })),
  profileSetEnabled: vi.fn(async () => {}),
  profileSaveNotes: vi.fn(async () => {}),
  profileReset: vi.fn(async () => {}),
  profileReflect: vi.fn(async () => "skipped"),
}));

describe("ProfileSettings (Phase 20)", () => {
  it("renders both profile files, the toggle, and preserved notes", async () => {
    const { ProfileSettings } = await import("../components/ProfileSettings");
    const { render, screen } = await import("@testing-library/react");
    render(<ProfileSettings />);
    expect(await screen.findByText(/How you learn \(Learning mode\)/)).toBeTruthy();
    expect(screen.getByText(/How you like reviews/)).toBeTruthy();
    expect(screen.getByLabelText("Adaptive profile enabled")).toBeChecked();
    expect(screen.getByText(/7 new signals/)).toBeTruthy();
    expect(screen.getByDisplayValue("my note")).toBeTruthy();
  });

  it("toggling off persists through the API", async () => {
    const { ProfileSettings } = await import("../components/ProfileSettings");
    const { profileSetEnabled } = await import("../api/profile");
    const { render, screen } = await import("@testing-library/react");
    const { default: userEvent } = await import("@testing-library/user-event");
    const user = userEvent.setup();
    render(<ProfileSettings />);
    await user.click(await screen.findByLabelText("Adaptive profile enabled"));
    expect(vi.mocked(profileSetEnabled)).toHaveBeenCalledWith(false);
  });
});
