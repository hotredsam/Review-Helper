import { describe, it, expect, beforeEach, vi } from "vitest";

const ctrl = vi.hoisted(() => ({ cb: null as null | ((e: any) => void), pending: [] as any[] }));

vi.mock("../api/chat", () => ({
  chatSend: vi.fn(async () => {}),
  chatNew: vi.fn(async () => 100),
  chatTranscripts: vi.fn(async () => []),
  chatMessages: vi.fn(async () => []),
  chatDelete: vi.fn(async () => {}),
  onChatEvent: vi.fn(async (cb: (e: any) => void) => {
    ctrl.cb = cb;
    return () => {};
  }),
}));
vi.mock("../api/suggestions", () => ({
  suggestionsList: vi.fn(async () => ctrl.pending),
}));

import { useChatStore, ensureChatListener } from "../store/chatStore";
import { chatSend } from "../api/chat";
import { suggestionsList } from "../api/suggestions";

beforeEach(() => {
  useChatStore.setState({ transcripts: {}, activeId: {}, messages: {}, status: {}, error: {}, pending: {} });
  ctrl.pending = [];
  vi.clearAllMocks();
});

const flush = () => new Promise((r) => setTimeout(r, 0));

describe("chat store (persisted transcripts)", () => {
  it("opens a transcript, streams a turn, and routes by transcript id", async () => {
    ensureChatListener();
    await useChatStore.getState().send(7, "What's my stack?");

    // a transcript was created (chat_new) and made active
    const tid = useChatStore.getState().activeId[7];
    expect(tid).toBe(100);
    const msgs = useChatStore.getState().messages[100];
    expect(msgs.map((m) => m.role)).toEqual(["user", "assistant"]);
    expect(msgs[0].text).toBe("What's my stack?");
    expect(useChatStore.getState().status[7]).toBe("streaming");
    expect(vi.mocked(chatSend)).toHaveBeenCalledWith(7, 100, "What's my stack?");

    // tokens stream into the assistant bubble, keyed by transcript_id
    ctrl.cb!({ type: "token", project_id: 7, transcript_id: 100, text: "You use " });
    ctrl.cb!({ type: "token", project_id: 7, transcript_id: 100, text: "SQLite." });
    expect(useChatStore.getState().messages[100][1].text).toBe("You use SQLite.");

    // done finalizes the reply
    ctrl.cb!({ type: "done", project_id: 7, transcript_id: 100, reply: "You use SQLite.", suggestions: 0 });
    expect(useChatStore.getState().messages[100][1].streaming).toBe(false);
    expect(useChatStore.getState().status[7]).toBe("idle");
  });

  it("surfaces a failure without a silent spinner", async () => {
    ensureChatListener();
    await useChatStore.getState().send(8, "hi");
    const tid = useChatStore.getState().activeId[8]!;
    ctrl.cb!({ type: "failed", project_id: 8, transcript_id: tid, detail: "Claude not available" });
    expect(useChatStore.getState().status[8]).toBe("error");
    expect(useChatStore.getState().error[8]).toBe("Claude not available");
    expect(useChatStore.getState().messages[tid][1].streaming).toBe(false);
  });

  it("loads pending suggestions after a turn that produced them", async () => {
    ctrl.pending = [
      { id: 1, kind: "decision", payload: { topic: "DB", choice: "SQLite" }, status: "pending", created_at: "" },
    ];
    ensureChatListener();
    await useChatStore.getState().send(9, "let's use sqlite");
    const tid = useChatStore.getState().activeId[9]!;
    ctrl.cb!({ type: "done", project_id: 9, transcript_id: tid, reply: "Good call.", suggestions: 1 });
    await flush();
    expect(vi.mocked(suggestionsList)).toHaveBeenCalledWith(9, "pending");
    expect(useChatStore.getState().pending[9]).toHaveLength(1);
  });
});
