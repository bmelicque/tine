use crate::{
    ast,
    parser::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_variable_declaration(
        &mut self,
        docs: Option<ast::Docs>,
    ) -> ast::VariableDeclaration {
        let Some((Ok(token), start_range)) = self.tokens.next() else {
            panic!();
        };
        let keyword = match token {
            Token::Const => ast::DeclarationKeyword::Const,
            Token::Var => ast::DeclarationKeyword::Var,
            _ => panic!(),
        };

        let pattern = match self.parse_pattern() {
            Some(p) => p,
            None => {
                let loc = self.localize(start_range).increment();
                self.error(DiagnosticKind::MissingPattern, loc);
                todo!("Pattern should be allowed to be None here");
            }
        };

        let op_range = self.expect(Token::Eq);
        let (value, loc) = match self.parse_expression() {
            Some(v) => {
                let loc = v.loc();
                (v, Location::merge(self.localize(start_range), loc))
            }
            None => {
                self.error(
                    DiagnosticKind::MissingExpression,
                    self.localize(op_range.clone()).increment(),
                );
                (
                    ast::Expression::Empty,
                    Location::merge(self.localize(start_range), self.localize(op_range)),
                )
            }
        };

        ast::VariableDeclaration {
            docs,
            loc,
            keyword,
            pattern: Box::new(pattern),
            value: Box::new(value),
        }
    }
}
