use pest::iterators::Pair;

use crate::{ast, parser::utils::merge_span};

use super::{
    parser::{ParseError, Rule},
    ParserEngine,
};

impl ParserEngine {
    pub fn parse_expression(&mut self, pair: Pair<'static, Rule>) -> ast::Expression {
        match pair.as_rule() {
            Rule::anonymous_expression
            | Rule::expression
            | Rule::primary
            | Rule::type_annotation => {
                if let Some(inner) = pair.into_inner().next() {
                    self.parse_expression(inner)
                } else {
                    ast::Expression::Empty
                }
            }
            Rule::composite_literal => self.parse_composite_literal(pair).into(),
            Rule::equality | Rule::relation | Rule::addition | Rule::multiplication => {
                self.parse_binary_ltr_expression(pair).into()
            }
            Rule::exponentiation => self.parse_exponentiation(pair).into(),
            Rule::value_identifier => self.parse_identifier(pair).into(),
            Rule::member_expression => self.parse_field_access_expression(pair).into(),
            Rule::tuple_indexing => self.parse_tuple_indexing(pair).into(),
            Rule::string_literal => ast::StringLiteral {
                span: pair.as_span(),
            }
            .into(),
            Rule::number_literal => self.parse_number_literal(pair).into(),
            Rule::boolean_literal => ast::BooleanLiteral {
                span: pair.as_span(),
                value: pair.as_str() == "true",
            }
            .into(),
            _ => ast::Expression::Empty,
        }
    }

    pub fn parse_expression_or_anonymous(
        &mut self,
        pair: Pair<'static, Rule>,
    ) -> ast::ExpressionOrAnonymous {
        // { struct_literal_body | array_literal_body | expression }
        assert!(pair.as_rule() == Rule::anonymous_expression);
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::expression => self.parse_expression(inner).into(),
            Rule::array_literal_body => self.parse_anonymous_array_literal(inner).into(),
            Rule::struct_literal_body => self.parse_anonymous_struct_literal(inner).into(),
            _ => panic!(),
        }
    }

    fn parse_binary_ltr_expression(&mut self, pair: Pair<'static, Rule>) -> ast::Expression {
        let span = pair.as_span().to_owned();
        let mut inner = pair.into_inner();
        let Some(next) = inner.next() else {
            return ast::Expression::Empty;
        };
        let mut left = self.parse_expression(next);

        let mut is_binary = false;
        while let Some(op_pair) = inner.next() {
            if !is_binary && left.is_empty() {
                self.errors.push(ParseError {
                    message: "Expression expected".to_string(),
                    span: op_pair.as_span(),
                });
            }
            is_binary = true;
            let operator = op_pair.as_str().to_string();

            let Some(right_pair) = inner.next() else {
                self.errors.push(ParseError {
                    message: "Expression expected".to_string(),
                    span: op_pair.as_span(),
                });
                continue;
            };

            let right = self.parse_expression(right_pair);
            if right.is_empty() {
                self.errors.push(ParseError {
                    message: "Expression expected".to_string(),
                    span: op_pair.as_span(),
                });
            }

            left = ast::BinaryExpression {
                span,
                left: Box::new(left),
                operator: operator.into(),
                right: Box::new(right),
            }
            .into();
        }

        left
    }

    fn parse_exponentiation(&mut self, pair: Pair<'static, Rule>) -> ast::Expression {
        assert!(pair.as_rule() == Rule::exponentiation);
        let span = pair.as_span();
        let mut node = ast::Expression::Empty;
        for sub_pair in pair.into_inner().rev() {
            let left = self.parse_expression(sub_pair);
            if node == ast::Expression::Empty {
                node = left;
                continue;
            }
            node = ast::BinaryExpression {
                left: Box::new(left),
                operator: ast::BinaryOperator::Pow,
                right: Box::new(node),
                // FIXME:
                span,
            }
            .into();
        }
        node
    }

    fn parse_field_access_expression(
        &mut self,
        pair: Pair<'static, Rule>,
    ) -> ast::FieldAccessExpression {
        assert!(pair.as_rule() == Rule::member_expression);
        let mut inner = pair.into_inner();
        let mut node = self.parse_expression(inner.next().unwrap());

        while let Some(sub_pair) = inner.next() {
            let right_span = sub_pair.as_span();
            let prop = self.parse_identifier(sub_pair);
            let left_span = prop.span;
            node = ast::FieldAccessExpression {
                span: merge_span(left_span, right_span),
                object: Box::new(node),
                prop,
            }
            .into()
        }

        match node {
            ast::Expression::FieldAccess(n) => n,
            _ => panic!("Unexpected variant!"),
        }
    }

    fn parse_tuple_indexing(&mut self, pair: Pair<'static, Rule>) -> ast::TupleIndexingExpression {
        assert!(pair.as_rule() == Rule::tuple_indexing);
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let tuple = Box::new(self.parse_expression(inner.next().unwrap()));

        let index = self.parse_number_literal(inner.next().unwrap());

        ast::TupleIndexingExpression { span, tuple, index }
    }

    fn parse_number_literal(&mut self, pair: Pair<'static, Rule>) -> ast::NumberLiteral {
        ast::NumberLiteral {
            span: pair.as_span(),
            value: pair.as_str().parse().unwrap_or(0.0),
        }
    }

    fn parse_identifier(&mut self, pair: Pair<'static, Rule>) -> ast::Identifier {
        ast::Identifier {
            span: pair.as_span(),
        }
    }
}
