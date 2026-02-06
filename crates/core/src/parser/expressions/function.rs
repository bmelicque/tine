use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_function_expression(&mut self, pair: Pair<'_, Rule>) -> ast::FunctionExpression {
        debug_assert_eq!(pair.as_rule(), Rule::function_expression);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let name = if inner.peek().unwrap().as_rule() == Rule::value_identifier {
            Some(self.parse_identifier(inner.next().unwrap()))
        } else {
            None
        };
        let params = self.parse_function_params(inner.next().unwrap());
        let return_type = if inner.peek().unwrap().as_rule() == Rule::type_annotation {
            Some(self.parse_type(inner.next().unwrap()))
        } else {
            None
        };
        let body = self.parse_block(inner.next().unwrap());
        ast::FunctionExpression {
            loc,
            name,
            params,
            return_type,
            body,
        }
    }

    pub fn parse_function_params(&mut self, pair: Pair<'_, Rule>) -> Vec<ast::FunctionParam> {
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
        let type_annotation = self.parse_type(inner.next().unwrap()).into();
        ast::FunctionParam {
            loc,
            name,
            type_annotation,
        }
    }

    pub fn parse_callback(&mut self, pair: Pair<'_, Rule>) -> ast::Callback {
        assert_eq!(pair.as_rule(), Rule::callback);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let params = self.parse_predicate_params(inner.next().unwrap());
        let body = Box::new(self.parse_block(inner.next().unwrap()).into());
        ast::Callback { loc, params, body }
    }

    fn parse_predicate_params(&mut self, pair: Pair<'_, Rule>) -> Vec<ast::CallbackParam> {
        assert_eq!(pair.as_rule(), Rule::callback_parameters);
        pair.into_inner()
            .map(|param_pair| self.parse_predicate_param(param_pair))
            .collect()
    }

    fn parse_predicate_param(&mut self, pair: Pair<'_, Rule>) -> ast::CallbackParam {
        assert_eq!(pair.as_rule(), Rule::callback_param);
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
    use crate::parser::parser::{Rule, TineParser};
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
        let input = "fn () {}";
        let result = parse_expression_input(input);

        let ast::Expression::Function(function) = result else {
            panic!("Expected a function, got {:?}", result);
        };

        assert_eq!(function.params.len(), 0);
    }
}
