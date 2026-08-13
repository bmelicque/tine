use std::ops::Range;

use tine_ast::*;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_implementations(&mut self) -> Implementation {
        let start_range = self.eat(&[Token::Impl]);
        let start_loc = self.localize(start_range);

        let implemented_type = match self.tokens.peek() {
            Some((Ok(Token::Ident(_)), _)) => Some(self.parse_named_type()),
            _ => None,
        };
        if implemented_type.is_none() {
            let loc = self.next_loc();
            self.error(DiagnosticKind::MissingName, loc);
        }

        let body = self.parse_implementation_body();
        let loc = match (&implemented_type, &body) {
            (_, Some(b)) => Location::merge(start_loc, b.loc),
            (Some(t), None) => Location::merge(start_loc, t.loc),
            _ => start_loc,
        };

        Implementation {
            loc,
            implemented_type,
            body,
        }
    }

    fn parse_implementation_body(&mut self) -> Option<ImplementationBody> {
        let Some((Ok(Token::LBrace), _)) = self.tokens.peek() else {
            let loc = self.next_loc();
            self.error(DiagnosticKind::MissingBody, loc);
            return None;
        };
        let start_range = self.eat(&[Token::LBrace]);
        let start_loc = self.localize(start_range);

        let items = self.parse_list(|p| p.parse_impl_item(), Token::Newline, Token::RBrace);

        let end_loc = self.close(Token::RBrace);

        Some(ImplementationBody {
            loc: Location::merge(start_loc, end_loc),
            items,
        })
    }

    fn parse_impl_item(&mut self) -> Option<MethodDefinition> {
        let docs = self.maybe_parse_docs();
        let public_range = self.eat_if(&[Token::Pub]);
        self.parse_method_definition(docs, public_range)
    }

    pub fn parse_method_definition(
        &mut self,
        docs: Option<Docs>,
        public_range: Option<Range<usize>>,
    ) -> Option<MethodDefinition> {
        let static_range = self.eat_if(&[Token::Static]);
        let mut_range = self.eat_if(&[Token::Mut]);
        if !self.maybe_is(|t| *t == Token::Fn) {
            let error_loc = self.next_loc();
            let expected = vec!["fn".into()];
            let error = DiagnosticKind::ExpectedToken { expected };
            self.error(error, error_loc);
            self.sync2(|t| matches!(t, Token::Newline | Token::RBrace));
            return None;
        }
        let public = public_range.is_some();
        let static_ = static_range.is_some();
        let mut_ = mut_range.is_some();
        if static_ && mut_ {
            let start = self.localize(static_range.clone().unwrap());
            let end = self.localize(mut_range.clone().unwrap());
            let loc = Location::merge(start, end);
            self.error(DiagnosticKind::StaticMutMethod, loc);
        }
        let f = self.parse_function_expression();
        if f.name.is_none() {
            self.error(DiagnosticKind::MissingName, f.loc);
        }

        if matches!(&f.body, Some(b) if b.as_block().is_none()) {
            let loc = f.body.as_ref().unwrap().loc();
            self.error(DiagnosticKind::ExpectedBlock, loc);
        };
        let loc = [public_range, static_range, mut_range]
            .into_iter()
            .filter_map(|r| r)
            .next()
            .map_or(f.loc, |r| Location::merge(self.localize(r), f.loc));
        Some(MethodDefinition {
            loc,
            docs,
            public,
            static_,
            mut_,
            name: f.name,
            type_params: f.type_params,
            params: f.params,
            return_type: f.return_type,
            body: f.body,
        })
    }

    pub fn maybe_parse_docs(&mut self) -> Option<Docs> {
        if !self.maybe_is(|t| matches!(t, Token::LineComment(_))) {
            return None;
        }
        let start = self.next_range().start;
        Some(self.parse_docs(start))
    }
}

#[cfg(test)]
mod tests {
    use tine_common::locations::Span;

    use crate::test_utils::{test_statement, StatementTest};

    use super::*;

    #[test]
    fn parse_empty_impl() {
        test_statement(StatementTest {
            input: "impl Type {}",
            expected: Statement::Implementation(Implementation {
                loc: Location::new(0, Span::new(0, 12)),
                implemented_type: Some(NamedType {
                    loc: Location::new(0, Span::new(5, 9)),
                    name: Identifier {
                        loc: Location::new(0, Span::new(5, 9)),
                        text: "Type".to_string(),
                    },
                    args: None,
                }),
                body: Some(ImplementationBody {
                    loc: Location::new(0, Span::new(10, 12)),
                    items: vec![],
                }),
            }),
            diagnostics: vec![],
        });
    }
}
