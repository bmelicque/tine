use std::cmp::{max, min, Ordering};

use crate::module_path::ModuleId;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        let start = min(first.start, second.start);
        let end = max(first.end, second.end);
        Self { start, end }
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

    /// Make a 1 char long span ending just before the start of given span
    pub fn decrement(&self) -> Self {
        let end = self.start;
        let start = if end > 0 { end - 1 } else { 0 };
        Self::new(start, end)
    }

    /// Get the char at `pos` (0-based index)
    pub fn nth_char(&self, pos: u32) -> Self {
        let start = self.start + pos;
        let end = start + 1;
        Self { start, end }
    }

    pub fn is_within(&self, test: Self) -> bool {
        self.start >= test.start && self.end <= test.end
    }

    pub fn contains(&self, test: u32) -> bool {
        self.start <= test && self.end >= test
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Location {
    module: ModuleId,
    span: Span,
}

impl Location {
    pub fn new(module: ModuleId, span: Span) -> Self {
        Self { module, span }
    }

    pub fn dummy() -> Self {
        Self {
            module: 0,
            span: Span::dummy(),
        }
    }

    pub fn merge(first: Self, second: Self) -> Self {
        if first.module != second.module {
            panic!()
        }
        let span = Span::merge(first.span, second.span);
        Self {
            module: first.module,
            span,
        }
    }

    pub fn module(&self) -> ModuleId {
        self.module
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn is_within(&self, test: Self) -> bool {
        self.module == test.module && self.span.is_within(test.span)
    }

    pub fn increment(&self) -> Location {
        let span = self.span.increment();
        Location {
            module: self.module,
            span,
        }
    }

    pub fn decrement(&self) -> Location {
        let span = self.span.decrement();
        Location {
            module: self.module,
            span,
        }
    }

    pub fn nth_char(&self, pos: u32) -> Self {
        let span = self.span.nth_char(pos);
        Location {
            module: self.module,
            span,
        }
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.module != other.module {
            return None;
        }
        if self.span.end < other.span.start {
            return Some(Ordering::Less);
        }
        if self.span.start > other.span.end {
            return Some(Ordering::Greater);
        }
        None
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

pub trait Locatable {
    fn loc(&self) -> Location;
}

pub fn vec_loc<L: Locatable>(vec: &Vec<L>) -> Option<Location> {
    let first = vec.first()?;
    let last = vec.last()?;
    Some(Location::merge(first.loc(), last.loc()))
}

pub fn maybe_vec_loc(vec: &[Option<Location>]) -> Option<Location> {
    let first = vec.iter().flatten().next()?;
    let last = vec.iter().flatten().next_back()?;
    Some(Location::merge(*first, *last))
}
