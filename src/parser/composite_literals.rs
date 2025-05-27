use pest::iterators::Pair;

use crate::ast::{self, ExpressionOrAnonymous};

use super::{parser::Rule, ParserEngine};

impl ParserEngine {
    pub fn parse_composite_literal(&mut self, pair: Pair<'static, Rule>) -> ast::CompositeLiteral {
        assert!(pair.as_rule() == Rule::composite_literal);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::map_literal => self.parse_map_literal(pair).into(),
            Rule::array_literal => self.parse_array_literal(pair).into(),
            Rule::option_literal => self.parse_option_literal(pair).into(),
            Rule::struct_literal => self.parse_struct_literal(pair).into(),
            Rule::variant_literal => self.parse_variant_literal(pair).into(),
            _ => panic!("Not implemented, got rule: {:?}", pair.as_rule()),
        }
    }

    fn parse_map_literal(&mut self, pair: Pair<'static, Rule>) -> ast::MapLiteral {
        assert!(pair.as_rule() == Rule::map_literal);
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let ty = self.parse_map_type(inner.next().unwrap());

        let map_body = inner.next().unwrap();
        assert!(map_body.as_rule() == Rule::map_body);
        let entries = map_body
            .into_inner()
            .map(|entry_pair| self.parse_map_entry(entry_pair))
            .collect();

        ast::MapLiteral { span, ty, entries }
    }

    fn parse_map_entry(&mut self, pair: Pair<'static, Rule>) -> ast::MapEntry {
        assert!(pair.as_rule() == Rule::map_entry);
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let key_pair = inner.next().unwrap().into_inner().next().unwrap();
        let key = Box::new(self.parse_expression(key_pair));
        let value = Box::new(self.parse_expression_or_anonymous(inner.next().unwrap()));

        ast::MapEntry { span, key, value }
    }

    fn parse_array_literal(&mut self, pair: Pair<'static, Rule>) -> ast::ArrayLiteral {
        assert!(pair.as_rule() == Rule::array_literal);
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let ty = self.parse_array_type(inner.next().unwrap());

        let elements = self.parse_array_literal_body(inner.next().unwrap());

        ast::ArrayLiteral { span, ty, elements }
    }

    fn parse_array_literal_body(
        &mut self,
        pair: Pair<'static, Rule>,
    ) -> Vec<ExpressionOrAnonymous> {
        pair.into_inner()
            .map(|el_pair| self.parse_expression_or_anonymous(el_pair))
            .filter(|expr| !expr.is_empty())
            .collect()
    }

    fn parse_option_literal(&mut self, pair: Pair<'static, Rule>) -> ast::OptionLiteral {
        assert!(pair.as_rule() == Rule::option_literal);
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let ty = self.parse_option_type(inner.next().unwrap());

        let value = inner
            .next()
            .and_then(|pair| Some(self.parse_expression_or_anonymous(pair)))
            .map(Box::new);

        ast::OptionLiteral { span, ty, value }
    }

    fn parse_struct_literal(&mut self, pair: Pair<'static, Rule>) -> ast::StructLiteral {
        assert!(pair.as_rule() == Rule::struct_literal);
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let ty = self.parse_named_type_with_args(inner.next().unwrap());
        let fields = self.parse_struct_literal_body(inner.next().unwrap());

        ast::StructLiteral { span, ty, fields }
    }

    pub fn parse_anonymous_struct_literal(
        &mut self,
        pair: Pair<'static, Rule>,
    ) -> ast::AnonymousStructLiteral {
        assert!(pair.as_rule() == Rule::struct_literal_body);
        let span = pair.as_span();
        let fields = self.parse_struct_literal_body(pair);

        ast::AnonymousStructLiteral { span, fields }
    }

    fn parse_struct_literal_body(
        &mut self,
        pair: Pair<'static, Rule>,
    ) -> Vec<ast::StructLiteralField> {
        assert!(pair.as_rule() == Rule::struct_literal_body);
        pair.into_inner()
            .map(|field| self.parse_struct_literal_field(field))
            .collect()
    }

    fn parse_struct_literal_field(&mut self, pair: Pair<'static, Rule>) -> ast::StructLiteralField {
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let prop = inner.next().unwrap().as_str().to_string();
        let value = self.parse_expression(inner.next().unwrap());

        ast::StructLiteralField { span, prop, value }
    }

    fn parse_variant_literal(&mut self, pair: Pair<'static, Rule>) -> ast::VariantLiteral {
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let ty = self.parse_variant_parent(inner.next().unwrap());
        let name = inner.next().unwrap().as_str().to_string();
        let body = inner
            .next()
            .map(|pair| self.parse_variant_literal_body(pair));

        ast::VariantLiteral {
            span,
            ty,
            name,
            body,
        }
    }

    fn parse_variant_parent(&mut self, pair: Pair<'static, Rule>) -> ast::NamedType {
        match pair.as_rule() {
            Rule::generic_type => self.parse_named_type_with_args(pair),
            Rule::type_name => self.parse_named_type(pair),
            _ => unreachable!(),
        }
    }

    fn parse_variant_literal_body(&mut self, pair: Pair<'static, Rule>) -> ast::VariantLiteralBody {
        assert!(pair.as_rule() == Rule::variant_literal_body);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::array_literal_body => self.parse_array_literal_body(pair).into(),
            Rule::struct_literal_body => self.parse_struct_literal_body(pair).into(),
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse_composite_literal_input(input: &'static str, rule: Rule) -> ast::CompositeLiteral {
        let pair = MyLanguageParser::parse(rule, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        parser_engine.parse_composite_literal(pair)
    }

    #[test]
    fn test_parse_map_literal() {
        let input = r#"string#number("key": 42, "another_key": 99)"#;
        let result = parse_composite_literal_input(input, Rule::composite_literal);

        match result {
            ast::CompositeLiteral::Map(map) => {
                assert_eq!(map.entries.len(), 2);

                // Check the first entry
                let entry1 = &map.entries[0];
                assert!(matches!(*entry1.key, ast::Expression::StringLiteral(_)));
                assert!(matches!(
                    *entry1.value,
                    ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(ast::NumberLiteral{value, ..})) if value == 42.0
                ));

                // Check the second entry
                let entry2 = &map.entries[1];
                assert!(matches!(*entry2.key, ast::Expression::StringLiteral(_)));
                assert!(matches!(
                    *entry2.value,
                    ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(ast::NumberLiteral{value, ..})) if value == 99.0
                ));
            }
            _ => panic!("Expected MapLiteral"),
        }
    }

    #[test]
    fn test_parse_array_literal() {
        let input = r#"[]number(1, 2, 3)"#;
        let result = parse_composite_literal_input(input, Rule::composite_literal);

        match result {
            ast::CompositeLiteral::Array(array) => {
                assert_eq!(array.elements.len(), 3);

                assert!(matches!(
                    array.elements[0],
                    ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
                        ast::NumberLiteral { value, .. }
                    )) if value == 1.0
                ));
                assert!(matches!(
                    array.elements[1],
                    ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
                        ast::NumberLiteral { value, .. }
                    )) if value == 2.0
                ));
                assert!(matches!(
                    array.elements[2],
                    ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
                        ast::NumberLiteral { value, .. }
                    )) if value == 3.0
                ));
            }
            _ => panic!("Expected ArrayLiteral"),
        }
    }

    #[test]
    fn test_parse_empty_array_literal() {
        let input = r#"[]string()"#;
        let result = parse_composite_literal_input(input, Rule::composite_literal);

        match result {
            ast::CompositeLiteral::Array(array) => {
                assert_eq!(array.elements.len(), 0, "{:?}", array.elements);
            }
            _ => panic!("Expected ArrayLiteral"),
        }
    }

    #[test]
    fn test_parse_option_literal() {
        let input = r#"?number(42)"#;
        let result = parse_composite_literal_input(input, Rule::composite_literal);

        match result {
            ast::CompositeLiteral::Option(option) => {
                assert!(matches!(
                    **option.value.as_ref().unwrap(),
                    ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
                        ast::NumberLiteral { value, .. }
                    )) if value == 42.0
                ));
            }
            _ => panic!("Expected OptionLiteral"),
        }
    }

    #[test]
    fn test_parse_struct_literal() {
        let input = r#"User(name: "John", age: 30)"#;
        let result = parse_composite_literal_input(input, Rule::composite_literal);

        match result {
            ast::CompositeLiteral::Struct(struct_literal) => {
                assert_eq!(struct_literal.fields.len(), 2);

                let field1 = &struct_literal.fields[0];
                assert_eq!(field1.prop, "name");

                let field2 = &struct_literal.fields[1];
                assert_eq!(field2.prop, "age");
            }
            _ => panic!("Expected StructLiteral"),
        }
    }

    #[test]
    fn test_parse_anonymous_struct_literal() {
        let input = r#"(name: "John", age: 30)"#;
        let pair = MyLanguageParser::parse(Rule::struct_literal_body, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        let result = parser_engine.parse_anonymous_struct_literal(pair);

        assert_eq!(result.fields.len(), 2);

        // Check the first field
        let field1 = &result.fields[0];
        assert_eq!(field1.prop, "name");

        // Check the second field
        let field2 = &result.fields[1];
        assert_eq!(field2.prop, "age");
    }

    #[test]
    fn test_parse_variant_literal_with_struct_body() {
        let input = r#"MyEnum.Variant(field1: "value1", field2: 42)"#;
        let result = parse_composite_literal_input(input, Rule::composite_literal);

        let ast::CompositeLiteral::Variant(result) = result else {
            panic!("Variant literal expected");
        };

        assert_eq!(result.name, "Variant");
        assert_eq!(result.ty.name, "MyEnum");

        match result.body {
            Some(ast::VariantLiteralBody::Struct(fields)) => {
                assert_eq!(fields.len(), 2);

                // Check the first field
                let field1 = &fields[0];
                assert_eq!(field1.prop, "field1");
                assert!(
                    matches!(
                        field1.value,
                        ast::Expression::StringLiteral(ast::StringLiteral { ref span, .. }) if span.as_str() == "\"value1\""
                    ),
                    "got {:?}",
                    field1.value
                );

                // Check the second field
                let field2 = &fields[1];
                assert_eq!(field2.prop, "field2");
                assert!(matches!(
                    field2.value,
                    ast::Expression::NumberLiteral(ast::NumberLiteral { value, .. }) if value == 42.0
                ));
            }
            _ => panic!("Expected Struct body for the variant"),
        }
    }

    #[test]
    fn test_parse_variant_literal_with_array_body() {
        let input = r#"MyEnum.Variant(1, 2, 3)"#;
        let pair = MyLanguageParser::parse(Rule::variant_literal, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        let result = parser_engine.parse_variant_literal(pair);

        assert_eq!(result.name, "Variant");
        assert_eq!(result.ty.name, "MyEnum");

        match result.body {
            Some(ast::VariantLiteralBody::Tuple(elements)) => {
                assert_eq!(elements.len(), 3);

                assert!(matches!(
                    elements[0],
                    ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
                        ast::NumberLiteral { value, .. }
                    )) if value == 1.0
                ));
                assert!(matches!(
                    elements[1],
                    ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
                        ast::NumberLiteral { value, .. }
                    )) if value == 2.0
                ));
                assert!(matches!(
                    elements[2],
                    ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
                        ast::NumberLiteral { value, .. }
                    )) if value == 3.0
                ));
            }
            _ => panic!("Expected Array body for the variant"),
        }
    }

    #[test]
    fn test_parse_variant_literal_without_body() {
        let input = r#"MyEnum.Variant"#;
        let pair = MyLanguageParser::parse(Rule::variant_literal, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        let result = parser_engine.parse_variant_literal(pair);

        assert_eq!(result.name, "Variant");
        assert_eq!(result.ty.name, "MyEnum");
        assert!(result.body.is_none());
    }
}
