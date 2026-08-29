use tine_ast::*;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_variable_declaration(
        &mut self,
        docs: Option<Docs>,
        pub_loc: Option<Location>,
    ) -> VariableDeclaration {
        let start_range = self.eat(&[Token::Let]);
        let kw_loc = self.localize(start_range);

        let pattern = self.with_mutable_binding(|self_| self_.parse_pattern());
        if pattern.is_none() {
            let loc = kw_loc.increment();
            self.error(DiagnosticKind::MissingPattern, loc);
        }

        let type_annotation = self.maybe_parse_annotation();

        let op_range = self.expect(Token::Eq);
        let value = self.parse_expression_with_block();
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

        VariableDeclaration {
            docs,
            loc,
            public: pub_loc.is_some(),
            mutable: false,
            pattern,
            annotation: type_annotation,
            value: value.into(),
        }
    }

    fn maybe_parse_annotation(&mut self) -> Option<Type> {
        let r = self.eat_if(&[Token::Colon])?;
        let ty = self.parse_type();
        if ty.is_none() {
            let loc = self.localize(r);
            self.error(DiagnosticKind::MissingType, loc);
        }
        ty
    }
}

#[cfg(test)]
mod tests {
    use tine_ast as ast;
    use tine_common::{
        diagnostics::{Diagnostic, DiagnosticKind},
        locations::{Location, Span},
    };

    use crate::Parser;

    /// Parses `src` as a single variable declaration and returns the resulting
    /// node. Panics with the source and collected diagnostics if parsing reported
    /// errors.
    fn parse_decl(src: &str) -> ast::VariableDeclaration {
        let mut parser = Parser::new(1, src);
        let decl = parser.parse_variable_declaration(None, None);
        let diagnostics = parser.diagnostics;
        assert!(
            diagnostics.is_empty(),
            "expected no diagnostics parsing `{src}`, got: {diagnostics:?}"
        );

        decl
    }

    /// Same as `parse_decl`, but for cases that are expected to produce a
    /// diagnostic
    fn parse_decl_expect_errors(src: &str) -> (ast::VariableDeclaration, Vec<Diagnostic>) {
        let mut parser = Parser::new(1, src);
        let decl = parser.parse_variable_declaration(None, None);
        let diagnostics = parser.diagnostics;
        (decl, diagnostics)
    }

    /// Returns the bound identifier's name, assuming a simple pattern.
    fn binding_name(pattern: &ast::Pattern) -> &str {
        match pattern {
            ast::Pattern::Identifier(id) => id.identifier.as_str(),
            ast::Pattern::Path(p) if p.segments.len() == 1 => p.segments[0].ident.as_str(),
            other => panic!("expected a simple identifier pattern, got: {other:?}"),
        }
    }

    /// Returns whether the pattern was marked `mut`.
    fn pattern_is_mutable(pattern: &ast::Pattern) -> bool {
        match pattern {
            ast::Pattern::Identifier(i) => i.mutable,
            ast::Pattern::Path(p) if p.segments.len() == 1 => false,
            other => panic!("expected a simple identifier pattern, got: {other:?}"),
        }
    }

    #[test]
    fn parses_immutable_binding() {
        let decl = parse_decl("let name = \"Ada\"");

        let pattern = decl.pattern.expect("pattern should be present");
        assert_eq!(binding_name(&pattern), "name");
        assert!(
            !pattern_is_mutable(&pattern),
            "`let` without `mut` should be immutable"
        );

        decl.value.expect("value should be present");

        assert!(
            !decl.public,
            "declaration without `pub` should not be public"
        );
    }

    #[test]
    fn parses_mutable_binding() {
        let decl = parse_decl("let mut count = 0");

        let pattern = decl.pattern.expect("pattern should be present");
        assert_eq!(binding_name(&pattern), "count");
        assert!(
            pattern_is_mutable(&pattern),
            "`let mut` should mark the pattern mutable, found {:?}",
            pattern
        );

        decl.value.expect("value should be present");
    }

    #[test]
    fn parses_public_declaration_when_pub_loc_given() {
        let mut parser = Parser::new(1, "let name = \"Ada\"");
        let pub_loc = Some(Location::new(1, Span::new(0, 3)));

        let decl = parser.parse_variable_declaration(None, pub_loc);

        assert!(
            decl.public,
            "declaration should be public when a pub_loc is supplied"
        );
    }

    #[test]
    fn parses_typed_binding() {
        let decl = parse_decl("let count: int = 0");
        decl.annotation.expect("type should be present");
        decl.value.expect("value should be present");
    }

    #[test]
    fn attaches_provided_docs_unchanged() {
        let mut parser = Parser::new(1, "let name = \"Ada\"");
        let docs = Some(ast::Docs {
            loc: Location::dummy(),
            text: "Explains what `name` is for.".into(),
        });

        let decl = parser.parse_variable_declaration(docs.clone(), None);

        assert_eq!(
            decl.docs, docs,
            "docs should be attached to the declaration unchanged"
        );
    }

    #[test]
    fn missing_pattern_reports_missing_pattern_diagnostic() {
        let (decl, diagnostics) = parse_decl_expect_errors("let = 5");

        assert!(
            decl.pattern.is_none(),
            "pattern should be absent when omitted from source"
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind == DiagnosticKind::MissingPattern),
            "expected a MissingPattern diagnostic, got: {diagnostics:?}"
        );
    }

    #[test]
    fn missing_expression_reports_missing_expression_diagnostic() {
        let (decl, diagnostics) = parse_decl_expect_errors("let name =");

        assert!(
            decl.value.is_none(),
            "value should be absent when omitted from source"
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind == DiagnosticKind::MissingExpression),
            "expected a MissingExpression diagnostic, got: {diagnostics:?}"
        );
    }

    #[test]
    fn missing_pattern_and_expression_reports_both_diagnostics() {
        let (_decl, diagnostics) = parse_decl_expect_errors("let =");

        assert!(diagnostics
            .iter()
            .any(|d| d.kind == DiagnosticKind::MissingPattern));
        assert!(diagnostics
            .iter()
            .any(|d| d.kind == DiagnosticKind::MissingExpression));
    }
}
