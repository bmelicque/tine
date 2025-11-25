use pest::{iterators::Pair, Span};

use crate::{
    ast,
    parser::{parser::Rule, utils::increment_span, ParserEngine},
};

impl ParserEngine {
    /// Parse a variable declaration.
    ///
    /// Expected pairs:
    /// `pattern ~ declaration_operator ~ expression`
    /// with `declaration_operator` being either `:=` (mutable) or `::` (constant)
    pub fn parse_variable_declaration(
        &mut self,
        pair: Pair<'static, Rule>,
    ) -> ast::VariableDeclaration {
        let whole_span = pair.as_span();
        let mut inner = pair.into_inner();

        let (docs, next) = match inner.next().unwrap() {
            next if next.as_rule() == Rule::doc_comment => {
                (self.parse_docs(next), inner.next().unwrap())
            }
            next => (None, next),
        };
        let span = Span::new(
            whole_span.get_input(),
            next.as_span().start(),
            whole_span.end(),
        )
        .unwrap();
        let pattern = Box::new(self.parse_pattern(next));
        let op_span = inner.next().unwrap().as_span();
        let op = op_span.as_str().to_string().into();

        let value = Box::new(self.parse_expression(inner.next().unwrap()));
        if value.is_empty() {
            self.error("expected expression".into(), increment_span(op_span));
        }

        ast::VariableDeclaration {
            docs,
            span,
            pattern,
            op,
            value,
        }
    }

    fn parse_docs(&mut self, pair: Pair<'static, Rule>) -> Option<Span<'static>> {
        debug_assert_eq!(pair.as_rule(), Rule::doc_comment);
        let docs_span = pair.as_span();
        let line_count = docs_span.lines_span().count();
        let last = docs_span.lines_span().take(line_count - 1).last().unwrap();
        Span::new(docs_span.get_input(), docs_span.start(), last.end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parser::parser::{MyLanguageParser, Rule},
        ParseError,
    };
    use pest::Parser;

    fn parse_statement_input(input: &'static str, rule: Rule) -> (ast::Statement, Vec<ParseError>) {
        let pair = MyLanguageParser::parse(rule, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        let stmt = parser_engine.parse_statement(pair);
        (stmt, parser_engine.errors)
    }

    #[test]
    fn test_parse_variable_declaration() {
        let input = "x := 42";
        let (stmt, errors) = parse_statement_input(input, Rule::variable_declaration);

        assert_eq!(errors.len(), 0);
        let ast::Statement::VariableDeclaration(var_decl) = stmt else {
            panic!("Expected VariableDeclaration");
        };
        match *var_decl.pattern {
            ast::Pattern::Identifier(ast::IdentifierPattern { span }) if span.as_str() == "x" => {}
            _ => panic!("Identifier pattern expected"),
        };
        assert_eq!(var_decl.op, ast::DeclarationOp::Mut);
        match *var_decl.value {
            ast::Expression::NumberLiteral(literal) => assert_eq!(literal.value, 42.0),
            _ => panic!("Expected NumberLiteral as variable value"),
        }
    }

    #[test]
    fn test_parse_variable_declaration_with_single_doc() {
        let input = r#"// a value
        x := 42"#;
        let (stmt, errors) = parse_statement_input(input, Rule::variable_declaration);

        assert_eq!(errors.len(), 0);
        let ast::Statement::VariableDeclaration(var_decl) = stmt else {
            panic!("Expected VariableDeclaration");
        };
        let expected = "// a value\n";
        match var_decl.docs {
            Some(span) if span.as_str() == expected => {}
            _ => panic!("expected comment '{}', got {:?}", expected, var_decl.docs),
        }
    }

    #[test]
    fn test_parse_variable_declaration_with_docs() {
        let input = r#"// docs
        // over several lines
        x := 42"#;
        let (stmt, errors) = parse_statement_input(input, Rule::variable_declaration);

        assert_eq!(errors.len(), 0);
        assert!(
            matches!(stmt, ast::Statement::VariableDeclaration(_)),
            "expected variable declaration"
        );
    }

    #[test]
    fn test_parse_variable_declaration_missing_value() {
        let input = "x := ";
        let (stmt, errors) = parse_statement_input(input, Rule::variable_declaration);

        assert_eq!(errors.len(), 1);
        let ast::Statement::VariableDeclaration(var_decl) = stmt else {
            panic!("Expected VariableDeclaration");
        };
        match *var_decl.pattern {
            ast::Pattern::Identifier(ast::IdentifierPattern { span }) if span.as_str() == "x" => {}
            _ => panic!("Identifier pattern expected"),
        };
        assert_eq!(var_decl.op, ast::DeclarationOp::Mut);
        assert_eq!(*var_decl.value, ast::Expression::Empty);
    }
}
