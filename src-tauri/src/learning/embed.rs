//! Local embeddings for study-material retrieval (Phase 21). One entry point —
//! the `Embedder` trait — mirroring the ModelProvider discipline. The real
//! implementation talks to Ollama's /api/embed with `nomic-embed-text`
//! (768-dim). The nomic task prefixes are NOT optional: embedding without
//! `search_document:` / `search_query:` silently craters retrieval quality.
//!
//! Unavailability is a capability, not an error: chunks store with NULL
//! embeddings, retrieval degrades to keyword (FTS5) only, and a backfill can
//! embed NULL rows the next time Ollama is up.

use std::time::Duration;

pub const EMBED_MODEL: &str = "nomic-embed-text";
pub const EMBED_DIM: usize = 768;

pub trait Embedder: Send + Sync {
    /// Embed document chunks (the `search_document:` side). One batched call.
    fn embed_documents(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String>;
    /// Embed a query (the `search_query:` side).
    fn embed_query(&self, text: &str) -> Result<Vec<f32>, String>;
}

pub struct OllamaEmbedder {
    endpoint: String,
}

impl Default for OllamaEmbedder {
    fn default() -> Self {
        OllamaEmbedder { endpoint: "http://localhost:11434".into() }
    }
}

impl OllamaEmbedder {
    /// Fast capability probe — reports unavailable within ~2s, never hangs.
    pub fn available(&self) -> bool {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .ok()
            .and_then(|c| c.get(format!("{}/api/version", self.endpoint)).send().ok())
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    fn call(&self, inputs: Vec<String>) -> Result<Vec<Vec<f32>>, String> {
        #[derive(serde::Deserialize)]
        struct Resp {
            embeddings: Vec<Vec<f32>>,
        }
        let resp = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| e.to_string())?
            .post(format!("{}/api/embed", self.endpoint))
            .json(&serde_json::json!({ "model": EMBED_MODEL, "input": inputs }))
            .send()
            .map_err(|e| format!("Ollama isn't reachable ({e}). Start it to enable semantic search."))?;
        if !resp.status().is_success() {
            return Err(format!(
                "Ollama embedding failed (HTTP {}). Is `{EMBED_MODEL}` pulled? Try: ollama pull {EMBED_MODEL}",
                resp.status()
            ));
        }
        let parsed: Resp = resp.json().map_err(|e| e.to_string())?;
        for v in &parsed.embeddings {
            if v.len() != EMBED_DIM {
                return Err(format!("Unexpected embedding dimension {} (wanted {EMBED_DIM}).", v.len()));
            }
        }
        Ok(parsed.embeddings)
    }
}

impl Embedder for OllamaEmbedder {
    fn embed_documents(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        self.call(texts.iter().map(|t| format!("search_document: {t}")).collect())
    }

    fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
        self.call(vec![format!("search_query: {text}")])
            .map(|mut v| v.pop().unwrap_or_default())
    }
}

pub fn to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

pub fn from_blob(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

#[cfg(test)]
pub mod test_support {
    use super::*;

    /// Deterministic toy embedder for tests: a bag-of-words hash projection.
    /// Same words → similar vectors; no network, no Ollama.
    pub struct FakeEmbedder;

    fn project(text: &str) -> Vec<f32> {
        let mut v = vec![0.0f32; 64];
        for word in text.to_lowercase().split(|c: char| !c.is_alphanumeric()) {
            if word.len() < 3 {
                continue;
            }
            let mut h: u64 = 1469598103934665603;
            for b in word.bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(1099511628211);
            }
            v[(h % 64) as usize] += 1.0;
        }
        v
    }

    impl Embedder for FakeEmbedder {
        fn embed_documents(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
            Ok(texts.iter().map(|t| project(t)).collect())
        }
        fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
            Ok(project(text))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_round_trip_preserves_vectors() {
        let v = vec![0.25f32, -1.5, 3.125];
        assert_eq!(from_blob(&to_blob(&v)), v);
    }

    #[test]
    fn cosine_basics() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert_eq!(cosine(&[], &[]), 0.0);
    }
}
