use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_loop(&mut self, pair: Pair<'static, Rule>) -> ast::Loop {
        assert_eq!(pair.as_rule(), Rule::loop_expression);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::for_expression => self.parse_for_expression(pair).into(),
            Rule::for_in_expression => self.parse_for_in_expression(pair).into(),
            rule => unreachable!("unexpected rule {:?}", rule),
        }
    }

    fn parse_for_expression(&mut self, pair: Pair<'static, Rule>) -> ast::ForExpression {
        assert_eq!(pair.as_rule(), Rule::for_expression);
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let condition = Box::new(self.parse_expression(inner.next().unwrap()));
        let body = self.parse_block(inner.next().unwrap());
        ast::ForExpression {
            span,
            condition,
            body,
        }
    }

    fn parse_for_in_expression(&mut self, pair: Pair<'static, Rule>) -> ast::ForInExpression {
        assert_eq!(pair.as_rule(), Rule::for_in_expression);
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let pattern = Box::new(self.parse_pattern(inner.next().unwrap()));
        let iterable = Box::new(self.parse_expression(inner.next().unwrap()));
        let body = self.parse_block(inner.next().unwrap());
        ast::ForInExpression {
            span,
            pattern,
            iterable,
            body,
        }
    }
}
