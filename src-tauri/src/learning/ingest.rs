//! L6 — upload ingest. Text/markdown uploads are read in the frontend; PDFs need
//! server-side extraction (pure-Rust `pdf-extract`). Degrades gracefully: a
//! malformed, encrypted, or image-only (scanned) PDF returns a clear, actionable
//! error so the user can paste the text instead — never a crash or blank.

const MAX_CHARS: usize = 40_000;

/// Extract readable text from an uploaded PDF's bytes, bounded and panic-safe.
pub fn extract_pdf(bytes: &[u8]) -> Result<String, String> {
    if bytes.is_empty() {
        return Err("That file was empty.".into());
    }
    // pdf-extract parses untrusted input and can panic on some malformed PDFs;
    // contain any panic and turn it into a clean error.
    let extracted = std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem(bytes))
        .map_err(|_| {
            "Couldn't read that PDF — it may be malformed or password-protected. Paste the text instead.".to_string()
        })?;
    let text = extracted.map_err(|e| {
        format!("Couldn't extract text from that PDF ({e}). If it's scanned/image-only, paste the text instead.")
    })?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("That PDF has no selectable text (it may be scanned images). Paste the text instead.".into());
    }
    Ok(trimmed.chars().take(MAX_CHARS).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_garbage_input_error_gracefully_not_panic() {
        assert!(extract_pdf(&[]).is_err(), "empty input is a clean error");
        // Random non-PDF bytes must not panic the process — just error.
        let garbage: Vec<u8> = (0..256u32).map(|i| (i % 256) as u8).collect();
        assert!(extract_pdf(&garbage).is_err(), "non-PDF bytes error, never panic");
    }
}
