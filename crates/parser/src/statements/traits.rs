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
            Some((Ok(Token::LBrace), _)) => self.eat(&[Token::LBrace]),
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
        let name = Some(self.maybe_parse_trait_method_name()?);
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

    fn maybe_parse_trait_method_name(&mut self) -> Option<ast::Identifier> {
        let recover_at = |t: &Token| {
            matches!(
                t,
                Token::Lt | Token::LParen | Token::Newline | Token::RBrace
            )
        };
        self.try_parse(
            |self_| {
                let (text, loc) = self_.maybe_eat(|t| t.identifier().map(|s| s.to_owned()))?;
                Some(ast::Identifier { text, loc })
            },
            recover_at,
        )
        .ok()?
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

#[cfg(test)]
mod tests {
    use tine_ast as ast;
    use tine_common::diagnostics::DiagnosticKind;

    use crate::Parser;

    fn parse_trait_definition(src: &str) -> (ast::TraitDefinition, Vec<crate::Diagnostic>) {
        let mut parser = Parser::new(0, src);

        let result = parser.parse_trait_definition(None, None);

        (result, parser.diagnostics)
    }

    #[test]
    fn parse_empty_trait() {
        let (trait_def, diags) = parse_trait_definition("trait Foo {}");

        assert!(
            diags.is_empty(),
            "diagnostics should be empty, but got {:?}",
            diags
        );
        assert_eq!(
            trait_def.name.as_ref().map(|n| n.text.as_str()),
            Some("Foo")
        );
        assert!(trait_def.params.is_none());
        assert!(trait_def.methods.as_ref().is_some_and(Vec::is_empty));
        assert!(!trait_def.public);
        assert!(trait_def.docs.is_none());
    }

    #[test]
    fn parse_generic_trait() {
        let (trait_def, diags) = parse_trait_definition("trait Foo<T> {}");

        assert!(diags.is_empty());

        let params = trait_def.params.expect("expected type parameters");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].text, "T");
    }

    #[test]
    fn parse_trait_without_body() {
        let (trait_def, _) = parse_trait_definition("trait Foo");

        assert_eq!(
            trait_def.name.as_ref().map(|n| n.text.as_str()),
            Some("Foo")
        );
        assert!(trait_def.methods.is_none());
    }

    #[test]
    fn parse_trait_with_simple_method() {
        let (trait_def, diags) = parse_trait_definition("trait Foo { bar() }");

        assert!(diags.is_empty());

        let methods = trait_def.methods.expect("expected methods");
        assert_eq!(methods.len(), 1);

        let method = &methods[0];
        assert_eq!(method.name.as_ref().map(|n| n.text.as_str()), Some("bar"));
        assert!(method.receiver.is_none());
        assert!(method.type_params.is_none());
        assert!(method.params.is_some());
        assert!(method.return_annotation.is_none());
    }

    #[test]
    fn parse_trait_with_multiple_methods() {
        let (trait_def, diags) = parse_trait_definition("trait Foo { bar()\n baz() }");

        assert!(
            diags.is_empty(),
            "expected no diagnostics, but got {:?}",
            diags
        );

        let methods = trait_def.methods.expect("expected methods");
        assert_eq!(
            methods.len(),
            2,
            "expected 2 methods, but got {:?}",
            methods
        );

        assert_eq!(
            methods[0].name.as_ref().map(|n| n.text.as_str()),
            Some("bar")
        );
        assert_eq!(
            methods[1].name.as_ref().map(|n| n.text.as_str()),
            Some("baz")
        );
    }

    #[test]
    fn parse_trait_method_with_receiver() {
        let (trait_def, diags) = parse_trait_definition("trait Foo { (self) bar() }");

        assert!(
            diags.is_empty(),
            "expected diagnostics to be empty, but got {:?}",
            diags
        );

        let methods = trait_def.methods.expect("expected methods");
        let method = &methods[0];

        let receiver = method.receiver.as_ref().expect("expected receiver");
        assert_eq!(receiver.text, "self");
        assert_eq!(method.name.as_ref().map(|n| n.text.as_str()), Some("bar"));
    }

    #[test]
    fn parse_trait_method_with_parameters() {
        let (trait_def, diags) = parse_trait_definition("trait Foo { bar(value: Int) }");

        assert!(diags.is_empty());

        let methods = trait_def.methods.expect("expected methods");
        let method = &methods[0];

        assert!(method.params.is_some());
    }

    #[test]
    fn parse_trait_method_with_return_type() {
        let (trait_def, diags) = parse_trait_definition("trait Foo { bar(): Int }");

        assert!(diags.is_empty());

        let methods = trait_def.methods.expect("expected methods");
        let method = &methods[0];

        assert!(method.return_annotation.is_some());
    }

    #[test]
    fn parse_generic_trait_method() {
        let (trait_def, diags) = parse_trait_definition("trait Foo { bar<T>(value: T): T }");

        assert!(diags.is_empty());

        let methods = trait_def.methods.expect("expected methods");
        let method = &methods[0];

        let type_params = method
            .type_params
            .as_ref()
            .expect("expected method type parameters");

        assert_eq!(type_params.len(), 1);
        assert_eq!(type_params[0].text, "T");
        assert!(method.params.is_some());
        assert!(method.return_annotation.is_some());
    }

    #[test]
    fn parse_trait_method_missing_parameters() {
        let (trait_def, diags) = parse_trait_definition("trait Foo { bar }");

        assert_eq!(diags.len(), 1);
        assert!(matches!(diags[0].kind, DiagnosticKind::MissingParams));

        let methods = trait_def.methods.expect("expected methods");
        assert_eq!(methods.len(), 1);

        let method = &methods[0];
        assert_eq!(method.name.as_ref().map(|n| n.text.as_str()), Some("bar"));
        assert!(method.params.is_none());
    }
}
