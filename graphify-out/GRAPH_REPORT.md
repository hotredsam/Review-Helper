# Graph Report - .  (2026-06-07)

## Corpus Check
- 199 files · ~84,162 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1267 nodes · 2747 edges · 65 communities (61 shown, 4 thin omitted)
- Extraction: 97% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 76 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Analysis Frontend API|Analysis Frontend API]]
- [[_COMMUNITY_Chat & Model Commands|Chat & Model Commands]]
- [[_COMMUNITY_Tech Detection & Q&A|Tech Detection & Q&A]]
- [[_COMMUNITY_IssuePhase Types|Issue/Phase Types]]
- [[_COMMUNITY_Model Provider Core|Model Provider Core]]
- [[_COMMUNITY_ChatDecisions Frontend|Chat/Decisions Frontend]]
- [[_COMMUNITY_GitHub REST API|GitHub REST API]]
- [[_COMMUNITY_Claude Code Provider|Claude Code Provider]]
- [[_COMMUNITY_Suggestion Approval|Suggestion Approval]]
- [[_COMMUNITY_Cards Backend|Cards Backend]]
- [[_COMMUNITY_CatalogStack Types|Catalog/Stack Types]]
- [[_COMMUNITY_Grill Commands|Grill Commands]]
- [[_COMMUNITY_Assessment & Charts UI|Assessment & Charts UI]]
- [[_COMMUNITY_Grill Frontend|Grill Frontend]]
- [[_COMMUNITY_CardsStack Frontend|Cards/Stack Frontend]]
- [[_COMMUNITY_Assessment Backend|Assessment Backend]]
- [[_COMMUNITY_Frontend Dependencies|Frontend Dependencies]]
- [[_COMMUNITY_Projects Repository|Projects Repository]]
- [[_COMMUNITY_Plan Store & Carry|Plan Store & Carry]]
- [[_COMMUNITY_GitHub Auth Commands|GitHub Auth Commands]]
- [[_COMMUNITY_Feature Triage|Feature Triage]]
- [[_COMMUNITY_GitHub UI & Dialogs|GitHub UI & Dialogs]]
- [[_COMMUNITY_Plan JSON Parsing|Plan JSON Parsing]]
- [[_COMMUNITY_Context Assembly|Context Assembly]]
- [[_COMMUNITY_Projects Frontend|Projects Frontend]]
- [[_COMMUNITY_Decisions Backend|Decisions Backend]]
- [[_COMMUNITY_Tauri Config|Tauri Config]]
- [[_COMMUNITY_App Shell & Tour|App Shell & Tour]]
- [[_COMMUNITY_GitHub Issues Sync|GitHub Issues Sync]]
- [[_COMMUNITY_TS Config|TS Config]]
- [[_COMMUNITY_Model Console UI|Model Console UI]]
- [[_COMMUNITY_Main Pane Sections|Main Pane Sections]]
- [[_COMMUNITY_Model Status UI|Model Status UI]]
- [[_COMMUNITY_Fake Test Provider|Fake Test Provider]]
- [[_COMMUNITY_Settings & Provider UI|Settings & Provider UI]]
- [[_COMMUNITY_Theme Switching|Theme Switching]]
- [[_COMMUNITY_Device Code Auth|Device Code Auth]]
- [[_COMMUNITY_Suggestion Parsing|Suggestion Parsing]]
- [[_COMMUNITY_Git Clone & Refresh|Git Clone & Refresh]]
- [[_COMMUNITY_Docs Ingest|Docs Ingest]]
- [[_COMMUNITY_Audit Log|Audit Log]]
- [[_COMMUNITY_Keychain Tokens|Keychain Tokens]]
- [[_COMMUNITY_Seed Real Repos|Seed Real Repos]]
- [[_COMMUNITY_Audit List Command|Audit List Command]]
- [[_COMMUNITY_Local Stub Provider|Local Stub Provider]]
- [[_COMMUNITY_TS Node Config|TS Node Config]]
- [[_COMMUNITY_Capability Permissions|Capability Permissions]]
- [[_COMMUNITY_App Info Lib|App Info Lib]]
- [[_COMMUNITY_Stack Catalog Data|Stack Catalog Data]]
- [[_COMMUNITY_Model Prompts|Model Prompts]]
- [[_COMMUNITY_Secrets Scan Staged|Secrets Scan Staged]]
- [[_COMMUNITY_Secrets Scan Walk|Secrets Scan Walk]]
- [[_COMMUNITY_Claude Hooks Settings|Claude Hooks Settings]]
- [[_COMMUNITY_Commit Guard Hook|Commit Guard Hook]]
- [[_COMMUNITY_VSCode Extensions|VSCode Extensions]]

