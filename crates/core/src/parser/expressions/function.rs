use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_function_expression(&mut self, pair: Pair<'_, Rule>) -> ast::FunctionExpression {
        assert_eq!(pair.as_rule(), Rule::function_expression);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let params = self.parse_function_params(inner.next().unwrap());
        let body = self.parse_function_body(inner.next().unwrap());
        ast::FunctionExpression { loc, params, body }
    }

    fn parse_function_params(&mut self, pair: Pair<'_, Rule>) -> Vec<ast::FunctionParam> {
        assert_eq!(pair.as_rule(), Rule::parameter_list);
        pair.into_inner()
            .map(|param_pair| self.parse_function_param(param_pair))
            .collect()
    }

    fn parse_function_param(&mut self, pair: Pair<'_, Rule>) -> ast::FunctionParam {
        assert_eq!(pair.as_rule(), Rule::parameter);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let name = self.parse_identifier(inner.next().unwrap());
        let type_annotation = self.parse_type(inner.next().unwrap());
        ast::FunctionParam {
            loc,
            name,
            type_annotation,
        }
    }

    fn parse_function_body(&mut self, pair: Pair<'_, Rule>) -> ast::FunctionBody {
        match pair.as_rule() {
            Rule::typed_block => self.parse_typed_block(pair).into(),
            Rule::expression => self.parse_expression(pair).into(),
            rule => unreachable!("unexpected rule {:?}", rule),
        }
    }

    fn parse_typed_block(&mut self, pair: Pair<'_, Rule>) -> ast::TypedBlock {
        assert_eq!(pair.as_rule(), Rule::typed_block);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let mut type_annotation = None;
        let mut block = ast::BlockExpression {
            loc,
            statements: vec![],
        };
        while let Some(next) = inner.next() {
            match next.as_rule() {
                Rule::type_annotation => type_annotation = Some(self.parse_type(next)),
                Rule::block => block = self.parse_block(next),
                rule => unreachable!("unexpected rule {:?}", rule),
            }
        }
        ast::TypedBlock {
            type_annotation,
            block,
        }
    }

    pub fn parse_predicate(&mut self, pair: Pair<'_, Rule>) -> ast::Predicate {
        assert_eq!(pair.as_rule(), Rule::predicate);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let params = self.parse_predicate_params(inner.next().unwrap());
        let body = self.parse_function_body(inner.next().unwrap());
        ast::Predicate { loc, params, body }
    }

    fn parse_predicate_params(&mut self, pair: Pair<'_, Rule>) -> Vec<ast::PredicateParam> {
        assert_eq!(pair.as_rule(), Rule::predicate_parameters);
        pair.into_inner()
            .map(|param_pair| self.parse_predicate_param(param_pair))
            .collect()
    }

    fn parse_predicate_param(&mut self, pair: Pair<'_, Rule>) -> ast::PredicateParam {
        assert_eq!(pair.as_rule(), Rule::predicate_param);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::parameter => self.parse_function_param(pair).into(),
            Rule::value_identifier => self.parse_identifier(pair).into(),
            rule => unreachable!("unexpected rule {:?}", rule),
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
    fn test_parse_empty_function() {
        let input = "() => {}";
        let result = parse_expression_input(input);

        let ast::Expression::Function(function) = result else {
            panic!("Expected a function");
        };

        assert_eq!(function.params.len(), 0);
    }
}
