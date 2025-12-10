use pest::iterators::{Pair, Pairs};

use crate::ast;

use super::{parser::Rule, ParserEngine};

impl ParserEngine {
    pub fn parse_pattern(&mut self, pair: Pair<'_, Rule>) -> ast::Pattern {
        match pair.as_rule() {
            Rule::pattern | Rule::grouped_pattern | Rule::pattern_element => {
                self.parse_pattern(pair.into_inner().next().unwrap())
            }
            Rule::identifier_pattern => self.parse_identifier_pattern(pair).into(),
            Rule::literal_pattern => self.parse_literal_pattern(pair).into(),
            Rule::struct_pattern => self.parse_struct_pattern(pair).into(),
            Rule::tuple_pattern => self.parse_tuple_pattern(pair).into(),
            Rule::variant_pattern => self.parse_variant_pattern(pair).into(),
            rule => unreachable!("got unexpected rule {:?}", rule),
        }
    }

    fn parse_identifier_pattern(&mut self, pair: Pair<'_, Rule>) -> ast::IdentifierPattern {
        match pair.as_str() {
            "break" | "continue" | "else" | "for" | "if" | "in" | "match" | "return" | "use" => {
                self.error(
                    format!("invalid identifier: '{}' is a reserved name", pair.as_str()),
                    pair.as_span().into(),
                );
            }
            _ => {}
        }

        let ident = ast::Identifier {
            span: pair.as_span().into(),
            text: pair.as_str().to_string(),
        };
        ast::IdentifierPattern(ident)
    }

