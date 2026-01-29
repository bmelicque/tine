use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_tuple_expression(&mut self, pair: Pair<'_, Rule>) -> ast::TupleExpression {
        assert_eq!(pair.as_rule(), Rule::tuple_expression);
        let loc = self.localize(pair.as_span());
        let elements = pair
            .into_inner()
            .map(|pair| self.parse_expression(pair))
            .collect();
        ast::TupleExpression { loc, elements }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{Rule, TineParser};
    use pest::Parser;

    fn parse_expression_input(input: &'static str) -> ast::Expression {
        let pair = TineParser::parse(Rule::tuple_or_expression, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new(0);
        parser_engine.parse_expression(pair)
    }

    #[test]
    fn test_parse_tuple_expression_with_multiple_elements() {
        let input = "(1, \"hello\", true)";
        let result = parse_expression_input(input);

        let ast::Expression::Tuple(result) = result else {
            panic!("Tuple expected")
        };
        assert_eq!(result.elements.len(), 3);

        assert!(matches!(
            result.elements[0],
            ast::Expression::IntLiteral(ast::IntLiteral { value, .. }) if value == 1
        ));

        assert!(matches!(
            result.elements[1],
            ast::Expression::StringLiteral(ast::StringLiteral { ref text, .. }) if text.as_str() == "\"hello\""
        ));

        assert!(matches!(
            result.elements[2],
            ast::Expression::BooleanLiteral(ast::BooleanLiteral { value, .. }) if value == true
        ));
    }

    #[test]
    fn test_parse_nested_tuple_expression() {
        let input = "(1, (\"nested\", false))";
        let result = parse_expression_input(input);
        let ast::Expression::Tuple(result) = result else {
            panic!("Expected tuple!")
        };

        assert_eq!(result.elements.len(), 2);

        assert!(matches!(
            result.elements[0],
            ast::Expression::IntLiteral(ast::IntLiteral { value, .. }) if value == 1
        ));

        let ast::Expression::Tuple(nested_tuple) = &result.elements[1] else {
            panic!("Expected a nested tuple");
        };
        assert_eq!(nested_tuple.elements.len(), 2);

        assert!(matches!(
            nested_tuple.elements[0],
            ast::Expression::StringLiteral(ast::StringLiteral { ref text, .. }) if text.as_str() == "\"nested\""
        ));

        assert!(matches!(
            nested_tuple.elements[1],
            ast::Expression::BooleanLiteral(ast::BooleanLiteral { value, .. }) if value == false
        ));
    }
}
