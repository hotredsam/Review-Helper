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

const dim = (score, reason) => ({ score, reason, reason_technical: reason });
const assessment = {
  overall: 64,
  dimensions: {
    architecture: dim(72, "Clear split between the screen and the data layer; a couple of files do too much."),
    modularity: dim(58, "A few large files mix concerns — splitting them would make changes safer."),
    context_hygiene: dim(61, "Most state is local; a couple of globals make the flow harder to follow."),
    security: dim(80, "Secrets stay out of the code and inputs are checked before use."),
    git_discipline: dim(55, "Commits are large; smaller, focused commits would read better."),
    workflow: dim(60, "A plan exists, but the build doesn't always follow it step by step."),
  },
  production: {
    overall: 71,
    scores: {
      tests: { score: 65 },
      error_handling: { score: 70 },
      secrets: { score: 90 },
      build_ci: { score: 60 },
      dependencies: { score: 75 },
      docs: { score: 50 },
    },
  },
  top_fixes: [
    "Split the two biggest files so each does one thing.",
    "Add tests around the save path so a change can't silently break it.",
    "Write a short README so a new person can run it in a minute.",
  ],
  hygiene: [
    "Delete the unused scratch files in the repo root.",
    "Pin the CI runner version so builds stay repeatable.",
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
  { id: 1, dimension: "scope", bank_topic: "MVP boundary", text: "What's the smallest version of this that's still useful to you?", recommended_answer: "Create, list, and edit notes — no sharing yet.", status: "open" },
  { id: 2, dimension: "users", bank_topic: "Primary user", text: "Who is the main person using this, and what are they trying to get done?", recommended_answer: "Just me, capturing meeting notes quickly.", status: "open" },
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
