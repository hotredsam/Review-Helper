// @ts-nocheck
/**
 * QA-only mock Tauri bridge so panes render with representative sample data in a
 * plain browser (Playwright screenshots). NOT part of the production build — only
 * `vite dev` serves preview.html, and production `vite build` ignores it.
 */
let nextCb = 1;
const callbacks: Record<number, (p: unknown) => void> = {};

const project = {
  id: 1,
  name: "meeting-notes-app",
  kind: "imported",
  app_type: "web",
  github_repo_url: "https://github.com/demo/meeting-notes-app",
  clone_path: "/tmp/demo",
  default_branch: "main",
  created_at: "2026-01-01 00:00:00",
  updated_at: "2026-06-01 00:00:00",
};

const dim = (score, reason, tech) => ({ score, reason, reason_technical: tech });
const assessment = {
  overall: 64,
  dimensions: {
    architecture: dim(72, "The screen and the data live in separate places — easy to follow.", "Clear UI/data-layer split; src/api wraps every Tauri command, but two components exceed 300 lines."),
    modularity: dim(58, "A few files are doing too much; breaking them up would help.", "3 files over 300 lines (StatePane, PlanPane, NewProjectDialog); duplicate tint() helpers."),
    context_hygiene: dim(61, "Most of the code is easy to pick up one piece at a time.", "Mostly local zustand state; a couple of module-level singletons widen scope. CLAUDE.md present."),
    security: dim(80, "Secrets are kept safe and inputs are checked.", "0 secret_pattern_hits; token in the macOS Keychain; prompt context fenced as untrusted data."),
    git_discipline: dim(55, "Changes land in big chunks; smaller commits would read better.", "Large multi-file commits; prefer atomic per-task commits (git_commits=128)."),
    workflow: dim(60, "There's a plan, but the work doesn't always follow it in order.", "PLAN.md exists; current_state drifts from the phase markers in .planning/."),
  },
  production: {
    overall: 71,
    scores: { tests: { score: 65 }, error_handling: { score: 70 }, secrets: { score: 90 }, build_ci: { score: 60 }, dependencies: { score: 75 }, docs: { score: 50 } },
  },
  top_fixes: [
    { easy: "Split the two biggest files so each does one job.", technical: "Extract ScoreRow/sections out of StatePane.tsx and split PlanPane.tsx (both >300 lines)." },
    { easy: "Add tests around saving so a change can't quietly break it.", technical: "Cover the save_into_tx → carry_into_tx path with an integration test." },
    { easy: "Write a short README so a new person can run it.", technical: "Add a README with prerequisites + `npm run tauri dev` and the test commands." },
  ],
  hygiene: [
    { easy: "Delete the leftover scratch files in the main folder.", technical: "Remove the two stray Mach-O binaries at repo root (gitignored but still on disk)." },
    { easy: "Lock the build machine version so builds stay the same.", technical: "Pin CI runs-on to macos-15 instead of macos-latest for reproducible artifacts." },
  ],
};

const plan = {
  version: 3,
  current_state:
    "## Where things stand\n\nA working prototype: notes can be created and listed.\n\n- The screen renders and saves notes\n- Storage is in-memory (no real database yet)\n- No tests around the save path\n\nAuth and search are **not** built yet.",
  body_md: "## Overview\n\nBuild a simple, reliable note app one phase at a time.",
  phases: [
    {
      id: 1, idx: 0, title: "Persistence", goal: "Notes survive a restart.", status: "done",
      tasks: [{ id: 1, idx: 0, title: "Wire up SQLite", body_md: "Use one connection behind a mutex.", verification: "A note is still there after relaunch.", status: "done" }],
    },
    {
      id: 2, idx: 1, title: "Search", goal: "Find a note by its text.", status: "in_progress",
      tasks: [{ id: 2, idx: 0, title: "Add a search box", body_md: "Filter the list as you type.", verification: "Typing filters the visible notes.", status: "not_started" }],
    },
  ],
  decisions: [{ topic: "Database", choice: "SQLite", rationale: "Simple, embedded, no server to run." }],
  stack: [{ pane: "frontend", choice: "React + Vite" }, { pane: "database", choice: "SQLite" }],
};

const questions = [
  { id: 1, dimension: "scope", bank_topic: "MVP boundary", text: "What's the smallest version of this that's still useful?", recommended_answer: "Create + list notes", ui_spec: { field: "single_choice", options: ["Create + list notes", "Add edit + delete", "Add search", "Add sharing"] }, status: "open" },
  { id: 2, dimension: "users", bank_topic: "Primary user", text: "Who is the main person using this?", recommended_answer: "Solo note-taker", ui_spec: { field: "short_text" }, status: "open" },
  { id: 3, dimension: "scope", bank_topic: "Priority", text: "How important is offline support for v1?", recommended_answer: "3", ui_spec: { field: "scale", min: 1, max: 5, min_label: "Nice-to-have", max_label: "Must-have" }, status: "open" },
  { id: 4, dimension: "data", bank_topic: "Entities", text: "Which of these does a note need to store?", recommended_answer: "Title, Body, Timestamp", ui_spec: { field: "multi_choice", options: ["Title", "Body", "Tags", "Timestamp", "Author"] }, status: "open" },
];

