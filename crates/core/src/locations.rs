use crate::analyzer::ModuleId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    start: u32,
    end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }

    pub fn merge(first: Self, second: Self) -> Self {
        if first.start > second.start || first.end < second.end {
            panic!()
        }
        Self {
            start: first.start,
            end: second.end,
        }
    }

    pub fn start(&self) -> u32 {
        return self.start;
    }

    pub fn end(&self) -> u32 {
        return self.end;
    }

    /// Make a 1 char long span starting just after the end of given span
    pub fn increment(&self) -> Self {
        let end = self.end;
        Self::new(end, end + 1)
    }

    pub fn is_within(&self, test: Self) -> bool {
        self.start >= test.start && self.end <= test.end
    }
}

impl From<pest::Span<'_>> for Span {
    fn from(value: pest::Span) -> Self {
        Self {
            start: value.start() as u32,
            end: value.end() as u32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Location {
    module: ModuleId,
    span: Span,
}
