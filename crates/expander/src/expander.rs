use tine_common::{
    diagnostics::{Diagnostic, DiagnosticKind},
    locations::Location,
};

#[derive(Default)]
pub(crate) struct Expander {
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl Expander {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn error(&mut self, at: Location, kind: DiagnosticKind) {
        self.diagnostics.push(Diagnostic::error(at, kind));
    }
}
