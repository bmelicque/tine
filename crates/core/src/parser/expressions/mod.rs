mod access_or_call;
mod array;
mod binary;
mod block;
mod composite_literals;
mod dom;
mod exponentiation;
mod function;
mod identifier;
mod ifs;
mod loops;
mod tuple;
mod unary;

use pest::iterators::Pair;

use crate::ast;

use super::{parser::Rule, ParserEngine};

impl ParserEngine {
    pub fn parse_expression(&mut self, pair: Pair<'_, Rule>) -> ast::Expression {
        match pair.as_rule() {
            Rule::anonymous_expression
            | Rule::expression
            | Rule::primary
            | Rule::tuple_or_expression
            | Rule::access_or_call_root
            | Rule::type_annotation => {
                if let Some(inner) = pair.into_inner().next() {
                    self.parse_expression(inner)
                } else {
                    ast::Expression::Empty
                }
            }
            Rule::access_or_call_expression => self.parse_access_or_call(pair),
            Rule::array_expression => self.parse_array_expression(pair).into(),
            Rule::block => self.parse_block(pair).into(),
            Rule::composite_literal => self.parse_composite_literal(pair).into(),
            Rule::element_expression => self.parse_element_expression(pair).into(),
            Rule::equality | Rule::relation | Rule::addition | Rule::multiplication => {
                self.parse_binary_ltr_expression(pair).into()
            }
            Rule::exponentiation => self.parse_exponentiation(pair).into(),
            Rule::function_expression => self.parse_function_expression(pair).into(),
            Rule::value_identifier => self.parse_identifier(pair).into(),
            Rule::if_expression => self.parse_if_expression(pair).into(),
            Rule::if_decl_expression => self.parse_if_pat_expression(pair).into(),
            Rule::loop_expression => self.parse_loop(pair).into(),
            Rule::match_expression => self.parse_match_expression(pair).into(),
            Rule::tuple_expression => self.parse_tuple_expression(pair).into(),
            Rule::unary => self.parse_unary_expression(pair).into(),
            Rule::string_literal => ast::StringLiteral {
                loc: self.localize(pair.as_span()),
                text: pair.as_str().to_string(),
            }
            .into(),
            Rule::float_literal => self.parse_float_literal(pair).into(),
            Rule::integer_literal => self.parse_integer_literal(pair).into(),
            Rule::boolean_literal => ast::BooleanLiteral {
                loc: self.localize(pair.as_span()),
                value: pair.as_str() == "true",
            }
            .into(),
            _ => ast::Expression::Empty,
        }
    }

    pub fn parse_expression_or_anonymous(
        &mut self,
        pair: Pair<'_, Rule>,
    ) -> ast::ExpressionOrAnonymous {
        // { struct_literal_body | array_literal_body | expression }
        assert!(pair.as_rule() == Rule::anonymous_expression);
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::expression => self.parse_expression(inner).into(),
            Rule::struct_literal_body => self.parse_anonymous_struct_literal(inner).into(),
            _ => panic!(),
        }
    }

    fn parse_integer_literal(&mut self, pair: Pair<'_, Rule>) -> ast::IntLiteral {
        debug_assert_eq!(pair.as_rule(), Rule::integer_literal);
        let loc = self.localize(pair.as_span());
        let value_str = pair.as_str().replace('_', "");
        let value = value_str.parse().unwrap_or(0);
        ast::IntLiteral { loc, value }
    }

    fn parse_float_literal(&mut self, pair: Pair<'_, Rule>) -> ast::FloatLiteral {
        debug_assert_eq!(pair.as_rule(), Rule::float_literal);
        let loc = self.localize(pair.as_span());
        let value_str = pair.as_str().replace('_', "");
        let value = value_str
            .parse()
            .unwrap_or(ordered_float::OrderedFloat(0.0));
        ast::FloatLiteral { loc, value }
    }

    fn parse_match_expression(&mut self, pair: Pair<'_, Rule>) -> ast::MatchExpression {
        assert!(pair.as_rule() == Rule::match_expression);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let scrutinee = Box::new(self.parse_expression(inner.next().unwrap()));
        let arms = inner
            .next()
            .unwrap()
            .into_inner()
            .map(|arm| self.parse_match_arm(arm))
            .collect();
        ast::MatchExpression {
            loc,
            scrutinee,
            arms,
        }
    }

    fn parse_match_arm(&mut self, pair: Pair<'_, Rule>) -> ast::MatchArm {
        assert!(pair.as_rule() == Rule::match_arm);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();
        let pattern = Box::new(self.parse_pattern(inner.next().unwrap()));
        let expression = Box::new(self.parse_expression(inner.next().unwrap()));
        ast::MatchArm {
            loc,
            pattern,
            expression,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parser::parser::{Rule, TineParser},
        Location, Span,
    };
    use pest::Parser;

    fn parse_expression_input(input: &'static str, rule: Rule) -> ast::Expression {
        let pair = TineParser::parse(rule, input).unwrap().next().unwrap();
        let mut parser_engine = ParserEngine::new(0);
        parser_engine.parse_expression(pair)
    }

    #[test]
    fn test_parse_int_literal() {
        let input = "42";
        let result = parse_expression_input(input, Rule::integer_literal);

        let expected = ast::Expression::IntLiteral(ast::IntLiteral {
            loc: Location::new(0, Span::new(0, 2)),
            value: 42,
        });

        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_float_literal() {
        let input = "3.14";
        let result = parse_expression_input(input, Rule::float_literal);

        let expected = ast::Expression::FloatLiteral(ast::FloatLiteral {
            loc: Location::new(0, Span::new(0, 4)),
            value: ordered_float::OrderedFloat(3.14),
        });

        assert_eq!(result, expected);
    }
}
