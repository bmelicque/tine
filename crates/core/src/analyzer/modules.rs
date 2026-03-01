use enum_from_derive::EnumFrom;

use crate::locations::Span;

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModulePath {
    /// A file path to a file in the project.
    ///
    /// This is expected to be an absolute, canonical path.
    Real(std::path::PathBuf),
    /// The name of another module, usually from the standard library or a
    /// project dependency
    Virtual(String),
}
impl From<&std::path::PathBuf> for ModulePath {
    fn from(value: &std::path::PathBuf) -> Self {
        Self::Real(value.clone())
    }
}
impl From<&str> for ModulePath {
    fn from(value: &str) -> Self {
        Self::Virtual(value.to_string())
    }
}
impl std::fmt::Display for ModulePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModulePath::Real(p) => write!(f, "{}", p.display()),
            ModulePath::Virtual(c) => write!(f, "{}", c),
        }
    }
}

pub type ModuleId = usize;

#[derive(Debug, Clone)]
pub struct Module {
    pub name: ModulePath,
    pub src: Source,
}

#[derive(Debug, Clone)]
pub struct Source {
    text: String,
    line_starts: Vec<u32>,
}

impl Source {
    pub fn new(src: &str) -> Self {
        Self {
            text: src.to_string(),
            line_starts: build_line_starts(src),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn line_col(&self, pos: u32) -> (usize, usize) {
        let line = match self.line_starts.binary_search(&pos) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        (line, (pos - self.line_starts[line]) as usize)
    }

    pub fn read_line(&self, line: usize) -> &str {
        let start = self.line_starts[line] as usize;
        let end = if line == self.line_starts.len() - 1 {
            self.text.len()
        } else {
            self.line_starts[line + 1] as usize
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
        Self { text, line_starts }
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
