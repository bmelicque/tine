use crate::{ast, parser2::Parser, Diagnostic};

pub(super) struct Test<'parser> {
    pub input: &'parser str,
    pub expected: ast::Expression,
    pub diagnostics: Vec<Diagnostic>,
}

pub(super) fn run(test: Test) {
    let mut parser = Parser::new(0, test.input);
    let result = parser.parse_expression();
    assert_eq!(result, Some(test.expected));
    assert_eq!(parser.diagnostics, test.diagnostics);
}
