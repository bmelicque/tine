use pest::iterators::Pair;

use crate::{
    ast,
    parser::{
        parser::{ParseError, Rule},
        ParserEngine,
    },
};

impl ParserEngine {
    pub fn parse_unary_expression(&mut self, pair: Pair<'static, Rule>) -> ast::UnaryExpression {
        assert!(pair.as_rule() == Rule::unary);
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let operator = self.parse_unary_operator(inner.next().unwrap());
        let operand = Box::new(self.parse_expression(inner.next().unwrap()));
        ast::UnaryExpression {
            span,
            operator,
            operand,
        }
    }

    fn parse_unary_operator(&mut self, pair: Pair<'static, Rule>) -> ast::UnaryOperator {
        assert!(pair.as_rule() == Rule::unary_op);
        match pair.as_str() {
            "&" => ast::UnaryOperator::Ampersand,
            "@" => ast::UnaryOperator::At,
            "!" => ast::UnaryOperator::Bang,
            "$" => ast::UnaryOperator::Dollar,
            "-" => ast::UnaryOperator::Minus,
            "*" => ast::UnaryOperator::Star,
            op => {
                self.errors.push(ParseError {
                    message: format!("Unknown unary operator: {}", op),
                    span: pair.as_span(),
                });
                ast::UnaryOperator::Star
            }
        }
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
        let mut parser_engine = ParserEngine::new();
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
