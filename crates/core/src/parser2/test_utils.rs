use crate::{ast, parser2::Parser, Diagnostic};

pub(super) struct ExpressionTest<'parser> {
    pub input: &'parser str,
    pub expected: ast::Expression,
    pub diagnostics: Vec<Diagnostic>,
}

pub(super) fn test_expression(test: ExpressionTest) {
    let mut parser = Parser::new(0, test.input);
    let result = parser.parse_expression();
    assert_eq!(result, Some(test.expected));
    assert_eq!(parser.diagnostics, test.diagnostics);
}

pub(super) struct StatementTest<'parser> {
    pub input: &'parser str,
    pub expected: ast::Statement,
    pub diagnostics: Vec<Diagnostic>,
}

pub(super) fn test_statement(test: StatementTest) {
    let mut parser = Parser::new(0, test.input);
    let result = parser.parse_statement();
    assert_eq!(result, Some(test.expected));
    assert_eq!(parser.diagnostics, test.diagnostics);
}
