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

describe("chat deletion (Phase 15)", () => {
  const meta = (id: number, title: string) => ({ id, title, updated_at: "", message_count: 1 });

  it("rail delete confirms through the Modal; cancel aborts with no API call", async () => {
    const { render, screen } = await import("@testing-library/react");
    const { default: userEvent } = await import("@testing-library/user-event");
    const { ChatHistoryRail } = await import("../components/ChatHistoryRail");
    const { chatDelete } = await import("../api/chat");

    useChatStore.setState({
      transcripts: { 7: [meta(100, "Stack chat"), meta(101, "Plan chat")] },
      activeId: { 7: 100 },
    } as any);

    const user = userEvent.setup();
    render(<ChatHistoryRail project={7} />);
    const [firstDelete] = screen.getAllByRole("button", { name: "Delete chat" });

    await user.click(firstDelete);
    expect(vi.mocked(chatDelete)).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(vi.mocked(chatDelete)).not.toHaveBeenCalled();
    expect(useChatStore.getState().transcripts[7]).toHaveLength(2);

    await user.click(screen.getAllByRole("button", { name: "Delete chat" })[0]);
    // The dialog's confirm button is also named "Delete chat" — it's the last one rendered.
    const buttons = screen.getAllByRole("button", { name: "Delete chat" });
    await user.click(buttons[buttons.length - 1]);
    expect(vi.mocked(chatDelete)).toHaveBeenCalledWith(100);
  });

  it("keeps the transcript listed and surfaces a notice when the backend delete fails", async () => {
    const { chatDelete } = await import("../api/chat");
    const { useUiStore } = await import("../store/uiStore");
    vi.mocked(chatDelete).mockRejectedValueOnce(new Error("disk I/O error"));
    useUiStore.getState().setNotice(null);
    useChatStore.setState({
      transcripts: { 7: [meta(100, "Stack chat")] },
      activeId: { 7: 100 },
    } as any);

    await useChatStore.getState().removeTranscript(7, 100);

    // Row stays — removing it would lie about what persisted.
    expect(useChatStore.getState().transcripts[7]).toHaveLength(1);
    expect(useUiStore.getState().notice).toMatch(/Couldn't delete chat/);
  });
});
