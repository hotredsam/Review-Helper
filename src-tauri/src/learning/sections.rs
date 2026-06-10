//! Structure-aware splitting of uploaded study material into sections, so big
//! documents are FULLY covered (the old path silently truncated at 40k chars).
//! Splits on Markdown-style headings first, then blank-line paragraph
//! boundaries, packing greedily to a target size — never mid-paragraph.

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// First heading seen in the section, for labeling ("Ch 3 — Photosynthesis").
    pub title: Option<String>,
    pub body: String,
}

/// Target size of one section. Comfortably inside one model call alongside the
/// system prompt; Phase 21 sub-chunks beneath this for retrieval.
pub const SECTION_TARGET_CHARS: usize = 30_000;

fn is_heading(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with('#')
        || (t.len() < 80
            && !t.is_empty()
            && (t.to_uppercase() == t && t.chars().filter(|c| c.is_alphabetic()).count() >= 4))
}

/// Split into paragraph-ish blocks (heading lines become their own block).
fn blocks(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            if !cur.trim().is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            continue;
        }
        if is_heading(line) && !cur.trim().is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        cur.push_str(line);
        cur.push('\n');
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

fn first_heading(block: &str) -> Option<String> {
    let line = block.lines().next()?.trim();
    if is_heading(line) {
        Some(line.trim_start_matches('#').trim().chars().take(120).collect())
    } else {
        None
    }
}

/// Pack blocks greedily into sections of ~`target` chars. A single oversized
/// block (one giant paragraph) is hard-split rather than dropped.
pub fn split_sections(text: &str, target: usize) -> Vec<Section> {
    let text = text.trim();
    if text.is_empty() {
        return vec![];
    }
    if text.len() <= target {
        return vec![Section { title: first_heading(text), body: text.to_string() }];
    }
    let mut sections: Vec<Section> = Vec::new();
    let mut cur = String::new();
    let mut cur_title: Option<String> = None;
    for block in blocks(text) {
        if cur.len() + block.len() > target && !cur.trim().is_empty() {
            sections.push(Section { title: cur_title.take(), body: std::mem::take(&mut cur) });
        }
        if cur.is_empty() {
            cur_title = first_heading(&block);
        }
        if block.len() > target {
            // One pathological paragraph: hard-split on char boundaries.
            let mut rest = block.as_str();
            while rest.len() > target {
                let mut cut = target;
                while !rest.is_char_boundary(cut) {
                    cut -= 1;
                }
                sections.push(Section { title: cur_title.take(), body: format!("{cur}{}", &rest[..cut]) });
                cur.clear();
                rest = &rest[cut..];
            }
            cur.push_str(rest);
        } else {
            cur.push_str(&block);
            cur.push('\n');
        }
    }
    if !cur.trim().is_empty() {
        sections.push(Section { title: cur_title, body: cur });
    }
    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_text_is_one_section() {
        let s = split_sections("# Intro\nhello world", 30_000);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].title.as_deref(), Some("Intro"));
    }

    #[test]
    fn large_text_splits_on_headings_and_covers_everything() {
        let mut doc = String::new();
        for i in 0..12 {
            doc.push_str(&format!("# Chapter {i}\n\n{}\n\n", format!("para {i} ").repeat(400)));
        }
        let sections = split_sections(&doc, 10_000);
        assert!(sections.len() > 1, "must split: {} chars", doc.len());
        let total: usize = sections.iter().map(|s| s.body.len()).sum();
        // Nothing silently dropped (joins differ only by whitespace).
        assert!(total as f64 > doc.len() as f64 * 0.95, "{total} vs {}", doc.len());
        assert!(sections.iter().all(|s| s.body.len() <= 12_000));
        assert_eq!(sections[0].title.as_deref(), Some("Chapter 0"));
    }

    #[test]
    fn one_giant_paragraph_is_hard_split_not_dropped() {
        let doc = "x".repeat(50_000);
        let sections = split_sections(&doc, 10_000);
        let total: usize = sections.iter().map(|s| s.body.len()).sum();
        assert_eq!(total, 50_000);
    }
}
