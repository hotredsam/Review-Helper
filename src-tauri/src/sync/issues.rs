//! Issue reconciliation — the pure engine behind the confirmed-preview sync.
//! Given the live GitHub issues and the plan's phases, it computes the exact
//! create/update/close actions WITHOUT touching GitHub, so the UI previews and
//! the user confirms before anything is applied. Idempotent and rename-safe:
//! a phase is matched to its issue by stored number first, then by a stable
//! marker embedded in the issue body. It never adopts foreign issues by title
//! and never wipes labels it didn't set.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

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
    pub issue_number: Option<u64>,    // recorded from a prior sync
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IssueAction {
    Create { marker: String, title: String, body: String, state: String, labels: Vec<String> },
    Update { number: u64, marker: String, title: String, body: String, state: String, labels: Vec<String> },
    Close { number: u64, title: String },
}

const MARKER_OPEN: &str = "<!-- rh-phase:";
const MARKER_CLOSE: &str = "-->";

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

/// Keep the issue's non-status labels (user-added) and set the one status label.
fn merge_labels(existing: &[String], status_label: &str) -> Vec<String> {
    let mut v: Vec<String> = existing.iter().filter(|l| !l.starts_with("status:")).cloned().collect();
    v.push(status_label.to_string());
    v
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

/// Compute create/update/close actions to reconcile `existing` issues with the
/// plan `phases`. Each phase maps to ONE issue — matched by recorded number,
/// then by marker. Foreign issues (no marker, not recorded) are never touched;
/// owned orphans (carry a marker, no matching phase) are closed.
pub fn reconcile(existing: &[IssueRef], phases: &[PhasePlan]) -> Vec<IssueAction> {
    let mut actions = Vec::new();
    let mut consumed: HashSet<u64> = HashSet::new();

    for p in phases {
        let (state, status_label) = status_state_label(&p.status);
        let body = build_body(p);
        let matched = p
            .issue_number
            .and_then(|n| existing.iter().find(|i| i.number == n && !consumed.contains(&i.number)))
            .or_else(|| {
                existing.iter().find(|i| {
                    !consumed.contains(&i.number) && extract_marker(&i.body).as_deref() == Some(p.marker.as_str())
                })
            });
        match matched {
            Some(i) => {
                consumed.insert(i.number);
                actions.push(IssueAction::Update {
                    number: i.number,
                    marker: p.marker.clone(),
                    title: p.title.clone(),
                    body,
                    state: state.into(),
                    labels: merge_labels(&i.labels, status_label),
                });
            }
            None => actions.push(IssueAction::Create {
                marker: p.marker.clone(),
                title: p.title.clone(),
                body,
                state: state.into(),
                labels: vec![status_label.into()],
            }),
        }
    }

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

    fn phase(marker: &str, title: &str, status: &str, num: Option<u64>) -> PhasePlan {
        PhasePlan {
            marker: marker.into(),
            title: title.into(),
            goal: Some("g".into()),
            status: status.into(),
            tasks: vec![("t".into(), status.into())],
            issue_number: num,
        }
    }
    fn issue(number: u64, title: &str, body: &str, state: &str, labels: &[&str]) -> IssueRef {
        IssueRef { number, title: title.into(), body: body.into(), state: state.into(), labels: labels.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn creates_when_no_existing_issue() {
        let a = reconcile(&[], &[phase("phase-setup", "Setup", "not_started", None)]);
        assert_eq!(a.len(), 1);
        assert!(matches!(&a[0], IssueAction::Create { labels, .. } if labels == &["status:todo"]));
    }

    #[test]
    fn idempotent_update_by_recorded_number_survives_rename() {
        // Phase renamed; marker changed; but the recorded number still matches.
        let existing = vec![issue(7, "Old name", "body\n<!-- rh-phase: phase-old -->", "open", &[])];
        let a = reconcile(&existing, &[phase("phase-new-name", "New name", "done", Some(7))]);
        assert_eq!(a.len(), 1);
        match &a[0] {
            IssueAction::Update { number, state, .. } => {
                assert_eq!(*number, 7);
                assert_eq!(state, "closed");
            }
            _ => panic!("expected Update via recorded number"),
        }
    }

    #[test]
    fn update_merges_user_labels_keeps_only_one_status() {
        let existing = vec![issue(5, "Setup", "x\n<!-- rh-phase: phase-setup -->", "open", &["priority:high", "status:todo"])];
        let a = reconcile(&existing, &[phase("phase-setup", "Setup", "in_progress", Some(5))]);
        match &a[0] {
            IssueAction::Update { labels, .. } => {
                assert!(labels.contains(&"priority:high".to_string()), "user label kept");
                assert!(labels.contains(&"status:in-progress".to_string()), "new status set");
                assert_eq!(labels.iter().filter(|l| l.starts_with("status:")).count(), 1, "exactly one status label");
            }
            _ => panic!("expected Update"),
        }
    }

    #[test]
    fn never_adopts_foreign_issues_by_title() {
        // A foreign issue with the same title but no marker + not recorded -> untouched.
        let existing = vec![issue(3, "Build the API", "someone else's issue", "open", &[])];
        let a = reconcile(&existing, &[phase("phase-build-the-api", "Build the API", "not_started", None)]);
        assert_eq!(a.len(), 1);
        assert!(matches!(&a[0], IssueAction::Create { .. }), "creates ours; leaves the foreign one alone");
        assert!(!a.iter().any(|x| matches!(x, IssueAction::Close { number: 3, .. })));
    }

    #[test]
    fn closes_owned_orphans_only() {
        let existing = vec![
            issue(10, "Old phase", "x\n<!-- rh-phase: phase-old -->", "open", &[]),
            issue(11, "Someone's bug", "no marker", "open", &[]),
        ];
        let a = reconcile(&existing, &[phase("phase-new", "New", "not_started", None)]);
        assert!(a.iter().any(|x| matches!(x, IssueAction::Close { number: 10, .. })));
        assert!(!a.iter().any(|x| matches!(x, IssueAction::Close { number: 11, .. })));
    }
}
