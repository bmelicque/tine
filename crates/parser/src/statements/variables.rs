use tine_ast as ast;
use tine_common::{diagnostics::DiagnosticKind, locations::Location};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_variable_declaration(
        &mut self,
        docs: Option<ast::Docs>,
        pub_loc: Option<Location>,
    ) -> ast::VariableDeclaration {
        let start_range = self.eat(&[Token::Let]);
        let kw_loc = self.localize(start_range);

        let pattern = self.with_mutable_binding(|self_| self_.parse_pattern());
        if pattern.is_none() {
            let loc = kw_loc.increment();
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
        let start_loc = pub_loc.unwrap_or(kw_loc);
        let loc = match &value {
            Some(v) => Location::merge(start_loc, v.loc()),
            None => Location::merge(start_loc, self.localize(op_range)),
        };

        ast::VariableDeclaration {
            docs,
            loc,
            public: pub_loc.is_some(),
            mutable: false,
            pattern,
            annotation: type_annotation,
            value: value.into(),
        }
    }
}
