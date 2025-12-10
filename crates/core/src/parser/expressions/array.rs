use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_array_expression(&mut self, pair: Pair<'_, Rule>) -> ast::ArrayExpression {
        assert_eq!(pair.as_rule(), Rule::array_expression);
        let span = pair.as_span().into();
        let elements = pair
            .into_inner()
            .map(|element| self.parse_expression(element))
            .filter(|expr| !matches!(expr, ast::Expression::Empty))
            .collect();
        ast::ArrayExpression { span, elements }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse_expression_input(input: &'static str) -> ast::Expression {
        let pair = MyLanguageParser::parse(Rule::array_expression, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        parser_engine.parse_expression(pair)
    }

    fn validate_array(expr: &ast::Expression) {
        let ast::Expression::Array(array) = expr else {
            panic!("Expected ArrayExpression");
        };

        assert_eq!(array.elements.len(), 3);

        assert!(matches!(
            array.elements[0],
            ast::Expression::NumberLiteral(ast::NumberLiteral { value, .. }) if value == 1.0
        ));

        assert!(matches!(
            array.elements[1],
            ast::Expression::NumberLiteral(ast::NumberLiteral { value, .. }) if value == 2.0
        ));

        assert!(matches!(
            array.elements[2],
            ast::Expression::NumberLiteral(ast::NumberLiteral { value, .. }) if value == 3.0
        ));
    }

    #[test]
    fn test_parse_empty_array() {
        let input = "[]";
        let result = parse_expression_input(input);
        let ast::Expression::Array(array) = result else {
            panic!("Expected ArrayExpression");
        };
        assert_eq!(
            array.elements.len(),
            0,
            "expected no elements, got {:?}",
            array.elements
        );
    }

    #[test]
    fn test_parse_array_expression() {
        let input = "[1, 2, 3]";
        let result = parse_expression_input(input);
        validate_array(&result);
    }

    #[test]
    fn test_parse_multiline_array() {
        let input = r#"[
            1,
            2,
            3,
        ]"#;
        let result = parse_expression_input(input);
        validate_array(&result);
    }

    #[test]
    fn test_parse_multiline_array_missing_last_comma() {
        let input = r#"[
            1,
            2,
            3
        ]"#;
        let result = parse_expression_input(input);
        validate_array(&result);
    }
}
