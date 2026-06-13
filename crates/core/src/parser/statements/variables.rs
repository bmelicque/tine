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
        let Some((Ok(_), start_range)) = self.tokens.next() else {
            panic!();
        };

        let pattern = self.with_mutable_binding(|self_| self_.parse_pattern());
        if pattern.is_none() {
            let loc = self.localize(start_range.clone()).increment();
            self.error(DiagnosticKind::MissingPattern, loc);
        }

        let op_range = self.expect(Token::Eq);
        let value = self.parse_expression();
        if value.is_none() {
            self.error(
                DiagnosticKind::MissingExpression,
                self.localize(op_range.clone()).increment(),
            );
        }
        let loc = match &value {
            Some(v) => Location::merge(self.localize(start_range), v.loc()),
            None => Location::merge(self.localize(start_range), self.localize(op_range)),
        };

        ast::VariableDeclaration {
            docs,
            loc,
            mutable: false,
            pattern,
            value: value.into(),
        }
    }
}
