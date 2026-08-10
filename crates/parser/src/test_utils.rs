use tine_ast as ast;

use crate::{Diagnostic, Parser};

pub(super) struct ExpressionTest<'parser> {
    pub input: &'parser str,
    pub expected: ast::Expression,
    pub diagnostics: Vec<Diagnostic>,
}

#[cfg(test)]
pub(super) fn parse_expression(src: &str) -> Result<ast::Expression, Vec<Diagnostic>> {
    let mut parser = Parser::new(0, src);
    match parser.parse_expression() {
        Some(expr) => Ok(expr),
        _ => Err(parser.diagnostics),
    }
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

#[cfg(test)]
pub(super) fn parse_statement(
    src: &str,
) -> Result<ast::Statement, (Option<ast::Statement>, Vec<Diagnostic>)> {
    let mut parser = Parser::new(0, src);
    let result = parser.parse_statement();
    match (result, parser.diagnostics.len()) {
        (Some(stmt), 0) => Ok(stmt),
        (r, _) => Err((r, parser.diagnostics)),
    }
}
