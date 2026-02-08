use crate::{
    ast,
    parser2::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_match_expression(&mut self) -> ast::MatchExpression {
        let start_range = self.eat(&[Token::Match]);
        let start_loc = self.localize(start_range);

        let scrutinee = self.parse_expression_without_block();
        if scrutinee.is_none() {
            let loc = self.next_loc();
            self.error(DiagnosticKind::MissingExpression, loc);
        }

        match self.tokens.peek() {
            Some((Ok(Token::LBrace), _)) => {}
            _ => {
                let error_loc = self.next_loc();
                self.error(DiagnosticKind::MissingBody, error_loc);
                let loc = match &scrutinee {
                    Some(expr) => Location::merge(start_loc, expr.loc()),
                    None => start_loc,
                };
                return ast::MatchExpression {
                    loc,
                    scrutinee: scrutinee.map(|s| Box::new(s)),
                    arms: None,
                };
            }
        }

        self.tokens.next(); // eat the { token

        let arms = self.parse_list(|p| p.parse_match_arm(), Token::Newline, Token::RBrace);

        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RBrace), _)) => self.eat(&[Token::RBrace]),
            _ => self.next_range(),
        };
        let end_loc = self.localize(end_range);

        ast::MatchExpression {
            loc: Location::merge(start_loc, end_loc),
            scrutinee: scrutinee.map(|s| Box::new(s)),
            arms: Some(arms),
        }
    }

    fn parse_match_arm(&mut self) -> Option<ast::MatchArm> {
        let pattern = self.parse_pattern();
        if pattern.is_none() {
            let error_loc = self.next_loc();
            self.error(DiagnosticKind::MissingPattern, error_loc);
        }
        let Some((Ok(Token::FatArrow), r)) = self.tokens.peek() else {
            let error = DiagnosticKind::ExpectedToken {
                expected: vec![Token::FatArrow.to_string()],
            };
            let error_loc = self.next_loc();
            self.error(error, error_loc);
            let loc = match &pattern {
                Some(pattern) => pattern.loc(),
                None => return None,
            };
            self.sync(&[Token::Newline, Token::RBrace]);
            return Some(ast::MatchArm {
                loc,
                pattern: pattern.map(|p| Box::new(p)),
                expression: None,
            });
        };
        let arrow_range = r.clone();
        let arrow_loc = self.localize(arrow_range);
        self.tokens.next();

        let expression = self.parse_expression();

        let loc = match (&pattern, &expression) {
            (Some(p), Some(e)) => Location::merge(p.loc(), e.loc()),
            (Some(p), None) => Location::merge(p.loc(), arrow_loc),
            (None, Some(e)) => Location::merge(arrow_loc, e.loc()),
            (None, None) => arrow_loc,
        };

        Some(ast::MatchArm {
            loc,
            pattern: pattern.map(|p| Box::new(p)),
            expression: expression.map(|e| Box::new(e)),
        })
    }
}
