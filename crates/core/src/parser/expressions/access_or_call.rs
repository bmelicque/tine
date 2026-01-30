use pest::iterators::Pair;

use crate::{
    ast,
    diagnostics::DiagnosticKind,
    parser::{parser::Rule, ParserEngine},
    Location,
};

impl ParserEngine {
    pub fn parse_access_or_call(&mut self, pair: Pair<'_, Rule>) -> ast::Expression {
        debug_assert_eq!(pair.as_rule(), Rule::access_or_call_expression);
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let mut node = self.parse_expression(inner.next().unwrap());

        for sub_pair in inner {
            match sub_pair.as_rule() {
                Rule::call_arguments => node = self.parse_call(node, sub_pair).into(),
                Rule::member_suffix => node = self.parse_member_expression(node, sub_pair).into(),
                Rule::expression_error => {
                    let loc = self.localize(span);
                    node = ast::Expression::Invalid(ast::InvalidExpression { loc });
                    self.error(DiagnosticKind::InvalidExpression, loc);
                    return node;
                }
                rule => unreachable!("unexpected rule '{:?}'", rule),
            }
        }

        node
    }

    fn parse_call(
        &mut self,
        root: ast::Expression,
        right_pair: Pair<'_, Rule>,
    ) -> ast::CallExpression {
        let right_loc = self.localize(right_pair.as_span());
        let left_loc = root.loc();
        let loc = Location::merge(left_loc, right_loc);
        println!("{:?} {:?} {:?}", left_loc, right_loc, loc);

        let args = self.parse_call_arguments(right_pair);
        ast::CallExpression {
            loc,
            callee: Box::new(root),
            args,
        }
    }

    fn parse_call_arguments(&mut self, pair: Pair<'_, Rule>) -> Vec<ast::CallArgument> {
        assert_eq!(pair.as_rule(), Rule::call_arguments);
        pair.into_inner()
            .map(|sub_pair| self.parse_call_argument(sub_pair))
            .filter(|expr| !matches!(expr, ast::CallArgument::Expression(ast::Expression::Empty)))
            .collect()
    }

    fn parse_call_argument(&mut self, pair: Pair<'_, Rule>) -> ast::CallArgument {
        assert_eq!(pair.as_rule(), Rule::call_argument);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::expression => self.parse_expression(pair).into(),
            Rule::callback => self.parse_callback(pair).into(),
            rule => unreachable!("Unexpected rule {:?}", rule),
        }
    }

    pub fn parse_member_expression(
        &mut self,
        root: ast::Expression,
        right_pair: Pair<'_, Rule>,
    ) -> ast::MemberExpression {
        debug_assert_eq!(right_pair.as_rule(), Rule::member_suffix);
        let right_loc = self.localize(right_pair.as_span());
        let left_loc = root.loc();
        let loc = Location::merge(left_loc, right_loc);

        let Some(right_pair) = right_pair.into_inner().next() else {
            self.error(DiagnosticKind::InvalidMember, right_loc.increment());
            return ast::MemberExpression {
                loc,
                object: Box::new(root),
                prop: None,
            };
        };

        match right_pair.as_rule() {
            Rule::value_identifier => ast::MemberExpression {
                loc,
                object: Box::new(root),
                prop: Some(self.parse_identifier(right_pair).into()),
            },
            Rule::integer_literal => {
                let index = self.parse_integer_literal(right_pair);
                if index.value < 0 {
                    self.error(DiagnosticKind::NegativeTupleIndex, index.loc);
                }
                ast::MemberExpression {
                    loc,
                    object: Box::new(root),
                    prop: Some(index.into()),
                }
            }
            rule => unreachable!("unexpected rule '{:?}'", rule),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parser::parser::{Rule, TineParser},
        Span,
    };
    use pest::Parser;

    fn parse_expression_input(input: &'static str) -> ast::Expression {
        let pair = TineParser::parse(Rule::access_or_call_expression, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new(0);
        parser_engine.parse_expression(pair)
    }

    #[test]
    fn test_parse_function_call() {
        let input = "function()";
        let expected = ast::Expression::Call(ast::CallExpression {
            loc: Location::new(0, Span::new(0, input.len() as u32)),
            callee: Box::new(ast::Expression::Identifier(ast::Identifier {
                loc: Location::new(0, Span::new(0, 8)),
                text: "function".to_string(),
            })),
            args: vec![],
        });
        let result = parse_expression_input(input);

        assert_eq!(result, expected);
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
            ast::CallArgument::Expression(ast::Expression::IntLiteral(n)) if n.value == 1 => {}
            _ => panic!("Expected number argument with value 42"),
        }
    }

    #[test]
    fn test_parse_field_access_expression() {
        let input = "object.property";
        let result = parse_expression_input(input);

        let ast::Expression::Member(expr) = result else {
            panic!("Expected FieldAccessExpression")
        };
        match expr.prop.unwrap() {
            ast::MemberProp::FieldName(ident) if ident.as_str() == "property" => {}
            node => panic!("expected FieldName with name 'property', got {:?}", node),
        }
    }

    #[test]
    fn test_parse_field_access_expression_with_trailing() {
        let input = "object.";
        let result = parse_expression_input(input);

        let ast::Expression::Member(expr) = result else {
            panic!("Expected MemberExpression")
        };
        assert_eq!(expr.prop, None);
    }
}
