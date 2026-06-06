import { describe, it, expect, beforeEach, vi } from "vitest";

// A tiny event bus standing in for the Rust model stream. `runModel` replays a
// scripted sequence of events to the registered listener.
const bus = vi.hoisted(() => ({
  handler: null as null | ((e: any) => void),
  script: [] as any[],
  calls: [] as Array<{ prompt: string; sessionId: string | null }>,
}));

vi.mock("../api/model", () => ({
  runModel: vi.fn(async (prompt: string, sessionId?: string | null) => {
    bus.calls.push({ prompt, sessionId: sessionId ?? null });
    for (const e of bus.script) bus.handler?.(e);
  }),
  onModelEvent: vi.fn(async (h: (e: any) => void) => {
    bus.handler = h;
    return () => {};
  }),
}));

import { useModelStore, ensureModelListener } from "../store/modelStore";

beforeEach(() => {
  // Note: don't reset bus.handler — the global listener is wired once and reused.
  bus.script = [];
  bus.calls = [];
  useModelStore.setState({
    turns: [],
    sessionId: null,
    streaming: false,
    error: null,
    unavailable: null,
    tools: [],
  });
});

describe("modelStore streaming", () => {
  it("accumulates streamed deltas into the assistant turn and captures the session", async () => {
    ensureModelListener();
    bus.script = [
      { type: "started", session_id: "sess-1", model: "haiku" },
      { type: "assistant_text", text: "po" },
      { type: "assistant_text", text: "ng" },
      { type: "completed", session_id: "sess-1", text: "pong" },
    ];

    await useModelStore.getState().send("ping");

    const s = useModelStore.getState();
    expect(s.turns).toEqual([
      { role: "user", text: "ping" },
      { role: "assistant", text: "pong" },
    ]);
    expect(s.sessionId).toBe("sess-1");
    expect(s.streaming).toBe(false);
  });

  it("resumes the session on a follow-up send", async () => {
    ensureModelListener();
    bus.script = [
      { type: "started", session_id: "sess-1", model: "haiku" },
      { type: "assistant_text", text: "ping" },
      { type: "completed", session_id: "sess-1", text: "ping" },
    ];
    await useModelStore.getState().send("first");

    bus.script = [
      { type: "started", session_id: "sess-1", model: "haiku" },
      { type: "assistant_text", text: "pong" },
      { type: "completed", session_id: "sess-1", text: "pong" },
    ];
    await useModelStore.getState().send("second");

    // The follow-up carried the captured session id, so Claude resumes it.
    expect(bus.calls[1]).toEqual({ prompt: "second", sessionId: "sess-1" });
    expect(useModelStore.getState().turns).toHaveLength(4);
  });

  it("surfaces an unavailable terminal event and stops streaming", async () => {
    ensureModelListener();
    bus.script = [{ type: "unavailable", reason: "not_installed", detail: "claude not found" }];

    await useModelStore.getState().send("hi");

    const s = useModelStore.getState();
    expect(s.streaming).toBe(false);
    expect(s.unavailable).toEqual({ reason: "not_installed", detail: "claude not found" });
  });
});
