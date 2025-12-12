use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_if_expression(&mut self, pair: Pair<'_, Rule>) -> ast::IfExpression {
        assert!(pair.as_rule() == Rule::if_expression);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let condition = Box::new(self.parse_expression(inner.next().unwrap()));
        let consequent = Box::new(self.parse_block(inner.next().unwrap()));
        let alternate = inner
            .next()
            .map(|pair| Box::new(self.parse_alternate(pair)));
        ast::IfExpression {
            loc,
            condition,
            consequent,
            alternate,
        }
    }

    /// Parse if expressions that use pattern matching as their condition
    pub fn parse_if_pat_expression(&mut self, pair: Pair<'_, Rule>) -> ast::IfPatExpression {
        assert!(pair.as_rule() == Rule::if_decl_expression);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let pattern = Box::new(self.parse_pattern(inner.next().unwrap()));
        let scrutinee = Box::new(self.parse_expression(inner.next().unwrap()));
        let consequent = Box::new(self.parse_block(inner.next().unwrap()));
        let alternate = inner
            .next()
            .map(|pair| Box::new(self.parse_alternate(pair)));
        ast::IfPatExpression {
            loc,
            pattern,
            scrutinee,
            consequent,
            alternate,
        }
    }

    fn parse_alternate(&mut self, pair: Pair<'_, Rule>) -> ast::Alternate {
        assert!(pair.as_rule() == Rule::alternate);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::block => self.parse_block(pair).into(),
            Rule::if_expression => self.parse_if_expression(pair).into(),
            Rule::if_decl_expression => self.parse_if_pat_expression(pair).into(),
            rule => unreachable!("unexpected rule {:?}", rule),
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
        let mut parser_engine = ParserEngine::new(0);
        parser_engine.parse_expression(pair)
    }

    #[test]
    fn test_parse_if_expression() {
        let input = "if true {}";
        let result = parse_expression_input(input);

        let ast::Expression::If(expr) = result else {
            panic!("Expected IfExpression!")
        };

        match *expr.condition {
            ast::Expression::BooleanLiteral(_) => {}
            _ => panic!("Expected boolean condition"),
        }
    }
}
