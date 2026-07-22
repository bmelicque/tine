use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{maybe_vec_loc, vec_loc, Locatable, Location},
};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_trait_definition(
        &mut self,
        docs: Option<ast::Docs>,
        pub_loc: Option<Location>,
    ) -> ast::TraitDefinition {
        let kw_range = self.eat(&[Token::Trait]);
        let kw_loc = self.localize(kw_range);
        let mut loc = pub_loc.map_or(kw_loc, |l| Location::merge(l, kw_loc));
        let public = pub_loc.is_some();

        let Ok(type_name) = self.parse_type_name(&[Token::LBrace]) else {
            return ast::TraitDefinition {
                loc,
                docs,
                public,
                name: None,
                params: None,
                methods: None,
            };
        };
        if let Some(type_name) = &type_name {
            loc = Location::merge(loc, type_name.loc);
        }

        let body = self.parse_trait_body();
        loc = body.as_ref().map_or(loc, |(_, l)| Location::merge(loc, *l));
        let methods = body.map(|(b, _)| b);

        let (name, params) = type_name.map_or((None, None), |n| (Some(n.name), n.params));
        ast::TraitDefinition {
            loc,
            docs,
            public,
            name,
            params,
            methods,
        }
    }

    fn parse_trait_body(&mut self) -> Option<(Vec<ast::TraitMethod>, Location)> {
        let start_range = match self.tokens.peek() {
            Some((Ok(Token::LBrace), r)) => r.clone(),
            _ => return None,
        };

        let methods = self.parse_list(
            |p| p.maybe_parse_method_signature(),
            Token::Comma,
            Token::RBrace,
        );

        let end_range = self.expect(Token::RBrace);
        let loc = self.localize(start_range.start..end_range.end);

        Some((methods, loc))
    }

    fn maybe_parse_method_signature(&mut self) -> Option<ast::TraitMethod> {
        let receiver = self.maybe_parse_receiver();
        let name = self.maybe_parse_name();
        let type_params = self.maybe_parse_type_params();
        let params = self.maybe_parse_function_params();
        let return_annotation = self.maybe_parse_return_type();

        let loc = maybe_vec_loc(&[
            name.as_ref().map(|i| i.loc),
            type_params.as_ref().and_then(|p| vec_loc(p)),
            params.as_ref().map(|p| p.loc),
            return_annotation.as_ref().map(|r| r.loc()),
        ])?;

        Some(ast::TraitMethod {
            loc,
            receiver,
            name,
            type_params,
            params,
            return_annotation,
        })
    }

    fn maybe_parse_receiver(&mut self) -> Option<ast::Identifier> {
        self.try_parse(Self::parse_receiver_helper, Self::is_receiver_sync)
            .ok()
            .flatten()
    }

    fn parse_receiver_helper(parser: &mut Parser<'_>) -> Option<ast::Identifier> {
        parser.maybe_eat(|t| t.lparen())?;
        let (text, loc) = parser.maybe_eat(|t| t.identifier().map(|s| s.to_owned()))?;
        parser.maybe_eat(|t| t.rparen())?;
        Some(ast::Identifier { text, loc })
    }

    fn is_receiver_sync(token: &Token) -> bool {
        matches!(
            token,
            Token::Ident(_) | Token::Lt | Token::LParen | Token::Newline | Token::RBrace
        )
    }

    fn maybe_parse_name(&mut self) -> Option<ast::Identifier> {
        let name_result = self.try_parse(
            |self_| {
                let (text, loc) = self_.maybe_eat(|t| t.identifier().map(|s| s.to_owned()))?;
                Some(ast::Identifier { text, loc })
            },
            |t| {
                matches!(
                    t,
                    Token::Lt | Token::LParen | Token::Newline | Token::RBrace
                )
            },
        );

        match name_result {
            Ok(Some(name)) => Some(name),
            Ok(None) => {
                let loc = self.next_loc();
                self.error(DiagnosticKind::MissingName, loc);
                None
            }
            Err(loc) => {
                self.error(DiagnosticKind::MissingName, loc);
                None
            }
        }
    }

    fn maybe_parse_type_params(&mut self) -> Option<Vec<ast::Identifier>> {
        self.try_parse(
            |self_| {
                self_.maybe_is(|t| matches!(t, Token::Lt)).then_some(())?;
                Some(self_.parse_type_params())
            },
            |t| matches!(t, Token::LParen | Token::Newline | Token::RBrace),
        )
        .ok()?
    }

    fn maybe_parse_function_params(&mut self) -> Option<ast::FunctionParams> {
        let result = self.try_parse(
            |self_| {
                self_
                    .maybe_is(|t| matches!(t, Token::LParen))
                    .then_some(())?;
                self_.parse_function_params()
            },
            |t| matches!(t, Token::RBrace),
        );

        match result {
            Ok(Some(t)) => Some(t),
            Ok(None) => {
                let loc = self.next_loc();
                self.error(DiagnosticKind::MissingParams, loc);
                None
            }
            Err(loc) => {
                self.error(DiagnosticKind::MissingParams, loc);
                None
            }
        }
    }

    fn maybe_parse_return_type(&mut self) -> Option<ast::Type> {
        self.try_parse(
            |self_| {
                self_.eat_if(&[Token::Colon])?;
                self_.parse_type()
            },
            |t| matches!(t, Token::Newline | Token::RBrace),
        )
        .ok()?
    }
}
