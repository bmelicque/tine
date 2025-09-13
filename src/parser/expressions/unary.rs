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
            "*" => ast::UnaryOperator::Deref,
            "&" => ast::UnaryOperator::MutableRef,
            "@" => ast::UnaryOperator::ImmutableRef,
            op => {
                self.errors.push(ParseError {
                    message: format!("Unknown unary operator: {}", op),
                    span: pair.as_span(),
                });
                ast::UnaryOperator::Deref
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
        assert_eq!(unary.operator, ast::UnaryOperator::Deref);

        match *unary.operand {
            ast::Expression::Identifier(id) if id.as_str() == "ref" => {}
            _ => panic!("Expected operand to be 'ref'"),
        }
    }
}
