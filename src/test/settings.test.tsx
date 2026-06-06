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