## God Nodes (most connected - your core abstractions)
1. `String` - 26 edges
2. `init_connection()` - 23 edges
3. `Result` - 18 edges
4. `http_client()` - 18 edges
5. `String` - 17 edges
6. `status_error()` - 16 edges
7. `String` - 16 edges
8. `compilerOptions` - 16 edges
9. `reconcile()` - 15 edges
10. `useProjectStore` - 15 edges

## Surprising Connections (you probably didn't know these)
- `with_conn()` --calls--> `F`  [INFERRED]
  src-tauri/src/projects.rs → src-tauri/src/util.rs
- `parse_assessment()` --calls--> `extract_json()`  [INFERRED]
  src-tauri/src/assess/mod.rs → src-tauri/src/plan/parse.rs
- `get_assessment()` --calls--> `P`  [INFERRED]
  src-tauri/src/assess/mod.rs → src-tauri/src/util.rs
- `db()` --calls--> `init_connection()`  [INFERRED]
  src-tauri/src/assess/mod.rs → src-tauri/src/db.rs
- `records_and_lists_source_to_version()` --calls--> `init_connection()`  [INFERRED]
  src-tauri/src/audit.rs → src-tauri/src/db.rs

## Import Cycles
- 1-file cycle: `src-tauri/src/assess/commands.rs -> src-tauri/src/assess/commands.rs`
- 1-file cycle: `src-tauri/src/assess/mod.rs -> src-tauri/src/assess/mod.rs`
- 1-file cycle: `src-tauri/src/audit/commands.rs -> src-tauri/src/audit/commands.rs`
- 1-file cycle: `src-tauri/src/cards/commands.rs -> src-tauri/src/cards/commands.rs`
- 1-file cycle: `src-tauri/src/cards/detect.rs -> src-tauri/src/cards/detect.rs`
- 1-file cycle: `src-tauri/src/chat/commands.rs -> src-tauri/src/chat/commands.rs`
- 1-file cycle: `src-tauri/src/db.rs -> src-tauri/src/db.rs`
- 1-file cycle: `src-tauri/src/decisions/commands.rs -> src-tauri/src/decisions/commands.rs`
- 1-file cycle: `src-tauri/src/features/commands.rs -> src-tauri/src/features/commands.rs`
- 1-file cycle: `src-tauri/src/github/clone.rs -> src-tauri/src/github/clone.rs`
- 1-file cycle: `src-tauri/src/model/claude.rs -> src-tauri/src/model/claude.rs`
- 1-file cycle: `src-tauri/src/github/commands.rs -> src-tauri/src/github/commands.rs`
- 1-file cycle: `src-tauri/src/github/device.rs -> src-tauri/src/github/device.rs`
- 1-file cycle: `src-tauri/src/grill/commands.rs -> src-tauri/src/grill/commands.rs`
- 1-file cycle: `src-tauri/src/grill/generate.rs -> src-tauri/src/grill/generate.rs`
- 1-file cycle: `src-tauri/src/model/commands.rs -> src-tauri/src/model/commands.rs`
- 1-file cycle: `src-tauri/src/plan/commands.rs -> src-tauri/src/plan/commands.rs`
- 1-file cycle: `src-tauri/src/plan/ingest.rs -> src-tauri/src/plan/ingest.rs`
- 1-file cycle: `src-tauri/src/plan/store.rs -> src-tauri/src/plan/store.rs`
- 1-file cycle: `src-tauri/src/projects.rs -> src-tauri/src/projects.rs`

## Communities (65 total, 4 thin omitted)

### Community 0 - "Analysis Frontend API"
Cohesion: 0.06
Nodes (41): AnalysisEvent, analyzeProject(), AuditEntry, auditList(), DecisionView, getPlan(), kickoffProject(), onAnalysisEvent() (+33 more)

### Community 1 - "Chat & Model Commands"
Cohesion: 0.07
Nodes (50): chat_send(), ChatEvent, run_chat(), Command, model_run(), model_status(), ModelStatus, probe_claude() (+42 more)

