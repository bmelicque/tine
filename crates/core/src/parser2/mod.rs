mod expressions;
#[cfg(test)]
mod test_utils;
mod tokens;
mod utils;

use std::{iter::Peekable, ops::Range};

use crate::{
    ast, parser2::tokens::Token, Diagnostic, DiagnosticKind, DiagnosticLevel, Location, ModuleId,
    Span,
};
use logos::{Logos, SpannedIter};

pub struct Parser<'src> {
    module: ModuleId,
    src: &'src str,
    diagnostics: Vec<Diagnostic>,
    tokens: Peekable<SpannedIter<'src, Token>>,
}

impl<'src> Parser<'src> {
    pub fn new(module: ModuleId, src: &'src str) -> Self {
        let tokens = Token::lexer(src).spanned().peekable();
        Parser {
            module,
            src,
            tokens,
            diagnostics: Vec::new(),
        }
    }

    pub fn localize(&self, range: Range<usize>) -> Location {
        Location::new(self.module, Span::new(range.start as u32, range.end as u32))
    }

    fn parse_expression(&mut self) -> Option<ast::Expression> {
        let Some(peeked) = self.tokens.peek().cloned() else {
            return None;
        };
        let Ok(peeked) = peeked.0.clone() else {
            return Some(ast::Expression::Invalid(ast::InvalidExpression {
                loc: self.localize(peeked.1),
            }));
        };
        //FIXME:
        Some(self.parse_value_expression())
    }

    fn parse_value_expression(&mut self) -> ast::Expression {
        let lhs = self.parse_postfix();
        lhs
    }

    pub(super) fn recover_before(&mut self, tokens: &[Token]) -> Range<usize> {
        let mut range = self
            .tokens
            .peek()
            .map(|token| token.1.clone())
            .unwrap_or(self.src.len()..self.src.len());
        while let Some(token) = &self.tokens.peek() {
            match token {
                (Ok(t), r) if tokens.contains(t) => {
                    break;
                }
                (_, r) => {
                    range.end = r.end;
                    self.tokens.next();
                }
            }
        }
        let error = DiagnosticKind::ExpectedToken {
            expected: tokens.iter().map(|t| t.to_string()).collect(),
        };
        self.error(error, self.localize(range.clone()));
        range
    }

    pub(super) fn recover_at(&mut self, token: &[Token]) -> Range<usize> {
        let mut range = self.recover_before(token);
        let (_, r) = self.tokens.next().unwrap();
        range.end = r.end;
        range
    }

    pub fn error(&mut self, kind: DiagnosticKind, loc: Location) {
        self.diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Error,
            loc,
            kind,
        });
    }
}
