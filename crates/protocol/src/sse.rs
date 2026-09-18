//! Minimal SSE parser for observability/sampling and future converter nodes.
//!
//! Not used in the passthrough hot path. Handles incremental byte chunks,
//! CRLF and LF line endings, `data:`/`event:`/`id:` fields, multi-line data,
//! and comment lines. Emits one [`SseEvent`] per blank line (frame boundary).

use bytes::BytesMut;

/// One parsed SSE event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub id: Option<String>,
    pub data: Vec<u8>,
}

impl SseEvent {
    pub fn data_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }
}

/// Incremental SSE stream parser.
#[derive(Debug, Default)]
pub struct SseParser {
    buf: BytesMut,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a chunk; returns all complete events terminated by the chunk.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buf.extend_from_slice(chunk);
        let mut events = Vec::new();
        // Consume complete frames (terminated by a blank line).
        while let Some(sep) = first_boundary(&self.buf) {
            let frame = self.buf.split_to(sep).to_vec();
            events.push(parse_frame(&frame));
        }
        events
    }

    /// Flush any remaining partial frame (used at end of stream).
    pub fn finish(&mut self) -> Vec<SseEvent> {
        let rest = std::mem::take(&mut self.buf);
        if rest.is_empty() {
            return Vec::new();
        }
        vec![parse_frame(&rest)]
    }
}

/// Byte index just past a frame's blank-line terminator (LF or CRLF), if any.
fn first_boundary(buf: &[u8]) -> Option<usize> {
    for i in 0..buf.len().saturating_sub(1) {
        if buf[i] == b'\n' {
            let next = buf[i + 1];
            if next == b'\n' {
                return Some(i + 2);
            }
            if next == b'\r' && buf.get(i + 2) == Some(&b'\n') {
                return Some(i + 3);
            }
        }
    }
    None
}

fn parse_frame(frame: &[u8]) -> SseEvent {
    let mut event: Option<String> = None;
    let mut id: Option<String> = None;
    let mut data: Vec<u8> = Vec::new();

    let mut line_start = 0;
    let mut i = 0;
    while i <= frame.len() {
        let is_end = i == frame.len();
        let is_line_end = is_end || frame[i] == b'\n';
        if is_line_end {
            let mut line = &frame[line_start..i];
            if line.ends_with(b"\r") {
                line = &line[..line.len() - 1];
            }
            process_line(line, &mut event, &mut id, &mut data);
            line_start = i + 1;
            if is_end {
                break;
            }
        }
        i += 1;
    }

    SseEvent { event, id, data }
}

fn process_line(
    line: &[u8],
    event: &mut Option<String>,
    id: &mut Option<String>,
    data: &mut Vec<u8>,
) {
    if line.is_empty() || line[0] == b':' {
        return; // blank or comment line
    }
    let Some(colon) = line.iter().position(|b| *b == b':') else {
        return; // no colon — ignore
    };
    let field = &line[..colon];
    let mut value = &line[colon + 1..];
    if value.first() == Some(&b' ') {
        value = &value[1..];
    }
    match field {
        b"event" => *event = Some(String::from_utf8_lossy(value).into_owned()),
        b"id" => *id = Some(String::from_utf8_lossy(value).into_owned()),
        b"data" => {
            data.extend_from_slice(value);
            data.push(b'\n');
        }
        b"retry" => { /* ignored in MVP */ }
        _ => { /* unknown field, ignore */ }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_event() {
        let mut p = SseParser::new();
        let events = p.push(b"event: delta\ndata: {\"a\":1}\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.as_deref(), Some("delta"));
        assert_eq!(events[0].data_str(), Some("{\"a\":1}\n"));
    }

    #[test]
    fn parses_chunked_split_frames() {
        let mut p = SseParser::new();
        assert!(p.push(b"event: delta\nda").is_empty());
        let events = p.push(b"ta: chunk1\ndata: chunk2\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data_str(), Some("chunk1\nchunk2\n"));
    }

    #[test]
    fn parses_multiple_events_in_one_chunk() {
        let mut p = SseParser::new();
        let events = p.push(b"data: one\n\ndata: two\n\n");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data_str(), Some("one\n"));
        assert_eq!(events[1].data_str(), Some("two\n"));
    }

    #[test]
    fn handles_crlf_and_comments() {
        let mut p = SseParser::new();
        let events = p.push(b": keepalive\r\ndata: hi\r\n\r\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data_str(), Some("hi\n"));
        assert_eq!(events[0].event, None);
    }

    #[test]
    fn finish_flushes_partial_frame() {
        let mut p = SseParser::new();
        p.push(b"data: partial");
        let events = p.finish();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data_str(), Some("partial\n"));
    }

    #[test]
    fn ignores_unknown_fields() {
        let mut p = SseParser::new();
        let events = p.push(b"retry: 1000\nfoo: x\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, b"");
    }
}