    fn parse_literal_pattern(&mut self, pair: Pair<'_, Rule>) -> ast::LiteralPattern {
        let span = pair.as_span().into();
        let text = pair.as_str().to_string();
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::boolean_literal => ast::BooleanLiteral {
                span,
                value: inner.as_str() == "true",
            }
            .into(),
            Rule::number_literal => ast::NumberLiteral {
                span,
                value: inner
                    .as_str()
                    .parse()
                    .unwrap_or(ordered_float::OrderedFloat(0.0)),
            }
            .into(),
            Rule::string_literal => ast::StringLiteral { span, text }.into(),
            rule => unreachable!("unexpected rule {:?}", rule),
        }
    }

    fn parse_struct_pattern(&mut self, pair: Pair<'_, Rule>) -> ast::StructPattern {
        let span = pair.as_span().into();
        let mut inner = pair.into_inner();
        let ty = Box::new(self.parse_named_type(inner.next().unwrap()));
        let fields = self.parse_struct_pattern_fields(inner.next().unwrap());
        ast::StructPattern { span, ty, fields }
    }

    fn parse_struct_pattern_fields(
        &mut self,
        pair: Pair<'_, Rule>,
    ) -> Vec<ast::StructPatternField> {
        assert!(pair.as_rule() == Rule::struct_pattern_elements);
        pair.into_inner()
            .map(|element| self.parse_struct_pattern_field(element))
            .collect()
    }

    fn parse_struct_pattern_field(&mut self, pair: Pair<'_, Rule>) -> ast::StructPatternField {
        assert!(pair.as_rule() == Rule::struct_pattern_field);
        let span = pair.as_span().into();
        let mut inner = pair.into_inner();
        let identifier = self.parse_identifier(inner.next().unwrap());
        let pattern = inner.next().map(|pair| self.parse_pattern(pair));
        ast::StructPatternField {
            span,
            identifier,
            pattern,
        }
    }

    fn parse_tuple_pattern(&mut self, pair: Pair<'_, Rule>) -> ast::TuplePattern {
        let span = pair.as_span().into();
        let elements = pair
            .into_inner()
            .map(|element| self.parse_pattern(element))
            .collect();
        ast::TuplePattern { span, elements }
    }

    fn parse_variant_pattern(&mut self, pair: Pair<'_, Rule>) -> ast::VariantPattern {
        let span = pair.as_span().into();
        let mut inner = pair.into_inner().next().unwrap().into_inner();
        let ty = Box::new(self.parse_named_type(inner.next().unwrap()));
        let name = inner.next().unwrap().as_str().to_string();
        let body = self.parse_variant_pattern_body(inner);

        ast::VariantPattern {
            span,
            ty,
            name,
            body,
        }
    }

    fn parse_variant_pattern_body(
        &mut self,
        mut pairs: Pairs<'_, Rule>,
    ) -> Option<ast::VariantPatternBody> {
        let Some(next) = pairs.next() else {
            return None;
        };
        if next.as_rule() == Rule::struct_pattern_elements {
            return Some(self.parse_struct_pattern_fields(next).into());
        }
        let mut elements = vec![self.parse_pattern(next)];
        for next in pairs {
            elements.push(self.parse_pattern(next));
        }
        Some(ast::VariantPatternBody::Tuple(elements.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse_pattern_input(input: &'static str, rule: Rule) -> ast::Pattern {
        let pair = MyLanguageParser::parse(rule, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        parser_engine.parse_pattern(pair)
    }

    #[test]
    fn test_parse_identifier_pattern() {
        let input = "username";
        let result = parse_pattern_input(input, Rule::identifier_pattern);

        match result {
            ast::Pattern::Identifier(identifier) => {
                assert_eq!(identifier.as_str(), "username");
            }
            _ => panic!("Expected IdentifierPattern"),
        }
    }

    #[test]
    fn test_parse_struct_pattern() {
        let input = "User (username, age)";
        let result = parse_pattern_input(input, Rule::struct_pattern);

        match result {
            ast::Pattern::Struct(struct_pattern) => {
                assert_eq!(struct_pattern.ty.name.as_str(), "User");
                assert_eq!(struct_pattern.fields.len(), 2);

                // Check the first field
                let field1 = &struct_pattern.fields[0];
                assert_eq!(field1.identifier.as_str(), "username");
                assert!(field1.pattern.is_none());

                // Check the second field
                let field2 = &struct_pattern.fields[1];
                assert_eq!(field2.identifier.as_str(), "age");
                assert!(field2.pattern.is_none());
            }
            _ => panic!("Expected StructPattern"),
        }
    }

    #[test]
    fn test_parse_struct_pattern_with_nested_pattern() {
        let input = "User(username, address: Address(city, zip))";
        let result = parse_pattern_input(input, Rule::struct_pattern);

        match result {
            ast::Pattern::Struct(struct_pattern) => {
                assert_eq!(struct_pattern.ty.name.as_str(), "User");
                assert_eq!(struct_pattern.fields.len(), 2);

                // Check the first field
                let field1 = &struct_pattern.fields[0];
                assert_eq!(field1.identifier.as_str(), "username");
                assert!(field1.pattern.is_none());

                // Check the second field (nested pattern)
                let field2 = &struct_pattern.fields[1];
                assert_eq!(field2.identifier.as_str(), "address");
                if let Some(ast::Pattern::Struct(nested_struct)) = &field2.pattern {
                    assert_eq!(nested_struct.ty.name.as_str(), "Address");
                    assert_eq!(nested_struct.fields.len(), 2);

                    // Check the nested fields
                    assert_eq!(nested_struct.fields[0].identifier.as_str(), "city");
                    assert!(nested_struct.fields[0].pattern.is_none());
                    assert_eq!(nested_struct.fields[1].identifier.as_str(), "zip");
                    assert!(nested_struct.fields[1].pattern.is_none());
                } else {
                    panic!("Expected nested StructPattern");
                }
            }
            _ => panic!("Expected StructPattern"),
        }
    }

    #[test]
    fn test_parse_tuple_pattern() {
        let input = "(username, age)";
        let result = parse_pattern_input(input, Rule::pattern);

        match result {
            ast::Pattern::Tuple(tuple_pattern) => {
                assert_eq!(tuple_pattern.elements.len(), 2);

                // Check the first element
                if let ast::Pattern::Identifier(identifier) = &tuple_pattern.elements[0] {
                    assert_eq!(identifier.as_str(), "username");
                } else {
                    panic!("Expected IdentifierPattern");
                }

                // Check the second element
                if let ast::Pattern::Identifier(identifier) = &tuple_pattern.elements[1] {
                    assert_eq!(identifier.as_str(), "age");
                } else {
                    panic!("Expected IdentifierPattern");
                }
            }
            _ => panic!("Expected TuplePattern"),
        }
    }

    #[test]
    fn test_parse_nested_tuple_pattern() {
        let input = "(username, (city, zip))";
        let result = parse_pattern_input(input, Rule::pattern);

        match result {
            ast::Pattern::Tuple(tuple_pattern) => {
                assert_eq!(tuple_pattern.elements.len(), 2);

                // Check the first element
                if let ast::Pattern::Identifier(identifier) = &tuple_pattern.elements[0] {
                    assert_eq!(identifier.as_str(), "username");
                } else {
                    panic!("Expected IdentifierPattern");
                }

                // Check the second element (nested tuple)
                if let ast::Pattern::Tuple(nested_tuple) = &tuple_pattern.elements[1] {
                    assert_eq!(nested_tuple.elements.len(), 2);

                    // Check the nested elements
                    if let ast::Pattern::Identifier(identifier) = &nested_tuple.elements[0] {
                        assert_eq!(identifier.as_str(), "city");
                    } else {
                        panic!("Expected IdentifierPattern");
                    }

                    if let ast::Pattern::Identifier(identifier) = &nested_tuple.elements[1] {
                        assert_eq!(identifier.as_str(), "zip");
                    } else {
                        panic!("Expected IdentifierPattern");
                    }
                } else {
                    panic!("Expected nested TuplePattern");
                }
            }
            _ => panic!("Expected TuplePattern"),
        }
    }
}