### Community 2 - "Tech Detection & Q&A"
Cohesion: 0.10
Nodes (43): db(), detect_tech_in_clone(), detects_tech_from_manifests_with_word_boundaries(), is_word_char(), mentions(), refuses_symlinked_manifest_escaping_the_clone(), GenQuestion, answer_question() (+35 more)

### Community 3 - "Issue/Phase Types"
Cohesion: 0.11
Nodes (43): IssueAction, PackageFile, PhasePlan, PhaseView, Db, Result, State, String (+35 more)

### Community 4 - "Model Provider Core"
Cohesion: 0.09
Nodes (37): ModelEvent, ModelProvider, ModelRequest, Tool, UnavailableReason, AnalysisEvent, analyze_project(), commit_fresh() (+29 more)

### Community 5 - "Chat/Decisions Frontend"
Cohesion: 0.10
Nodes (27): ChatEvent, chatSend(), onChatEvent(), Decision, decisionsList(), decisionSupersede(), Suggestion, suggestionApprove() (+19 more)

### Community 6 - "GitHub REST API"
Cohesion: 0.16
Nodes (39): From, b64encode(), branch_head_sha(), close_issue(), create_issue(), create_repo(), create_repo_with(), default_branch() (+31 more)

### Community 7 - "Claude Code Provider"
Cohesion: 0.09
Nodes (27): augmented_path(), check_available(), classifies_credit_exhaustion(), classify_result(), classify_stderr(), ClaudeCodeProvider, extra_path_dirs(), not_installed_binary_yields_unavailable() (+19 more)

### Community 8 - "Suggestion Approval"
Cohesion: 0.14
Nodes (40): approve(), approve_all(), approve_all_is_atomic_on_failure(), approve_in_tx(), approve_rejects_corrupt_payload_and_writes_nothing(), approve_writes_the_right_table_dismiss_writes_nothing(), approving_an_answer_links_it_so_context_surfaces_it(), count() (+32 more)

### Community 9 - "Cards Backend"
Cohesion: 0.14
Nodes (37): Card, card_capture(), card_explain(), card_get(), CardGate, cards_list(), has_content(), capture() (+29 more)

### Community 10 - "Catalog/Stack Types"
Cohesion: 0.13
Nodes (37): CatalogOption, PremadeStack, Selection, Db, HashMap, Result, State, String (+29 more)

### Community 11 - "Grill Commands"
Cohesion: 0.12
Nodes (33): grill_answer(), grill_chat_resolve(), grill_delete(), grill_generate(), grill_list(), grill_set_status(), GrillEvent, GrillGate (+25 more)

### Community 12 - "Assessment & Charts UI"
Cohesion: 0.11
Nodes (25): AssessmentEvent, AssessmentView, assessProject(), DimScore, getAssessment(), onAssessmentEvent(), clamp(), Donut() (+17 more)

### Community 13 - "Grill Frontend"
Cohesion: 0.12
Nodes (21): grillAnswer(), grillChatResolve(), grillDelete(), GrillEvent, grillGenerate(), grillList(), grillSetStatus(), onGrillEvent() (+13 more)

### Community 14 - "Cards/Stack Frontend"
Cohesion: 0.11
Nodes (21): Card, cardExplain(), cardsList(), CatalogOption, PremadeStack, Selection, stackApplyPremade(), stackCatalog() (+13 more)

### Community 15 - "Assessment Backend"
Cohesion: 0.12
Nodes (30): assess_project(), AssessmentEvent, get_assessment(), real_assessment_scores_a_repo(), run_assessment(), assess_user(), AssessmentView, db() (+22 more)

### Community 16 - "Frontend Dependencies"
Cohesion: 0.06
Nodes (32): dependencies, lucide-react, react, react-dom, tailwindcss, @tailwindcss/vite, @tauri-apps/api, @tauri-apps/plugin-opener (+24 more)

### Community 17 - "Projects Repository"
Cohesion: 0.23
Nodes (31): FnOnce, create_project(), delete(), delete_project(), get(), get_project(), insert(), insert_attached() (+23 more)

### Community 18 - "Plan Store & Carry"
Cohesion: 0.21
Nodes (31): carry_into_tx(), carry_status(), carry_status_preserves_completion_across_a_merge(), carry_status_survives_phase_reorder_and_rename(), carry_status_warns_when_a_completed_phase_is_dropped(), carry_tasks(), db(), DecisionView (+23 more)

