use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_call_expression(&mut self, pair: Pair<'static, Rule>) -> ast::CallExpression {
        assert_eq!(pair.as_rule(), Rule::call_expression);
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let callee = Box::new(self.parse_callee(inner.next().unwrap()));
        let args = self.parse_call_arguments(inner.next().unwrap());

        ast::CallExpression { span, callee, args }
    }

    fn parse_callee(&mut self, pair: Pair<'static, Rule>) -> ast::Expression {
        assert_eq!(pair.as_rule(), Rule::callee);
        let pair = pair.into_inner().next().unwrap();
        self.parse_expression(pair)
    }

    fn parse_call_arguments(&mut self, pair: Pair<'static, Rule>) -> Vec<ast::CallArgument> {
        assert_eq!(pair.as_rule(), Rule::call_arguments);
        pair.into_inner()
            .map(|sub_pair| self.parse_call_argument(sub_pair))
            .filter(|expr| !matches!(expr, ast::CallArgument::Expression(ast::Expression::Empty)))
            .collect()
    }

    fn parse_call_argument(&mut self, pair: Pair<'static, Rule>) -> ast::CallArgument {
        assert_eq!(pair.as_rule(), Rule::call_argument);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::expression => self.parse_expression(pair).into(),
            Rule::predicate => self.parse_predicate(pair).into(),
            rule => unreachable!("Unexpected rule {:?}", rule),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse_expression_input(input: &'static str) -> ast::Expression {
        let pair = MyLanguageParser::parse(Rule::call_expression, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        parser_engine.parse_expression(pair)
    }

    #[test]
    fn test_parse_function_call() {
        let input = "function()";
        let result = parse_expression_input(input);

        let ast::Expression::Call(call) = result else {
            panic!("Expected CallExpression");
        };
        match *call.callee {
            ast::Expression::Identifier(id) if id.as_str() == "function" => {}
            _ => panic!("Expected callee to be 'function'"),
        }
        assert_eq!(call.args.len(), 0, "expected no args, got {:?}", call.args);
    }

    #[test]
    fn test_parse_function_call_with_args() {
        let input = "function(1, 2, 3)";
        let result = parse_expression_input(input);

        let ast::Expression::Call(call) = result else {
            panic!("Expected CallExpression");
        };
        match *call.callee {
            ast::Expression::Identifier(id) if id.as_str() == "function" => {}
            _ => panic!("Expected callee to be 'function'"),
        }
        assert_eq!(call.args.len(), 3, "expected no args, got {:?}", call.args);

        match &call.args[0] {
            ast::CallArgument::Expression(ast::Expression::NumberLiteral(n)) if n.value == 1.0 => {}
            _ => panic!("Expected number argument with value 42"),
        }
    }
}
