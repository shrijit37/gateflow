//! Streaming-protocol detection (OpenAI / Anthropic / Gemini / Raw).
//!
//! Detection order:
//! 1. Explicit override: `X-Gateflow-Protocol` request header.
//! 2. Strong header signals: `anthropic-version` → Anthropic,
//!    `x-goog-api-key` → Gemini.
//! 3. Payload sniffing on a small body prefix.
//! 4. Fallback: [`Protocol::Raw`].

use gateflow_engine::Protocol;
use http::HeaderMap;

/// Explicit override header.
pub const OVERRIDE_HEADER: &str = "x-gateflow-protocol";

const SNIFF_WINDOW: usize = 4096;

/// Everything the data-plane has without streaming the whole body.
#[derive(Debug, Clone, Default)]
pub struct DetectHints {
    pub path: String,
    pub headers: HeaderMap,
    /// First bytes of the request body (<= 4 KiB).
    pub body_prefix: Vec<u8>,
}

pub fn detect(hints: &DetectHints) -> Protocol {
    if let Some(override_val) = hints
        .headers
        .get(OVERRIDE_HEADER)
        .and_then(|v| v.to_str().ok())
    {
        return match override_val.trim().to_ascii_lowercase().as_str() {
            "openai" | "open-ai" => Protocol::Openai,
            "anthropic" | "claude" => Protocol::Anthropic,
            "gemini" | "google" => Protocol::Gemini,
            "raw" => Protocol::Raw,
            _ => sniff_body(&hints.body_prefix),
        };
    }

    // Header signals (cheap, unambiguous).
    if hints.headers.contains_key("anthropic-version") {
        return Protocol::Anthropic;
    }
    if hints.headers.contains_key("x-goog-api-key") {
        return Protocol::Gemini;
    }
    if hints
        .headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.trim_start().starts_with("Bearer sk-ant-"))
    {
        return Protocol::Anthropic;
    }

    sniff_body(&hints.body_prefix)
}

/// Byte-substring sniffing over the window. Never parses the body.
fn sniff_body(prefix: &[u8]) -> Protocol {
    let window = &prefix[..prefix.len().min(SNIFF_WINDOW)];
    if contains(window, b"\"anthropic_version\"") {
        return Protocol::Anthropic;
    }
    if contains(window, b"\"generationConfig\"") || contains(window, b"\"systemInstruction\"") {
        return Protocol::Gemini;
    }
    if contains(window, b"\"contents\"") && contains(window, b"\"model\"") {
        return Protocol::Gemini;
    }
    if contains(window, b"\"messages\"") && contains(window, b"\"stream\"") {
        return Protocol::Openai;
    }
    Protocol::Raw
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hints(prefix: &[u8]) -> DetectHints {
        DetectHints {
            path: "/v1/chat/completions".into(),
            headers: HeaderMap::new(),
            body_prefix: prefix.to_vec(),
        }
    }

    #[test]
    fn override_header_wins() {
        let mut h = hints(b"{}");
        h.headers.insert(OVERRIDE_HEADER, "gemini".parse().unwrap());
        assert_eq!(detect(&h), Protocol::Gemini);
    }

    #[test]
    fn anthropic_version_header() {
        let mut h = hints(b"{}");
        h.headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
        assert_eq!(detect(&h), Protocol::Anthropic);
    }

    #[test]
    fn gemini_api_key_header() {
        let mut h = hints(b"{}");
        h.headers.insert("x-goog-api-key", "abc".parse().unwrap());
        assert_eq!(detect(&h), Protocol::Gemini);
    }

    #[test]
    fn anthropic_bearer_token() {
        let mut h = hints(b"{}");
        h.headers.insert(
            "authorization",
            "Bearer sk-ant-api03-foo".parse().unwrap(),
        );
        assert_eq!(detect(&h), Protocol::Anthropic);
    }

    #[test]
    fn openai_payload() {
        let body = br#"{"model":"gpt-4o","stream":true,"messages":[{"role":"user","content":"hi"}]}"#;
        assert_eq!(detect(&hints(body)), Protocol::Openai);
    }

    #[test]
    fn anthropic_payload() {
        let body = br#"{"model":"claude-sonnet-4-5","max_tokens":512,"anthropic_version":"2023-06-01","messages":[{"role":"user","content":"hi"}]}"#;
        assert_eq!(detect(&hints(body)), Protocol::Anthropic);
    }

    #[test]
    fn gemini_payload() {
        let body = br#"{"contents":[{"parts":[{"text":"hi"}]}],"generationConfig":{"temperature":0.7}}"#;
        assert_eq!(detect(&hints(body)), Protocol::Gemini);
    }

    #[test]
    fn unknown_payload_falls_back_to_raw() {
        assert_eq!(detect(&hints(b"garbage bytes")), Protocol::Raw);
    }
}