### Community 19 - "GitHub Auth Commands"
Cohesion: 0.21
Nodes (27): DeviceCode, client_id(), DevicePollResult, github_connect_gh(), github_device_poll(), github_device_start(), github_list_repos(), github_sign_out() (+19 more)

### Community 20 - "Feature Triage"
Cohesion: 0.17
Nodes (28): Feature, feature_add(), feature_set_status(), features_list(), features_pending_count(), transcribe_audio_stub(), add(), add_lists_and_triages_features() (+20 more)

### Community 21 - "GitHub UI & Dialogs"
Cohesion: 0.12
Nodes (17): githubConnectGh(), githubListRepos(), githubSignOut(), githubStatus, RepoSummary, Modal(), ModalProps, ConnectHint() (+9 more)

### Community 22 - "Plan JSON Parsing"
Cohesion: 0.13
Nodes (23): D, GenDecision, GenPhase, GenStack, GenTask, coerces_array_prose_fields_to_strings(), extract_json(), extracts_from_fenced_and_prefixed_output() (+15 more)

### Community 23 - "Context Assembly"
Cohesion: 0.21
Nodes (20): assembles_from_seeded_rows_excluding_inactive(), code_fence_in_a_value_cannot_break_out_of_its_delimiter(), ContextAnswer, ContextDecision, ContextStack, db(), empty_project_yields_an_empty_bundle(), fence_safe() (+12 more)

### Community 24 - "Projects Frontend"
Cohesion: 0.19
Nodes (14): cloneProject(), createProject(), createRepoProject(), deleteProject(), importRepo(), linkRepoByUrl(), listProjects(), Project (+6 more)

### Community 25 - "Decisions Backend"
Cohesion: 0.17
Nodes (19): Decision, decision_supersede(), decisions_list(), db(), Decision, list(), lists_all_fields_and_supersede_keeps_history(), project() (+11 more)

### Community 26 - "Tauri Config"
Cohesion: 0.10
Nodes (20): app, security, windows, build, beforeBuildCommand, beforeDevCommand, devUrl, frontendDist (+12 more)

### Community 27 - "App Shell & Tour"
Cohesion: 0.21
Nodes (10): Props, Sidebar(), Tour(), TOUR_STEPS, tourSeen(), TourStep, App(), NoticeBar() (+2 more)

### Community 28 - "GitHub Issues Sync"
Cohesion: 0.27
Nodes (18): Option, String, Vec, build_body(), closes_owned_orphans_only(), creates_when_no_existing_issue(), extract_marker(), idempotent_update_by_recorded_number_survives_rename() (+10 more)

### Community 29 - "TS Config"
Cohesion: 0.11
Nodes (18): compilerOptions, allowImportingTsExtensions, isolatedModules, jsx, lib, module, moduleResolution, noEmit (+10 more)

### Community 30 - "Model Console UI"
Cohesion: 0.23
Nodes (12): ModelEvent, onModelEvent(), runModel(), UnavailableReason, ModelConsole(), appendAssistant(), ensureModelListener(), handleEvent() (+4 more)

### Community 31 - "Main Pane Sections"
Cohesion: 0.20
Nodes (11): ComingSoon(), EmptyState(), EmptyStateProps, MainPane(), Props, RepoCache(), Section, sectionById() (+3 more)

### Community 32 - "Model Status UI"
Cohesion: 0.21
Nodes (10): getModelStatus(), ModelStatus, ModelBanner(), ModelDebug(), StatusState, useStatusStore, claudeDown, claudeUp (+2 more)

### Community 33 - "Fake Test Provider"
Cohesion: 0.28
Nodes (10): a_scripted_unavailable_is_terminal(), collect(), FakeProvider, streams_ordered_events_ending_in_one_terminal(), FnMut, ModelEvent, ModelProvider, ModelRequest (+2 more)

### Community 34 - "Settings & Provider UI"
Cohesion: 0.24
Nodes (8): getModelConfig(), ModelConfig, ProviderKind, setModelConfig(), GithubConnect(), ProviderSettings(), SettingsView(), store

### Community 35 - "Theme Switching"
Cohesion: 0.23
Nodes (7): Client, ThemeSwitcher(), ThemeId, ThemeMeta, THEMES, ThemeState, useThemeStore

### Community 36 - "Device Code Auth"
Cohesion: 0.31
Nodes (10): classify_poll(), DeviceCode, parse_device_code(), parses_a_device_code_response(), poll_token(), PollOutcome, request_device_code(), Result (+2 more)

