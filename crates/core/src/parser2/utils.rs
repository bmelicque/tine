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
                            self.recover_before(&recover_before);
                        }
                    };
                    self.skip_next_if(&separator);
                }
            }
        }

        elements
    }
}
