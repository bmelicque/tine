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
        let start_range = self.eat(&[Token::Let]);

        let pattern = self.with_mutable_binding(|self_| self_.parse_pattern());
        if pattern.is_none() {
            let loc = self.localize(start_range.clone()).increment();
            self.error(DiagnosticKind::MissingPattern, loc);
        }

        let type_annotation = match self.tokens.peek() {
            Some((Ok(Token::Colon), _)) => self.parse_type(),
            _ => None,
        };

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
            annotation: type_annotation,
            value: value.into(),
        }
    }
}
