//! Issue reconciliation — the pure engine shared by import-side reconciliation
//! (T3) and push-to-main issue sync (T4). Given the live GitHub issues and the
//! current plan's phases, it computes the exact set of actions (create / update
//! / close) WITHOUT touching GitHub, so the UI can preview and the user can
//! confirm before anything is applied. Idempotent: a second run yields only
//! updates (no duplicate issues), matched by a stable marker embedded in the
//! issue body.

use std::collections::HashSet;

use serde::Serialize;

/// A GitHub issue as reconciliation needs it.
#[derive(Debug, Clone, PartialEq)]
pub struct IssueRef {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub state: String, // "open" | "closed"
    pub labels: Vec<String>,
}

/// A plan phase to mirror as an issue.
#[derive(Debug, Clone, PartialEq)]
pub struct PhasePlan {
    pub marker: String,
    pub title: String,
    pub goal: Option<String>,
    pub status: String,
    pub tasks: Vec<(String, String)>, // (title, status)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IssueAction {
    Create { marker: String, title: String, body: String, state: String, label: String },
    Update { number: u64, marker: String, title: String, body: String, state: String, label: String },
    Close { number: u64, title: String },
}

const MARKER_OPEN: &str = "<!-- rh-phase:";
const MARKER_CLOSE: &str = "-->";

/// The stable phase marker embedded in an issue body, if any.
pub fn extract_marker(body: &str) -> Option<String> {
    let i = body.find(MARKER_OPEN)?;
    let rest = &body[i + MARKER_OPEN.len()..];
    let end = rest.find(MARKER_CLOSE)?;
    let m = rest[..end].trim();
    (!m.is_empty()).then(|| m.to_string())
}

fn status_state_label(status: &str) -> (&'static str, &'static str) {
    match status {
        "done" => ("closed", "status:done"),
        "in_progress" => ("open", "status:in-progress"),
        _ => ("open", "status:todo"),
    }
}

/// The issue body for a phase, including the hidden stable marker.
pub fn build_body(p: &PhasePlan) -> String {
    let mut s = String::new();
    if let Some(g) = p.goal.as_deref().filter(|g| !g.trim().is_empty()) {
        s.push_str(g.trim());
        s.push_str("\n\n");
    }
    s.push_str("### Tasks\n");
    for (t, st) in &p.tasks {
        let mark = if st == "done" { "x" } else { " " };
        s.push_str(&format!("- [{mark}] {t}\n"));
    }
    s.push_str(&format!("\n{MARKER_OPEN} {} {MARKER_CLOSE}\n", p.marker));
    s
}

/// Compute the create/update/close actions to reconcile `existing` issues with
/// the plan `phases`. Pure — no GitHub calls. Each phase maps to ONE issue
/// (matched by marker, then by exact title to adopt a pre-existing issue);
/// orphaned issues we own (they carry a marker) but that no longer map to a
/// phase are closed; issues we don't own (no marker) are left untouched.
pub fn reconcile(existing: &[IssueRef], phases: &[PhasePlan]) -> Vec<IssueAction> {
    let mut actions = Vec::new();
    let mut consumed: HashSet<u64> = HashSet::new();

    for p in phases {
        let (state, label) = status_state_label(&p.status);
        let body = build_body(p);
        let matched = existing
            .iter()
            .find(|i| !consumed.contains(&i.number) && extract_marker(&i.body).as_deref() == Some(p.marker.as_str()))
            .or_else(|| existing.iter().find(|i| !consumed.contains(&i.number) && i.title == p.title));
        match matched {
            Some(i) => {
                consumed.insert(i.number);
                actions.push(IssueAction::Update {
                    number: i.number,
                    marker: p.marker.clone(),
                    title: p.title.clone(),
                    body,
                    state: state.into(),
                    label: label.into(),
                });
            }
            None => actions.push(IssueAction::Create {
                marker: p.marker.clone(),
                title: p.title.clone(),
                body,
                state: state.into(),
                label: label.into(),
            }),
        }
    }

    // Orphans we own (carry a marker) but no longer map to a phase → close.
    for i in existing {
        if consumed.contains(&i.number) {
            continue;
        }
        if extract_marker(&i.body).is_some() && i.state != "closed" {
            actions.push(IssueAction::Close { number: i.number, title: i.title.clone() });
        }
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phase(marker: &str, title: &str, status: &str) -> PhasePlan {
        PhasePlan { marker: marker.into(), title: title.into(), goal: Some("g".into()), status: status.into(), tasks: vec![("t".into(), status.into())] }
    }
    fn issue(number: u64, title: &str, body: &str, state: &str) -> IssueRef {
        IssueRef { number, title: title.into(), body: body.into(), state: state.into(), labels: vec![] }
    }

    #[test]
    fn extracts_marker() {
        assert_eq!(extract_marker("body\n\n<!-- rh-phase: phase-setup -->\n").as_deref(), Some("phase-setup"));
        assert_eq!(extract_marker("no marker here"), None);
    }

    #[test]
    fn creates_when_no_existing_issues() {
        let actions = reconcile(&[], &[phase("phase-setup", "Setup", "not_started")]);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            IssueAction::Create { title, state, label, body, .. } => {
                assert_eq!(title, "Setup");
                assert_eq!(state, "open");
                assert_eq!(label, "status:todo");
                assert!(body.contains("rh-phase: phase-setup"));
            }
            _ => panic!("expected Create"),
        }
    }

    #[test]
    fn idempotent_update_by_marker_no_duplicates() {
        let existing = vec![issue(7, "Setup", "old body\n<!-- rh-phase: phase-setup -->", "open")];
        let actions = reconcile(&existing, &[phase("phase-setup", "Setup", "done")]);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            IssueAction::Update { number, state, label, .. } => {
                assert_eq!(*number, 7);
                assert_eq!(state, "closed", "done -> closed");
                assert_eq!(label, "status:done");
            }
            _ => panic!("expected Update, got {:?}", actions[0]),
        }
    }

    #[test]
    fn adopts_a_pre_existing_issue_by_title() {
        // An old issue with no marker, same title -> adopted (Update embeds marker).
        let existing = vec![issue(3, "Build the API", "legacy issue, no marker", "open")];
        let actions = reconcile(&existing, &[phase("phase-build-the-api", "Build the API", "in_progress")]);
        match &actions[0] {
            IssueAction::Update { number, body, label, .. } => {
                assert_eq!(*number, 3);
                assert!(body.contains("rh-phase: phase-build-the-api"), "marker embedded on adopt");
                assert_eq!(label, "status:in-progress");
            }
            _ => panic!("expected adopt-Update"),
        }
    }

    #[test]
    fn closes_owned_orphans_but_leaves_foreign_issues_alone() {
        let existing = vec![
            issue(10, "Old phase", "x\n<!-- rh-phase: phase-old -->", "open"), // ours, no phase -> close
            issue(11, "Someone's bug", "unrelated, no marker", "open"),         // not ours -> leave
        ];
        let actions = reconcile(&existing, &[phase("phase-new", "New", "not_started")]);
        // one Create (New), one Close (10); 11 untouched.
        assert!(actions.iter().any(|a| matches!(a, IssueAction::Create { .. })));
        assert!(actions.iter().any(|a| matches!(a, IssueAction::Close { number: 10, .. })));
        assert!(!actions.iter().any(|a| matches!(a, IssueAction::Close { number: 11, .. })), "foreign issue untouched");
    }
}