const stackCatalog = {
  frontend: [{ choice: "React + Vite" }, { choice: "Svelte" }, { choice: "Vue" }],
  backend: [{ choice: "Node + Express" }, { choice: "Python + FastAPI" }],
  database: [{ choice: "SQLite" }, { choice: "Postgres" }],
  deployment: [{ choice: "Vercel" }, { choice: "Docker" }],
  pipes: [{ choice: "Background jobs" }, { choice: "Webhooks" }],
};
const stackList = [
  { pane: "frontend", choice: "React + Vite", alternatives: "", rationale: "Fast, familiar, great developer experience." },
  { pane: "backend", choice: "Node + Express", alternatives: "", rationale: "Simple to start, easy to host." },
  { pane: "database", choice: "SQLite", alternatives: "", rationale: "Embedded, zero-config, reliable." },
  { pane: "deployment", choice: "", alternatives: "", rationale: "" },
  { pane: "pipes", choice: "", alternatives: "", rationale: "" },
];

const cards = [
  { id: 1, term: "Caching", domain: "backend", what_md: "Storing a result so you don't have to recompute it.", when_md: null, why_md: "Speeds up repeated work.", source: "seed" },
  { id: 2, term: "Database migrations", domain: "backend", what_md: "Versioned schema changes applied in order.", when_md: null, why_md: "Keeps every copy of the database in step.", source: "detected" },
  { id: 3, term: "Separation of concerns", domain: "architecture", what_md: "Each part of the code does one job.", when_md: null, why_md: "Makes changes safer and easier to follow.", source: "seed" },
  { id: 4, term: "Idempotency", domain: "backend", what_md: "Running the same operation twice has the same effect as once.", when_md: null, why_md: "Safe retries without double effects.", source: "seed" },
];

const handlers: Record<string, (args?: unknown) => unknown> = {
  app_info: () => ({ name: "Review Helper", version: "0.1.0" }),
  model_status: () => ({ provider: "claude", available: true, version: "2.1.168 (Claude Code)", reason: null, command: "claude --version", exit_code: 0, stderr: "" }),
  get_model_config: () => ({ provider: "claude", local_endpoint: null, api_credit_overflow: false }),
  list_projects: () => [project],
  get_project: () => project,
  create_project: () => project,
  github_status: () => ({ connected: false }),
  github_list_repos: () => [],
  get_assessment: () => assessment,
  get_plan: () => plan,
  audit_list: () => [{ version: 3, source: "update", at: "2026-06-01 12:00:00" }],
  grill_list: () => questions,
  stack_catalog: () => stackCatalog,
  stack_premade: () => [{ name: "Web app", summary: "React + Node + SQLite" }, { name: "Static site", summary: "Vite + Markdown" }],
  stack_list: () => stackList,
  cards_list: () => cards,
  card_get: () => cards[0],
  suggestions_list: () => [],
  decisions_list: () => plan.decisions.map((d, i) => ({ id: i + 1, topic: d.topic, choice: d.choice, rationale: d.rationale, alternatives: "", consequences: "", source_ref: "plan", status: "active", created_at: "" })),
  features_list: () => [],
  features_pending_count: () => 0,
  chat_new: () => 99,
  chat_transcripts: () => [
    { id: 1, title: "Does this app have all CRUD features?", updated_at: "2026-06-01 10:00:00", message_count: 4 },
    { id: 2, title: "How should I handle going offline?", updated_at: "2026-05-30 09:00:00", message_count: 2 },
  ],
  chat_messages: (args: any) =>
    (args?.transcriptId ?? 1) === 2
      ? [
          { role: "user", content: "How should I handle going offline?" },
          { role: "assistant", content: "Queue writes locally and sync them when the connection is back." },
        ]
      : [
          { role: "user", content: "Does this app have all CRUD features?" },
          { role: "assistant", content: "Yes — create, read, and update are wired; delete is the one gap (notes can't be deleted yet). Want me to add it to the plan?" },
          { role: "user", content: "Yes, add delete." },
          { role: "assistant", content: "Added a “Delete notes” task to the Search phase, and noted it as a decision." },
        ],
};

function mock(cmd: string, args?: unknown) {
  const h = handlers[cmd];
  if (h) return Promise.resolve(h(args));
  // Tauri event plumbing + any unmocked command: resolve harmlessly.
  return Promise.resolve(0);
}

export function installMock() {
  (window as any).__TAURI_INTERNALS__ = {
    invoke: (cmd: string, args?: unknown) => mock(cmd, args),
    transformCallback: (cb: (p: unknown) => void) => {
      const id = nextCb++;
      callbacks[id] = cb;
      return id;
    },
    convertFileSrc: (p: string) => p,
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  };
}
