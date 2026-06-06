import { describe, it, expect, beforeEach, vi } from "vitest";

const ctrl = vi.hoisted(() => ({ cb: null as null | ((e: any) => void) }));

vi.mock("../api/chat", () => ({
  chatSend: vi.fn(async () => {}),
  onChatEvent: vi.fn(async (cb: (e: any) => void) => {
    ctrl.cb = cb;
    return () => {};
  }),
}));

import { useChatStore, ensureChatListener } from "../store/chatStore";
import { chatSend } from "../api/chat";

beforeEach(() => {
  useChatStore.setState({ messages: {}, session: {}, status: {}, error: {}, lastSuggestions: {} });
  vi.clearAllMocks();
});

describe("chat store", () => {
  it("streams a turn and resumes the session across turns", async () => {
    ensureChatListener();
    await useChatStore.getState().send(7, "What's my stack?");

    let msgs = useChatStore.getState().messages[7];
    expect(msgs.map((m) => m.role)).toEqual(["user", "assistant"]);
    expect(msgs[0].text).toBe("What's my stack?");
    expect(useChatStore.getState().status[7]).toBe("streaming");
    expect(vi.mocked(chatSend)).toHaveBeenCalledWith(7, "What's my stack?", null);

    // tokens stream into the assistant bubble
    ctrl.cb!({ type: "token", project_id: 7, text: "You use " });
    ctrl.cb!({ type: "token", project_id: 7, text: "SQLite." });
    expect(useChatStore.getState().messages[7][1].text).toBe("You use SQLite.");

    // done finalizes the reply + stores the session id
    ctrl.cb!({ type: "done", project_id: 7, session_id: "sess-1", reply: "You use SQLite.", suggestions: 0 });
    msgs = useChatStore.getState().messages[7];
    expect(msgs[1].streaming).toBe(false);
    expect(useChatStore.getState().session[7]).toBe("sess-1");
    expect(useChatStore.getState().status[7]).toBe("idle");

    // the next turn resumes with the stored session id
    await useChatStore.getState().send(7, "And the DB file?");
    expect(vi.mocked(chatSend)).toHaveBeenLastCalledWith(7, "And the DB file?", "sess-1");
  });

  it("surfaces a failure without a silent spinner", async () => {
    ensureChatListener();
    await useChatStore.getState().send(8, "hi");
    ctrl.cb!({ type: "failed", project_id: 8, detail: "Claude not available" });
    expect(useChatStore.getState().status[8]).toBe("error");
    expect(useChatStore.getState().error[8]).toBe("Claude not available");
    expect(useChatStore.getState().messages[8][1].streaming).toBe(false);
  });
});
