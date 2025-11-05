use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, utils::merge_span, ParserEngine},
};

impl ParserEngine {
    pub fn parse_field_access_expression(
        &mut self,
        pair: Pair<'static, Rule>,
    ) -> ast::FieldAccessExpression {
        assert!(pair.as_rule() == Rule::member_expression);
        let mut inner = pair.into_inner();
        let mut node = self.parse_expression(inner.next().unwrap());

        for sub_pair in inner {
            let right_span = sub_pair.as_span();
            let prop = self.parse_identifier(sub_pair);
            let left_span = node.as_span();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse_expression_input(input: &'static str) -> ast::Expression {
        let pair = MyLanguageParser::parse(Rule::member_expression, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        parser_engine.parse_expression(pair)
    }

    #[test]
    fn test_parse_field_access_expression() {
        let input = "object.property";
        let result = parse_expression_input(input);

        let ast::Expression::FieldAccess(field_access) = result else {
            panic!("Expected FieldAccessExpression")
        };
        assert_eq!(field_access.object.as_span().as_str(), "object");
        assert_eq!(field_access.prop.span.as_str(), "property");
    }
}
