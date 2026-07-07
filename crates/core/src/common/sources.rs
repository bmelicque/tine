use crate::Span;

#[derive(Debug, Default)]
pub struct Source {
    text: String,
    line_offsets: Vec<u32>,
}

impl Source {
    pub fn new(src: &str) -> Self {
        Self {
            text: src.to_string(),
            line_offsets: build_line_starts(src),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn line_col(&self, pos: u32) -> (usize, usize) {
        let line = match self.line_offsets.binary_search(&pos) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        (line, (pos - self.line_offsets[line]) as usize)
    }

    pub fn read_line(&self, line: usize) -> &str {
        let start = self.line_offsets[line] as usize;
        let end = if line == self.line_offsets.len() - 1 {
            self.text.len()
        } else {
            self.line_offsets[line + 1] as usize
        };
        &self.text[start..end]
    }

    pub fn read_span(&self, span: Span) -> &str {
        let start = span.start() as usize;
        let end = span.end() as usize;
        &self.text[start..end]
    }
}

impl From<String> for Source {
    fn from(text: String) -> Self {
        let line_starts = build_line_starts(&text);
        Self {
            text,
            line_offsets: line_starts,
        }
    }
}

fn build_line_starts(text: &str) -> Vec<u32> {
    let mut starts = vec![0];
    for (i, char) in text.char_indices() {
        if char == '\n' {
            let pos = (i + char.len_utf8()) as u32;
            starts.push(pos);
        }
    }
    starts
}
