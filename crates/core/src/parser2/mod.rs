mod expressions;
mod patterns;
mod statements;
mod types;

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

    pub(super) fn next_range(&mut self) -> Range<usize> {
        match self.tokens.peek() {
            Some((_, range)) => range.clone(),
            None => {
                let start = self.src.len();
                start..start
            }
        }
    }

    pub(super) fn eat(&mut self, tokens: &[Token]) -> Range<usize> {
        match self.tokens.next() {
            Some((Ok(tok), range)) if tokens.contains(&tok) => range,
            _ => panic!("expected one of {:?}", tokens),
        }
    }

    pub(super) fn expect(&mut self, token: Token) -> Range<usize> {
        match self.tokens.peek() {
            Some((Ok(tok), r)) if tok == &token => {
                let range = r.clone();
                self.tokens.next(); // consume the token
                range
            }
            _ => self.recover_at(&[token]),
        }
    }

    pub(super) fn expect_either(&mut self, tokens: &[Token]) {
        match self.tokens.peek() {
            Some((Ok(tok), r)) if tokens.contains(tok) => {}
            _ => {
                self.recover_before(tokens, &[]);
            }
        }
    }

    /// Eat the next token if it passes the given test.
    ///
    /// If the test is passed, this function returns `Ok((token, token_range))`.
    ///
    /// If the test doesn't pass, tokens will be consumed until either:
    /// - a token that pass the test is found
    /// - a 'sync' token is found (see `sync` parameter) (it is not consumed)
    /// - the end of the file is reached
    /// Then the function returns `Err(skipped_range)`
    ///
    /// This method does not push any diagnostic by itself, this should be handled by the calling method.
    pub(super) fn better_expect<F, T>(
        &mut self,
        test: F,
        sync: &[Token],
    ) -> Result<(T, Range<usize>), Range<usize>>
    where
        F: Fn(&Token) -> Option<T>,
    {
        match self.tokens.peek() {
            Some((Ok(token), range)) => match test(token) {
                Some(data) => {
                    let range = range.clone();
                    self.tokens.next();
                    return Ok((data, range));
                }
                None => {}
            },
            Some(_) => {}
            None => return Err(self.next_range()),
        }

        let mut range = self
            .tokens
            .peek()
            .map(|token| token.1.clone())
            .unwrap_or(self.src.len()..self.src.len());

        while let Some(token) = &self.tokens.peek() {
            match token {
                (Ok(t), r) if test(t).is_some() || sync.contains(t) => {
                    break;
                }
                (_, r) => {
                    range.end = r.end;
                    self.tokens.next();
                }
            }
        }
        Err(range)
    }

    pub(super) fn recover_before(&mut self, tokens: &[Token], sync: &[Token]) -> Range<usize> {
        let mut range = self
            .tokens
            .peek()
            .map(|token| token.1.clone())
            .unwrap_or(self.src.len()..self.src.len());
        while let Some(token) = &self.tokens.peek() {
            match token {
                (Ok(t), r) if tokens.contains(t) || sync.contains(t) => {
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
        let mut range = self.recover_before(token, &[]);
        match self.tokens.next() {
            Some((_, r)) => {
                range.end = r.end;
                range
            }
            None => {
                range.end = self.src.len();
                range
            }
        }
    }

    pub fn error(&mut self, kind: DiagnosticKind, loc: Location) {
        self.diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Error,
            loc,
            kind,
        });
    }
}
