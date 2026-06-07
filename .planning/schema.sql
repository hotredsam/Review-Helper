PRAGMA foreign_keys = ON;

CREATE TABLE projects (
  id INTEGER PRIMARY KEY, name TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('imported','new')),
  app_type TEXT, github_repo_url TEXT, clone_path TEXT,
  default_branch TEXT DEFAULT 'main',
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE plans (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  version INTEGER NOT NULL, current_state TEXT, body_md TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (project_id, version)
);
CREATE TABLE phases (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  plan_version INTEGER NOT NULL, idx INTEGER NOT NULL, title TEXT NOT NULL, goal TEXT,
  status TEXT NOT NULL DEFAULT 'not_started' CHECK (status IN ('not_started','in_progress','done')),
  github_issue_number INTEGER, marker TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE tasks (
  id INTEGER PRIMARY KEY,
  phase_id INTEGER NOT NULL REFERENCES phases(id) ON DELETE CASCADE,
  idx INTEGER NOT NULL, title TEXT NOT NULL, body_md TEXT, verification TEXT,
  status TEXT NOT NULL DEFAULT 'not_started' CHECK (status IN ('not_started','in_progress','done'))
);
CREATE TABLE decisions (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  topic TEXT NOT NULL, choice TEXT NOT NULL, rationale TEXT, alternatives TEXT,
  consequences TEXT, source_ref TEXT, plan_version INTEGER,
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','superseded')),
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE questions (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  dimension TEXT, bank_topic TEXT, text TEXT NOT NULL, recommended_answer TEXT,
  status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','answered','not_relevant','unknown','deleted')),
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE answers (
  id INTEGER PRIMARY KEY,
  question_id INTEGER REFERENCES questions(id) ON DELETE SET NULL,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  body TEXT NOT NULL, source TEXT CHECK (source IN ('typed','audio','chat')),
  incorporated_in_version INTEGER,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE features (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  title TEXT NOT NULL, detail TEXT, source TEXT CHECK (source IN ('text','audio')),
  status TEXT NOT NULL DEFAULT 'inbox' CHECK (status IN ('inbox','triaged','in_plan','rejected')),
  target_phase_id INTEGER REFERENCES phases(id) ON DELETE SET NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE assessments (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  plan_version INTEGER, dimension_scores TEXT, overall INTEGER,
  production_readiness TEXT, hygiene TEXT, top_fixes TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE stack_selections (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  pane TEXT NOT NULL CHECK (pane IN ('frontend','backend','database','deployment','pipes')),
  choice TEXT, alternatives TEXT, rationale TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (project_id, pane)
);
CREATE TABLE learning_cards (
  id INTEGER PRIMARY KEY, term TEXT NOT NULL,
  domain TEXT CHECK (domain IN ('architecture','frontend','backend','pipes','deployment','business','design','ux','other')),
  what_md TEXT, when_md TEXT, why_md TEXT,
  source TEXT CHECK (source IN ('seed','detected','generated')),
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (term)
);
CREATE TABLE suggestions (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('decision','answer','feature','stack')),
  payload TEXT,
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','approved','dismissed')),
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);
CREATE INDEX idx_phases_project ON phases(project_id, plan_version, idx);
CREATE INDEX idx_decisions_project ON decisions(project_id, status);
CREATE INDEX idx_questions_project ON questions(project_id, status);
CREATE INDEX idx_features_project ON features(project_id, status);
CREATE INDEX idx_answers_question_project ON answers(question_id, project_id);
CREATE INDEX idx_suggestions_project_status ON suggestions(project_id, status);
