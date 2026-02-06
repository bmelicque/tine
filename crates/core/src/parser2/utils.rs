use std::ops::Range;

use crate::{
    parser2::{tokens::Token, Parser},
    DiagnosticKind,
};

impl Parser<'_> {
    pub(super) fn skip_next_if(&mut self, is: &Token) {
        if let Some((Ok(t), _)) = self.tokens.peek().cloned() {
            if t == *is {
                self.tokens.next();
            }
        }
    }

    /// Parse a list of elements, separated by `separator` then an optional newline.
    /// Parsing stops right before it finds a `terminator` token, without consuming it.
    pub(super) fn parse_list<F, R>(
        &mut self,
        parser: F,
        separator: Token,
        terminator: Token,
    ) -> Vec<R>
    where
        F: Fn(&mut Parser<'_>) -> Option<R>,
    {
        let mut elements = Vec::new();
        let recover_before = [Token::Newline, separator.clone(), terminator.clone()];

        while let Some((Ok(token), range)) = self.tokens.peek().cloned() {
            match token.clone() {
                Token::Newline => {
                    self.tokens.next();
                }
                t if t == terminator => break,
                t if t == separator => {
                    let loc = self.localize(range.clone());
                    let error = DiagnosticKind::UnexpectedToken {
                        token: separator.to_string(),
                    };
                    self.error(error, loc);
                    self.tokens.next();
                    self.skip_next_if(&Token::Newline);
                }
                _ => {
                    if let Some(element) = parser(self) {
                        elements.push(element);
                    }
                    if let Some((Ok(token), _)) = self.tokens.peek().cloned() {
                        if !recover_before.contains(&token) {
                            self.recover_before(&recover_before, &[]);
                        }
                    };
                    self.skip_next_if(&separator);
                }
            }
        }

        elements
    }

    pub(super) fn parse_type_params(&mut self) -> Vec<String> {
        self.eat(&[Token::Lt]);
        let params = self.parse_list(|p| p.parse_type_param(), Token::Comma, Token::Gt);
        self.expect(Token::Gt);
        params
    }

    fn parse_type_param(&mut self) -> Option<String> {
        unimplemented!()
    }

    pub(super) fn report_missing(&mut self, diagnostic: DiagnosticKind) -> Range<usize> {
        let range = self.next_range();
        self.error(diagnostic, self.localize(range.clone()));
        range
    }
}

pub fn is_type_name(name: &str) -> bool {
    name.chars()
        .filter(|c| *c != '_')
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
}

pub fn normalize_doc_comment(input: &str) -> String {
    let mut paragraphs = Vec::new();
    let mut current = Vec::new();

    for line in input.lines() {
        let line = line
            .trim_start()
            .strip_prefix("//")
            .unwrap_or(line)
            .trim_start();

        if !line.is_empty() {
            current.push(line.to_string());
        } else if !current.is_empty() {
            paragraphs.push(current.join(" "));
            current.clear();
        }
    }

    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }

    paragraphs.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_type_name() {
        assert!(is_type_name("FooBar"));
        assert!(is_type_name("FooBar123"));
        assert!(is_type_name("_FooBar"));
        assert!(!is_type_name("foo_bar"));
        assert!(!is_type_name("_foo_bar"));
    }
}
