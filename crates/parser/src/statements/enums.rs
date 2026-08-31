use std::ops::Range;

use tine_ast::*;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{statements::utils::TypeName, tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_enum(
        &mut self,
        docs: Option<Docs>,
        meta: Option<Vec<MetaAttribute>>,
        pub_loc: Option<Location>,
    ) -> EnumDefinition {
        let kw_range = self.eat(&[Token::Enum]);
        let start = pub_loc.unwrap_or(self.localize(kw_range));

        let (name, params) = self.try_parse_type_name();
        let Some((items, end)) = self
            .try_parse(|s| s.parse_enum_items(), |t| matches!(t, Token::Newline))
            .ok()
            .flatten()
        else {
            let error_loc = self.next_loc().nth_char(0);
            self.error(DiagnosticKind::MissingBody, error_loc);
            return EnumDefinition {
                loc: enum_loc(start, &name, &params, None),
                docs,
                meta,
                public: pub_loc.is_some(),
                name,
                params,
                items: None,
            };
        };

        EnumDefinition {
            docs,
            meta,
            loc: enum_loc(start, &name, &params, Some(end)),
            public: pub_loc.is_some(),
            name,
            params,
            items: Some(items),
        }
    }

    fn parse_enum_items(&mut self) -> Option<(Vec<EnumItem>, Location)> {
        self.eat_if(&[Token::LBrace])?;
        let items = self.parse_list(|p| p.parse_enum_item(), Token::Comma, Token::RBrace);
        let end = self.expect(Token::RBrace);
        let end = self.localize(end);
        Some((items, end))
    }

    fn parse_enum_item(&mut self) -> Option<EnumItem> {
        let docs = self.maybe_parse_docs();
        let public_range = self.eat_if(&[Token::Pub]);

        match self.tokens.peek()?.0.as_ref().ok()? {
            Token::Static | Token::Mut | Token::Fn => self
                .parse_method_definition(docs, public_range)
                .map(Into::into),
            _ => self
                .parse_variant_definition(docs, public_range)
                .map(Into::into),
        }
    }

    fn parse_variant_definition(
        &mut self,
        docs: Option<Docs>,
        public_range: Option<Range<usize>>,
    ) -> Option<VariantDefinition> {
        let Ok(type_name) = self.parse_type_name(&[Token::LBrace, Token::LParen, Token::Newline])
        else {
            return None;
        };
        if let Some(TypeName {
            params: Some(_),
            loc,
            ..
        }) = type_name
        {
            self.error(DiagnosticKind::UnexpectedTypeParams, loc);
        }
        let body = self.parse_variant_body();
        let public_loc = public_range.map(|r| self.localize(r));
        let loc = Location::merge_list(&[
            public_loc.as_ref().map(|x| x as &dyn Locatable),
            type_name.as_ref().map(|x| x as &dyn Locatable),
            body.as_ref().map(|x| x as &dyn Locatable),
            body.as_ref().map(|x| x as &dyn Locatable),
        ])?;
        Some(VariantDefinition {
            loc,
            docs,
            public: public_loc.is_some(),
            body,
            name: type_name.map(|t| t.name),
        })
    }

    fn parse_variant_body(&mut self) -> Option<VariantBody> {
        let (_, start_loc) = self.maybe_eat(|t| t.lparen())?;

        let elements = self.parse_list(
            |parser| parser.parse_variant_body_element(),
            Token::Comma,
            Token::RParen,
        );

        let end_range = self.expect(Token::RParen);
        let end_loc = self.localize(end_range);

        Some(VariantBody {
            loc: Location::merge(start_loc, end_loc),
            elements,
        })
    }

    fn parse_variant_body_element(&mut self) -> Option<(bool, Type)> {
        let is_public = self.eat_if(&[Token::Pub]).is_some();
        let ty = self.parse_type()?;
        Some((is_public, ty))
    }
}

fn enum_loc(
    start: Location,
    name: &Option<Identifier>,
    params: &Option<Vec<Identifier>>,
    end: Option<Location>,
) -> Location {
    if let Some(end) = end {
        Location::merge(start, end)
    } else if let Some(param) = params.as_ref().and_then(|p| p.last()) {
        Location::merge(start, param.loc())
    } else if let Some(name) = &name {
        Location::merge(start, name.loc)
    } else {
        start
    }
}

#[cfg(test)]
mod tests {
    use tine_common::diagnostics::DiagnosticKind;

    use crate::test_utils::parse_statement;

    #[test]
    fn parse_enum_definition() {
        let stmt = parse_statement("enum Enum {}").expect("expected no errors");
        stmt.as_enum().expect("expected enum definition");
    }

    #[test]
    fn parse_enum_definition_only_kw() {
        let (_, errors) = parse_statement("enum").expect_err("expected errors");
        assert_eq!(errors.len(), 2);
        assert!(matches!(errors[0].kind, DiagnosticKind::MissingName));
        assert!(matches!(errors[1].kind, DiagnosticKind::MissingBody));
    }
}
