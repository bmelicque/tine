use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
    Location,
};

impl ParserEngine {
    pub fn parse_exponentiation(&mut self, pair: Pair<'_, Rule>) -> ast::Expression {
        assert!(pair.as_rule() == Rule::exponentiation);
        let mut loc = self.localize(pair.as_span());
        let mut node = ast::Expression::Empty;
        for sub_pair in pair.into_inner().rev() {
            let left = self.parse_expression(sub_pair);
            if node == ast::Expression::Empty {
                node = left;
                continue;
            }
            loc = Location::merge(left.loc(), loc);
            node = ast::BinaryExpression {
                left: Box::new(left),
                operator: ast::BinaryOperator::Pow,
                right: Box::new(node),
                loc,
            }
            .into();
        }
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{Rule, TineParser};
    use pest::Parser;

    fn parse_expression_input(input: &'static str) -> ast::Expression {
        let pair = TineParser::parse(Rule::exponentiation, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new(0);
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
            ast::Expression::IntLiteral(left) => assert_eq!(left.value, 2),
            _ => panic!("Expected IntLiteral on the left"),
        }

        let ast::Expression::Binary(inner_binary) = *binary.right else {
            panic!("Expected BinaryExpression on the right")
        };
        assert_eq!(inner_binary.operator, ast::BinaryOperator::Pow);
        match *inner_binary.left {
            ast::Expression::IntLiteral(left) => assert_eq!(left.value, 3),
            _ => panic!("Expected IntLiteral on the left"),
        }
        match *inner_binary.right {
            ast::Expression::IntLiteral(right) => assert_eq!(right.value, 2),
            _ => panic!("Expected IntLiteral on the right"),
        }
    }
}
