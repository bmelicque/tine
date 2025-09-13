use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, utils::merge_span, ParserEngine},
};

impl ParserEngine {
    pub fn parse_exponentiation(&mut self, pair: Pair<'static, Rule>) -> ast::Expression {
        assert!(pair.as_rule() == Rule::exponentiation);
        let mut span = pair.as_span();
        let mut node = ast::Expression::Empty;
        for sub_pair in pair.into_inner().rev() {
            let left = self.parse_expression(sub_pair);
            if node == ast::Expression::Empty {
                node = left;
                continue;
            }
            span = merge_span(left.as_span(), span);
            node = ast::BinaryExpression {
                left: Box::new(left),
                operator: ast::BinaryOperator::Pow,
                right: Box::new(node),
                span,
            }
            .into();
        }
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse_expression_input(input: &'static str) -> ast::Expression {
        let pair = MyLanguageParser::parse(Rule::exponentiation, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        parser_engine.parse_expression(pair)
    }

    #[test]
    fn test_parse_exponentiation_expression() {
        let input = "2 ** 3 ** 2";
        let result = parse_expression_input(input);

        let ast::Expression::Binary(binary) = result else {
            panic!("Expected BinaryExpression")
        };
        assert_eq!(binary.operator, ast::BinaryOperator::Pow);
        match *binary.left {
            ast::Expression::NumberLiteral(left) => assert_eq!(left.value, 2.0),
            _ => panic!("Expected NumberLiteral on the left"),
        }

        let ast::Expression::Binary(inner_binary) = *binary.right else {
            panic!("Expected BinaryExpression on the right")
        };
        assert_eq!(inner_binary.operator, ast::BinaryOperator::Pow);
        match *inner_binary.left {
            ast::Expression::NumberLiteral(left) => assert_eq!(left.value, 3.0),
            _ => panic!("Expected NumberLiteral on the left"),
        }
        match *inner_binary.right {
            ast::Expression::NumberLiteral(right) => assert_eq!(right.value, 2.0),
            _ => panic!("Expected NumberLiteral on the right"),
        }
    }
}
