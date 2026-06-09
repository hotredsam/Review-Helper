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
  ui_spec TEXT, -- model-emitted input UI spec (JSON): field type + options
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
  -- Case-insensitive: lookups are case-insensitive, so "Foo" and "foo" must
  -- be the same card, not two duplicates.
  UNIQUE (term COLLATE NOCASE)
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
CREATE INDEX idx_answers_project_question ON answers(project_id, question_id);
CREATE INDEX idx_suggestions_project_status ON suggestions(project_id, status);

-- Persisted chat transcripts (v3): past chats survive restarts; the model is
-- given the full text of all prior chats for cross-chat memory.
CREATE TABLE chat_transcripts (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  title TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE chat_messages (
  id INTEGER PRIMARY KEY,
  transcript_id INTEGER NOT NULL REFERENCES chat_transcripts(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN ('user','assistant')),
  content TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_chat_transcripts_project ON chat_transcripts(project_id, updated_at);
CREATE INDEX idx_chat_messages_transcript ON chat_messages(transcript_id, id);

-- Understand-hub additions (v5): which cards belong to a project, cached premade
-- questions per card, and an inline per-card chat.
CREATE TABLE project_cards (
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  term TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (project_id, term)
);
CREATE TABLE card_questions (
  id INTEGER PRIMARY KEY,
  term TEXT NOT NULL,
  question TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE card_chat_messages (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  term TEXT NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('user','assistant')),
  content TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_card_questions_term ON card_questions(term);
CREATE INDEX idx_card_chat ON card_chat_messages(project_id, term, id);

-- Learning mode (v6): subjects the user wants to study, independent of code
-- projects. Evidence-based (per deep-research): retrieval practice + spaced
-- repetition (FSRS, stored per flashcard) + per-skill mastery (Bayesian
-- Knowledge Tracing, learning_skill_mastery). No "learning styles" — the
-- learner profile adapts on real interaction signals only.
CREATE TABLE learning_subjects (
  id INTEGER PRIMARY KEY,
  title TEXT NOT NULL,
  source_kind TEXT NOT NULL CHECK (source_kind IN ('describe','upload')),
  source_text TEXT,                 -- described goal or extracted upload text (bounded)
  stage TEXT NOT NULL DEFAULT 'intake' CHECK (stage IN ('intake','proposed','ready')),
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE learning_intake (
  id INTEGER PRIMARY KEY,
  subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
  idx INTEGER NOT NULL,
  question TEXT NOT NULL,
  answer TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE learning_modules (
  id INTEGER PRIMARY KEY,
  subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
  idx INTEGER NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('notes','flashcards','quiz','tutor')),
  title TEXT NOT NULL, summary TEXT, skill TEXT,
  included INTEGER NOT NULL DEFAULT 1,
  status TEXT NOT NULL DEFAULT 'proposed' CHECK (status IN ('proposed','generating','ready','failed')),
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE learning_notes (
  id INTEGER PRIMARY KEY,
  module_id INTEGER NOT NULL REFERENCES learning_modules(id) ON DELETE CASCADE,
  body_md TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE learning_flashcards (
  id INTEGER PRIMARY KEY,
  module_id INTEGER NOT NULL REFERENCES learning_modules(id) ON DELETE CASCADE,
  subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
  skill TEXT, front TEXT NOT NULL, back TEXT NOT NULL,
  fsrs_json TEXT,                   -- serialized rs-fsrs Card state (null until first review)
  due TEXT,                         -- next due (datetime) for the review queue
  reps INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE learning_quiz_questions (
  id INTEGER PRIMARY KEY,
  module_id INTEGER NOT NULL REFERENCES learning_modules(id) ON DELETE CASCADE,
  subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
  skill TEXT, question TEXT NOT NULL,
  options TEXT NOT NULL,            -- JSON array of choices
  answer_idx INTEGER NOT NULL,      -- index of the correct choice
  explanation TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE learning_quiz_attempts (
  id INTEGER PRIMARY KEY,
  question_id INTEGER NOT NULL REFERENCES learning_quiz_questions(id) ON DELETE CASCADE,
  subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
  correct INTEGER NOT NULL, latency_ms INTEGER,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE learning_skill_mastery (
  subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
  skill TEXT NOT NULL,
  p_known REAL NOT NULL DEFAULT 0.3, -- Bayesian Knowledge Tracing mastery estimate
  n_obs INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (subject_id, skill)
);
CREATE TABLE learning_tutor_messages (
  id INTEGER PRIMARY KEY,
  subject_id INTEGER NOT NULL REFERENCES learning_subjects(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN ('user','assistant')),
  content TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE learning_profile (
  subject_id INTEGER PRIMARY KEY REFERENCES learning_subjects(id) ON DELETE CASCADE,
  sessions INTEGER NOT NULL DEFAULT 0,
  total_attempts INTEGER NOT NULL DEFAULT 0,
  total_correct INTEGER NOT NULL DEFAULT 0,
  total_latency_ms INTEGER NOT NULL DEFAULT 0,
  flashcard_reviews INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_learning_intake ON learning_intake(subject_id, idx);
CREATE INDEX idx_learning_modules ON learning_modules(subject_id, idx);
CREATE INDEX idx_learning_flashcards_due ON learning_flashcards(subject_id, due);
CREATE INDEX idx_learning_quiz ON learning_quiz_questions(subject_id);
CREATE INDEX idx_learning_tutor ON learning_tutor_messages(subject_id, id);
