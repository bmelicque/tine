use pest::iterators::Pair;

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
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let pattern = Box::new(self.parse_pattern(inner.next().unwrap()));
        let op_span = inner.next().unwrap().as_span();
        let op = op_span.as_str().to_string().into();

        let value = Box::new(self.parse_expression(inner.next().unwrap()));
        if value.is_empty() {
            self.error("expected expression".into(), increment_span(op_span));
        }

        ast::VariableDeclaration {
            span,
            pattern,
            op,
            value,
        }
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
