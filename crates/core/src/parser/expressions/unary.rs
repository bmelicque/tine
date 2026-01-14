use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_unary_expression(&mut self, pair: Pair<'_, Rule>) -> ast::UnaryExpression {
        assert!(pair.as_rule() == Rule::unary);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let operator = self.parse_unary_operator(inner.next().unwrap());
        let operand = Box::new(self.parse_expression(inner.next().unwrap()));
        ast::UnaryExpression {
            loc,
            operator,
            operand,
        }
    }

    fn parse_unary_operator(&mut self, pair: Pair<'_, Rule>) -> ast::UnaryOperator {
        assert!(pair.as_rule() == Rule::unary_op);
        match pair.as_str() {
            "&" => ast::UnaryOperator::Ampersand,
            "@" => ast::UnaryOperator::At,
            "!" => ast::UnaryOperator::Bang,
            "$" => ast::UnaryOperator::Dollar,
            "-" => ast::UnaryOperator::Minus,
            "*" => ast::UnaryOperator::Star,
            op => {
                self.error(
                    format!("Unknown unary operator: {}", op),
                    self.localize(pair.as_span()),
                );
                ast::UnaryOperator::Star
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{TineParser, Rule};
    use pest::Parser;

    fn parse_expression_input(input: &'static str) -> ast::Expression {
        let pair = TineParser::parse(Rule::expression, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new(0);
        parser_engine.parse_expression(pair)
    }

    #[test]
    fn test_parse_dereference() {
        let input = "*ref";
        let result = parse_expression_input(input);

        let ast::Expression::Unary(unary) = result else {
            panic!("Expected UnaryExpression");
        };
        assert_eq!(unary.operator, ast::UnaryOperator::Star);

        match *unary.operand {
            ast::Expression::Identifier(id) if id.as_str() == "ref" => {}
            _ => panic!("Expected operand to be 'ref'"),
        }
    }

    #[test]
    fn test_parse_immutable_reference() {
        let input = "@value";
        let result = parse_expression_input(input);

        let ast::Expression::Unary(unary) = result else {
            panic!("Expected UnaryExpression");
        };
        assert_eq!(unary.operator, ast::UnaryOperator::At);

        match *unary.operand {
            ast::Expression::Identifier(id) if id.as_str() == "value" => {}
            _ => panic!("Expected operand to be 'value'"),
        }
    }

    #[test]
    fn test_parse_mutable_reference() {
        let input = "@value";
        let result = parse_expression_input(input);

        let ast::Expression::Unary(unary) = result else {
            panic!("Expected UnaryExpression");
        };
        assert_eq!(unary.operator, ast::UnaryOperator::At);

        match *unary.operand {
            ast::Expression::Identifier(id) if id.as_str() == "value" => {}
            _ => panic!("Expected operand to be 'value'"),
        }
    }
}