### Community 37 - "Suggestion Parsing"
Cohesion: 0.27
Nodes (11): blocks_only_yields_empty_reply_with_suggestions(), empty_and_whitespace_input_is_safe(), malformed_or_unknown_blocks_are_skipped_not_invented(), no_blocks_yields_no_suggestions(), parse_kind(), parse_suggestions(), parses_a_decision_and_a_feature_and_strips_the_blocks(), ParsedSuggestion (+3 more)

### Community 38 - "Git Clone & Refresh"
Cohesion: 0.41
Nodes (11): askpass_helper_never_embeds_the_token(), clone_or_refresh(), ensure_askpass(), real_clone_and_refresh(), refresh(), run_git(), Option, Path (+3 more)

### Community 39 - "Docs Ingest"
Cohesion: 0.33
Nodes (10): add_file(), collect_existing_docs(), collects_root_and_subdir_docs(), empty_when_no_docs(), refuses_symlinked_doc_escaping_the_clone(), tmp(), Path, PathBuf (+2 more)

### Community 40 - "Audit Log"
Cohesion: 0.36
Nodes (9): AuditEntry, key(), list(), record(), records_and_lists_source_to_version(), Connection, Result, String (+1 more)

### Community 41 - "Keychain Tokens"
Cohesion: 0.47
Nodes (7): delete_token(), entry(), get_token(), save_token(), Option, Result, String

### Community 42 - "Seed Real Repos"
Cohesion: 0.39
Nodes (8): gh_token(), repos_from_env(), run_model(), seed_recent_repos(), ModelRequest, Option, String, Vec

### Community 43 - "Audit List Command"
Cohesion: 0.32
Nodes (7): audit_list(), AuditEntry, Db, Result, State, String, Vec

### Community 44 - "Local Stub Provider"
Cohesion: 0.29
Nodes (6): LocalStubProvider, stub_emits_one_terminal_notice(), FnMut, ModelEvent, ModelProvider, ModelRequest

### Community 45 - "TS Node Config"
Cohesion: 0.25
Nodes (7): compilerOptions, allowSyntheticDefaultImports, composite, module, moduleResolution, skipLibCheck, include

### Community 46 - "Capability Permissions"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 47 - "App Info Lib"
Cohesion: 0.47
Nodes (4): app_info(), app_info_reports_name_and_version(), AppInfo, String

### Community 48 - "Stack Catalog Data"
Cohesion: 0.33
Nodes (5): backend, database, deployment, frontend, pipes

### Community 49 - "Model Prompts"
Cohesion: 0.67
Nodes (3): kickoff_user(), merge_user(), String

### Community 50 - "Secrets Scan Staged"
Cohesion: 0.83
Nodes (3): main(), scan_text(), staged_files()

## Knowledge Gaps
- **254 isolated node(s):** `guard-commit.sh script`, `PreToolUse`, `recommendations`, `name`, `private` (+249 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **4 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `init_connection()` connect `Tech Detection & Q&A` to `Chat & Model Commands`, `Issue/Phase Types`, `Model Provider Core`, `Audit Log`, `Cards Backend`, `Suggestion Approval`, `Grill Commands`, `Catalog/Stack Types`, `Assessment Backend`, `Projects Repository`, `Plan Store & Carry`, `Feature Triage`, `Context Assembly`, `Decisions Backend`?**
  _High betweenness centrality (0.533) - this node is a cross-community bridge._
- **Why does `memory_db()` connect `Chat & Model Commands` to `Tech Detection & Q&A`?**
  _High betweenness centrality (0.363) - this node is a cross-community bridge._
- **Are the 17 inferred relationships involving `init_connection()` (e.g. with `db()` and `db()`) actually correct?**
  _`init_connection()` has 17 INFERRED edges - model-reasoned connections that need verification._
- **Are the 14 inferred relationships involving `http_client()` (e.g. with `close_issue()` and `create_issue()`) actually correct?**
  _`http_client()` has 14 INFERRED edges - model-reasoned connections that need verification._
- **What connects `guard-commit.sh script`, `PreToolUse`, `recommendations` to the rest of the system?**
  _254 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Analysis Frontend API` be split into smaller, more focused modules?**
  _Cohesion score 0.06253652834599649 - nodes in this community are weakly interconnected._
- **Should `Chat & Model Commands` be split into smaller, more focused modules?**
  _Cohesion score 0.0726764500349406 - nodes in this community are weakly interconnected._