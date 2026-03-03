use crate::{
    ast,
    parser::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub(super) fn parse_constructor_literal(
        &mut self,
        qualifiers: Vec<ast::Identifier>,
    ) -> ast::ConstructorLiteral {
        let constructor = self.parse_constructor();
        let body = self.parse_constructor_body();
        let loc = match (qualifiers.first(), &body) {
            (Some(qualifier), Some(body)) => Location::merge(qualifier.loc, body.loc()),
            (Some(qualifier), None) => Location::merge(qualifier.loc, constructor.loc()),
            (None, Some(body)) => Location::merge(constructor.loc(), body.loc()),
            (None, None) => constructor.loc(),
        };
        ast::ConstructorLiteral {
            loc,
            qualifiers,
            constructor,
            body,
        }
    }

    fn parse_constructor(&mut self) -> ast::Constructor {
        // This function should be called only if the next token is a type identifier.
        // In that case, `parse_type` should always return `Some` thing, so unwrapping is safe.
        let constructor = self.parse_type().unwrap();

        match constructor {
            ast::Type::Named(named_type) => {
                if let Some((Ok(Token::Dot), _)) = self.tokens.peek() {
                    self.parse_variant_constructor(named_type).into()
                } else {
                    named_type.into()
                }
            }
            ast::Type::Map(map_type) => map_type.into(),
            _ => {
                self.error(DiagnosticKind::InvalidTypeConstructor, constructor.loc());
                constructor.into()
            }
        }
    }

    fn parse_variant_constructor(&mut self, named_type: ast::NamedType) -> ast::VariantConstructor {
        let range = self.eat(&[Token::Dot]);
        let variant_name = match self.tokens.peek() {
            Some((Ok(Token::Ident(_)), _)) => Some(self.parse_identifier()),
            _ => None,
        };
        let loc = match &variant_name {
            Some(variant_name) => Location::merge(named_type.loc, variant_name.loc),
            None => Location::merge(named_type.loc, self.localize(range)),
        };
        ast::VariantConstructor {
            loc,
            enum_name: Box::new(named_type),
            variant_name,
        }
    }

    fn parse_constructor_body(&mut self) -> Option<ast::ConstructorBody> {
        match self.tokens.peek() {
            Some((Ok(Token::LBrace), _)) => Some(self.parse_struct_literal_body().into()),
            Some((Ok(Token::LParen), _)) => Some(self.parse_tuple().into()),
            _ => {
                let error = DiagnosticKind::ExpectedToken {
                    expected: vec!["(".to_string(), "{".to_string()],
                };
                let error_loc = self.next_loc();
                self.error(error, error_loc);
                None
            }
        }
    }

    fn parse_struct_literal_body(&mut self) -> ast::StructLiteralBody {
        let start_range = self.eat(&[Token::LBrace]);
        let fields = self.parse_list(|p| p.parse_constructor_field(), Token::Comma, Token::RBrace);
        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RBrace), r)) => r.clone(),
            _ => self.recover_at(&[Token::RBrace]),
        };
        let loc = self.localize(start_range.start..end_range.end);
        ast::StructLiteralBody { loc, fields }
    }

    fn parse_constructor_field(&mut self) -> Option<ast::ConstructorField> {
        let key = match self.tokens.peek() {
            Some((Ok(Token::Ident(_)), _)) => Some(self.parse_identifier().into()),
            Some((Ok(Token::String(_) | Token::Int(_) | Token::Float(_) | Token::Bool(_)), _)) => {
                self.parse_atom().map(|a| a.into())
            }
            Some((Ok(Token::LBracket), _)) => {
                self.eat(&[Token::LBracket]);
                let expr = self.parse_expression();
                if expr.is_none() {
                    let loc = self.next_loc();
                    self.error(DiagnosticKind::MissingExpression, loc);
                }
                self.expect(Token::RBracket);
                expr
            }
            _ => None,
        };
        let key: Option<ast::ConstructorKey> = key.map(|k| k.into());

        let Some((Ok(Token::Colon), _)) = self.tokens.peek() else {
            let error = DiagnosticKind::ExpectedToken {
                expected: vec![":".to_string()],
            };
            let error_loc = self.next_loc();
            self.error(error, error_loc);
            return match &key {
                Some(some) => Some(ast::ConstructorField {
                    loc: some.loc(),
                    key,
                    value: None,
                }),
                None => None,
            };
        };
        let colon_range = self.eat(&[Token::Colon]);

        let value = self.parse_expression();
        if value.is_none() {
            let error = DiagnosticKind::MissingExpression;
            let error_loc = self.next_loc();
            self.error(error, error_loc);
        }

        let loc = match (&key, &value) {
            (Some(k), Some(v)) => Location::merge(k.loc(), v.loc()),
            (Some(k), None) => Location::merge(k.loc(), self.localize(colon_range)),
            (None, Some(v)) => Location::merge(self.localize(colon_range), v.loc()),
            (None, None) => self.localize(colon_range),
        };

        Some(ast::ConstructorField { loc, key, value })
    }
}
