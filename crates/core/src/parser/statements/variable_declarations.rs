use pest::iterators::Pair;

use crate::{
    ast,
    locations::Span,
    parser::{parser::Rule, utils::normalize_doc_comment, ParserEngine},
    Location,
};

impl ParserEngine {
    /// Parse a variable declaration.
    ///
    /// Expected pairs:
    /// `doc_comment? ~ declaration_operator ~ pattern ~ "=" ~ expression`
    /// with `declaration_operator` being either `:=` (mutable) or `::` (constant)
    pub fn parse_variable_declaration(&mut self, pair: Pair<'_, Rule>) -> ast::VariableDeclaration {
        let whole_span = Span::from(pair.as_span());
        let mut inner = pair.into_inner();

        let (docs, next) = match inner.next().unwrap() {
            next if next.as_rule() == Rule::doc_comment => {
                (Some(self.parse_docs(next)), inner.next().unwrap())
            }
            next => (None, next),
        };
        let span = Span::new(next.as_span().start() as u32, whole_span.end());
        let loc = Location::new(self.module, span);

        let keyword = next.as_str().into();
        let pattern = Box::new(self.parse_pattern(inner.next().unwrap()));
        let value = Box::new(self.parse_expression(inner.next().unwrap()));
        if value.is_empty() {
            self.error("expected expression".into(), loc.increment());
        }

        ast::VariableDeclaration {
            docs,
            loc,
            keyword,
            pattern,
            value,
        }
    }

    fn parse_docs(&mut self, pair: Pair<'_, Rule>) -> ast::Docs {
        debug_assert_eq!(pair.as_rule(), Rule::doc_comment);
        let docs_span = pair.as_span();
        let line_count = docs_span.lines_span().count();
        let last = docs_span.lines_span().take(line_count - 1).last().unwrap();
        let span = pest::Span::new(docs_span.get_input(), docs_span.start(), last.end()).unwrap();
        let loc = self.localize(span);
        let text = normalize_doc_comment(span.as_str());
        ast::Docs { loc, text }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parser::parser::{Rule, TineParser},
        ParseError,
    };
    use pest::Parser;

    fn parse_statement_input(input: &'static str, rule: Rule) -> (ast::Statement, Vec<ParseError>) {
        let pair = TineParser::parse(rule, input).unwrap().next().unwrap();
        let mut parser_engine = ParserEngine::new(0);
        let stmt = parser_engine.parse_statement(pair);
        (stmt, parser_engine.errors)
    }

    #[test]
    fn test_parse_variable_declaration() {
        let input = "var x = 42";
        let (stmt, errors) = parse_statement_input(input, Rule::variable_declaration);

        assert_eq!(errors.len(), 0);
        let ast::Statement::VariableDeclaration(var_decl) = stmt else {
            panic!("Expected VariableDeclaration");
        };
        match *var_decl.pattern {
            ast::Pattern::Identifier(ident) if ident.as_str() == "x" => {}
            _ => panic!("Identifier pattern expected"),
        };
        assert_eq!(var_decl.keyword, ast::DeclarationKeyword::Var);
        match *var_decl.value {
            ast::Expression::NumberLiteral(literal) => assert_eq!(literal.value, 42.0),
            _ => panic!("Expected NumberLiteral as variable value"),
        }
    }

    #[test]
    fn test_parse_variable_declaration_with_single_doc() {
        let input = r#"// a value
        var x = 42"#;
        let (stmt, errors) = parse_statement_input(input, Rule::variable_declaration);

        assert_eq!(errors.len(), 0);
        let ast::Statement::VariableDeclaration(var_decl) = stmt else {
            panic!("Expected VariableDeclaration");
        };
        let expected = "a value";
        match var_decl.docs {
            Some(docs) if docs.text.as_str() == expected => {}
            _ => panic!("expected comment '{}', got {:?}", expected, var_decl.docs),
        }
    }

    #[test]
    fn test_parse_variable_declaration_with_docs() {
        let input = r#"// docs
        // over several lines
        var x = 42"#;
        let (stmt, errors) = parse_statement_input(input, Rule::variable_declaration);

        assert_eq!(errors.len(), 0);
        assert!(
            matches!(stmt, ast::Statement::VariableDeclaration(_)),
            "expected variable declaration"
        );
    }

    #[test]
    fn test_parse_variable_declaration_missing_value() {
        let input = "var x = ";
        let (stmt, errors) = parse_statement_input(input, Rule::variable_declaration);

        assert_eq!(errors.len(), 1);
        let ast::Statement::VariableDeclaration(var_decl) = stmt else {
            panic!("Expected VariableDeclaration");
        };
        match *var_decl.pattern {
            ast::Pattern::Identifier(ident) if ident.as_str() == "x" => {}
            _ => panic!("Identifier pattern expected"),
        };
        assert_eq!(var_decl.keyword, ast::DeclarationKeyword::Var);
        assert_eq!(*var_decl.value, ast::Expression::Empty);
    }
}
