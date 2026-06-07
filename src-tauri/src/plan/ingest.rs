//! Pre-scan a clone for existing planning material and pull its content into the
//! analysis prompt, so the first plan provably reflects (rather than relies on
//! the model to find) docs like README / PLANNING.md / ROADMAP / .planning/.

use std::path::Path;

const KNOWN_ROOT_FILES: [&str; 10] = [
    "README.md",
    "README",
    "PLANNING.md",
    "ROADMAP.md",
    "ROADMAP",
    "TODO.md",
    "TODO",
    "ARCHITECTURE.md",
    "CONTRIBUTING.md",
    "CHANGELOG.md",
];

const PER_FILE_CAP: usize = 8_000;
const TOTAL_CAP: usize = 32_000;

/// Collect existing planning docs as a prompt block. Empty string if none found.
pub fn collect_existing_docs(clone_path: &str) -> String {
    let root = Path::new(clone_path);
    // Treat the clone as untrusted: resolve the real clone root, then refuse any
    // doc whose real path escapes it, so a hostile/misconfigured repo can't point
    // README/.planning at e.g. /etc/passwd or ~/.ssh and leak it into the
    // analysis prompt. Mirrors the defense in cards/detect.rs.
    let Ok(canon_root) = std::fs::canonicalize(root) else {
        return String::new();
    };
    let mut found: Vec<(String, String)> = Vec::new();
    let mut total = 0usize;

    if let Ok(entries) = std::fs::read_dir(&canon_root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if KNOWN_ROOT_FILES.iter().any(|k| k.eq_ignore_ascii_case(&name)) {
                add_file(&entry.path(), &name, &canon_root, &mut found, &mut total);
            }
        }
    }

    for sub in [".planning", "docs"] {
        if total >= TOTAL_CAP {
            break;
        }
        if let Ok(entries) = std::fs::read_dir(canon_root.join(sub)) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|x| x.eq_ignore_ascii_case("md")) {
                    let rel = format!("{sub}/{}", entry.file_name().to_string_lossy());
                    add_file(&path, &rel, &canon_root, &mut found, &mut total);
                }
            }
        }
    }

    if found.is_empty() {
        return String::new();
    }
    // Stable order so prompts are deterministic.
    found.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = String::from(
        "## Existing planning material found in the repo\n\n\
         Absorb and build on this — do not discard or contradict it.\n",
    );
    for (name, content) in &found {
        out.push_str(&format!("\n### {name}\n{content}\n"));
    }
    out
}

fn add_file(path: &Path, name: &str, canon_root: &Path, found: &mut Vec<(String, String)>, total: &mut usize) {
    if *total >= TOTAL_CAP {
        return;
    }
    // Resolve symlinks; require the real target to stay inside the clone.
    let Ok(real) = std::fs::canonicalize(path) else {
        return;
    };
    if !real.starts_with(canon_root) {
        return;
    }
    if let Ok(content) = std::fs::read_to_string(&real) {
        let capped: String = content.chars().take(PER_FILE_CAP).collect();
        *total += capped.len();
        found.push((name.to_string(), capped));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn tmp() -> std::path::PathBuf {
        // Atomic counter => unique even when tests run in parallel.
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("rh-ingest-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn collects_root_and_subdir_docs() {
        let dir = tmp();
        std::fs::write(dir.join("PLANNING.md"), "Phase 1: build auth.").unwrap();
        std::fs::write(dir.join("README.md"), "# my app").unwrap();
        std::fs::create_dir_all(dir.join(".planning")).unwrap();
        std::fs::write(dir.join(".planning").join("DECISIONS.md"), "Use SQLite.").unwrap();
        std::fs::write(dir.join("ignore.txt"), "not a doc").unwrap();

        let block = collect_existing_docs(dir.to_str().unwrap());
        assert!(block.contains("### PLANNING.md"));
        assert!(block.contains("Phase 1: build auth."));
        assert!(block.contains(".planning/DECISIONS.md"));
        assert!(block.contains("Use SQLite."));
        assert!(!block.contains("not a doc"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn empty_when_no_docs() {
        let dir = tmp();
        std::fs::write(dir.join("main.rs"), "fn main() {}").unwrap();
        assert!(collect_existing_docs(dir.to_str().unwrap()).is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlinked_doc_escaping_the_clone() {
        // base/secret.md lives OUTSIDE the clone; clone/README.md symlinks to it.
        let base = tmp();
        let clone = base.join("clone");
        std::fs::create_dir_all(&clone).unwrap();
        let secret = base.join("secret.md");
        std::fs::write(&secret, "TOP SECRET — should never reach the prompt").unwrap();
        std::os::unix::fs::symlink(&secret, clone.join("README.md")).unwrap();

        let block = collect_existing_docs(clone.to_str().unwrap());
        assert!(!block.contains("TOP SECRET"), "an escaping symlinked doc must not be read");
        assert!(block.is_empty(), "nothing collected from the escaping symlink");

        std::fs::remove_dir_all(&base).ok();
    }
}
