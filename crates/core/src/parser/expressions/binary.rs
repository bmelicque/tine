use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_binary_ltr_expression(&mut self, pair: Pair<'_, Rule>) -> ast::Expression {
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let Some(next) = inner.next() else {
            return ast::Expression::Empty;
        };
        let mut left = self.parse_expression(next);

        let mut is_binary = false;
        while let Some(op_pair) = inner.next() {
            if !is_binary && left.is_empty() {
                let loc = self.localize(op_pair.as_span());
                self.error("Expression expected".to_string(), loc);
            }
            is_binary = true;
            let operator = op_pair.as_str().to_string();

            let Some(right_pair) = inner.next() else {
                let loc = self.localize(op_pair.as_span());
                self.error("Expression expected".to_string(), loc);
                continue;
            };

            let right = self.parse_expression(right_pair);
            if right.is_empty() {
                let loc = self.localize(op_pair.as_span());
                self.error("Expression expected".to_string(), loc);
            }

            left = ast::BinaryExpression {
                loc,
                left: Box::new(left),
                operator: operator.into(),
                right: Box::new(right),
            }
            .into();
        }

        left
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse_expression_input(input: &'static str) -> ast::Expression {
        let pair = MyLanguageParser::parse(Rule::expression, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new(0);
        parser_engine.parse_expression(pair)
    }

    #[test]
    fn test_parse_binary_expression() {
        let input = "1 + 2 * 3";
        let result = parse_expression_input(input);

        let ast::Expression::Binary(binary) = result else {
            panic!("Expected BinaryExpression");
        };

        assert_eq!(binary.operator, ast::BinaryOperator::Add);
        match *binary.left {
            ast::Expression::NumberLiteral(left) => assert_eq!(left.value, 1.0),
            _ => panic!("Expected NumberLiteral on the left"),
        }

        let ast::Expression::Binary(inner_binary) = *binary.right else {
            panic!("Expected BinaryExpression on the right");
        };
        assert_eq!(inner_binary.operator, ast::BinaryOperator::Mul);
        match *inner_binary.left {
            ast::Expression::NumberLiteral(left) => assert_eq!(left.value, 2.0),
            _ => panic!("Expected NumberLiteral on the left"),
        }
        match *inner_binary.right {
            ast::Expression::NumberLiteral(right) => assert_eq!(right.value, 3.0),
            _ => panic!("Expected NumberLiteral on the right"),
        }
    }

    #[test]
    fn test_parse_equality() {
        let input = "1 == 1";
        let result = parse_expression_input(input);

        let ast::Expression::Binary(binary) = result else {
            panic!("Expected BinaryExpression, got {:?}", result);
        };

        assert_eq!(binary.operator, ast::BinaryOperator::EqEq);
        match *binary.left {
            ast::Expression::NumberLiteral(left) => assert_eq!(left.value, 1.0),
            _ => panic!("Expected NumberLiteral on the left"),
        }
        match *binary.right {
            ast::Expression::NumberLiteral(right) => assert_eq!(right.value, 1.0),
            _ => panic!("Expected NumberLiteral on the right"),
        }
    }
}
