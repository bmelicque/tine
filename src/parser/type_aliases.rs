use pest::iterators::Pair;

use crate::ast::{self, EnumDefinition};

use super::{
    parser::{ParseError, Rule},
    utils::{is_camel_case, is_pascal_case},
    ParserEngine,
};

impl ParserEngine {
    pub fn parse_type_alias(&mut self, pair: Pair<'static, Rule>) -> ast::TypeAlias {
        assert!(pair.as_rule() == Rule::type_alias);
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let name = inner.next().unwrap().as_str().to_string();

        let mut params = None;
        let mut definition = None;
        while let Some(pair) = inner.next() {
            match pair.as_rule() {
                Rule::type_params => {
                    params = Some(self.parse_type_params(pair));
                }
                Rule::type_def => {
                    definition = Some(self.parse_type_definition(pair));
                }
                _ => unreachable!(),
            }
        }

        ast::TypeAlias {
            span,
            name,
            params,
            definition: Box::new(definition.unwrap()),
        }
    }

    fn parse_type_params(&mut self, pair: Pair<'static, Rule>) -> Vec<String> {
        assert_eq!(pair.as_rule(), Rule::type_params);
        let mut type_params = Vec::new();
        let mut type_param_names = std::collections::HashSet::new();
        let inner = pair.into_inner();
        for param_pair in inner {
            assert_eq!(param_pair.as_rule(), Rule::type_identifier);
            let param_name = param_pair.as_str().to_string();
            if !is_pascal_case(&param_name) {
                self.errors.push(ParseError {
                    message: format!(
                        "Type parameter name '{}' should be in Pascal case",
                        param_name
                    ),
                    span: param_pair.as_span(),
                });
            }
            if !type_param_names.insert(param_name.clone()) {
                self.errors.push(ParseError {
                    message: format!("Duplicate type parameter name '{}'", param_name),
                    span: param_pair.as_span(),
                });
            }
            type_params.push(param_name);
        }
        type_params
    }

    fn parse_type_definition(&mut self, pair: Pair<'static, Rule>) -> ast::TypeDefinition {
        assert_eq!(pair.as_rule(), Rule::type_def);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::struct_body => self.parse_struct_body(pair).into(),
            Rule::sum_type => self.parse_enum_definition(pair).into(),
            Rule::trait_type => self.parse_trait_definition(pair).into(),
            _ => unreachable!(),
        }
    }

    fn parse_struct_body(&mut self, pair: Pair<'static, Rule>) -> ast::StructDefinition {
        let span = pair.as_span();
        let mut fields = Vec::new();
        let mut field_names = std::collections::HashSet::new();

        for field_pair in pair.into_inner() {
            let field = self.parse_struct_field(field_pair.into_inner().next().unwrap());

            // Check for duplicate field names
            if !field_names.insert(field.as_name()) {
                self.errors.push(ParseError {
                    message: format!("Duplicate field name '{}'", field.as_name()),
                    span: field.as_span(),
                });
            }

            fields.push(field);
        }

        ast::StructDefinition { span, fields }
    }

    fn parse_struct_field(&mut self, pair: Pair<'static, Rule>) -> ast::StructDefinitionField {
        let span = pair.as_span();
        let mut field_inner = pair.clone().into_inner();

        let name = field_inner.next().unwrap().as_str().to_string();
        if !is_camel_case(&name) {
            self.errors.push(ParseError {
                message: format!("Field name '{}' should be in camelCase", name),
                span: pair.as_span(),
            });
        }

        let next = field_inner.next().unwrap();

        match pair.as_rule() {
            Rule::mandatory_field => {
                assert!(next.as_rule() == Rule::type_element);
                ast::StructMandatoryField {
                    span,
                    name,
                    definition: self.parse_type(next.into_inner().next().unwrap()),
                }
                .into()
            }
            Rule::optional_field => ast::StructOptionalField {
                span,
                name,
                default: self.parse_expression(next),
            }
            .into(),
            _ => unreachable!(),
        }
    }

    pub fn parse_enum_definition(&mut self, pair: Pair<'static, Rule>) -> EnumDefinition {
        let span = pair.as_span();

        let variants: Vec<ast::VariantDefinition> = pair
            .into_inner()
            .map(|pair| self.parse_variant_definition(pair))
            .collect();

        let mut variant_names = std::collections::HashSet::new();
        for variant in variants.iter() {
            if !variant_names.insert(variant.as_name()) {
                self.errors.push(ParseError {
                    message: format!("Duplicate constructor name '{}'", variant.as_name()),
                    span,
                });
            }
        }

        ast::EnumDefinition { span, variants }
    }

    fn parse_variant_definition(&mut self, pair: Pair<'static, Rule>) -> ast::VariantDefinition {
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let name = inner.next().unwrap().as_str().to_string();

        let Some(body) = inner.next() else {
            return ast::UnitVariant { span, name }.into();
        };

        match body.as_rule() {
            // TODO: parse multiple elements
            Rule::sum_param => ast::TupleVariant {
                span,
                name,
                elements: vec![self.parse_type(body)],
            }
            .into(),
            Rule::struct_body => ast::StructVariant {
                span,
                name,
                def: self.parse_struct_body(body),
            }
            .into(),
            rule => unreachable!("Unexpected rule '{:?}' as variant body", rule),
        }
    }

    fn parse_trait_definition(&mut self, pair: Pair<'static, Rule>) -> ast::TraitDefinition {
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let name_pair = inner.next().unwrap(); // Should be the identifier inside `()`
        let name = name_pair.as_str().to_string();

        if !is_pascal_case(&name) {
            self.errors.push(ParseError {
                message: format!("Trait name '{}' should be in PascalCase", name),
                span: name_pair.as_span(),
            });
        }

        let body_pair = inner.next().unwrap(); // Should be the struct_body after the dot
        let body = Box::new(self.parse_struct_body(body_pair));

        ast::TraitDefinition { span, name, body }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{MyLanguageParser, Rule};
    use pest::Parser;

    fn parse_type_alias_input(
        input: &'static str,
        rule: Rule,
    ) -> (ast::TypeAlias, Vec<ParseError>) {
        let pair = MyLanguageParser::parse(rule, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new();
        (parser_engine.parse_type_alias(pair), parser_engine.errors)
    }

    // TODO: implement simple aliasing
    // #[test]
    // fn test_parse_simple_type_alias() {
    //     let input = "MyAlias :: number";
    //     let (result, _) = parse_type_alias_input(input, Rule::type_alias);

    //     assert_eq!(result.name, "MyAlias");
    //     assert!(result.params.is_none());
    //     match *result.definition {
    //         ast::TypeDefinition::Struct(def) => {
    //             assert!(def.fields.is_empty());
    //         }
    //         _ => panic!("Expected StructDefinition"),
    //     }
    // }

    #[test]
    fn test_parse_generic_type_alias() {
        let input = "Box[T] :: (value T)";
        let (result, _) = parse_type_alias_input(input, Rule::type_alias);

        assert_eq!(result.name, "Box");
        assert!(result.params.is_some());
        let params = result.params.unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "T");

        match *result.definition {
            ast::TypeDefinition::Struct(def) => {
                assert_eq!(def.fields.len(), 1);
                let field = &def.fields[0];
                assert_eq!(field.as_name(), "value");
            }
            _ => panic!("Expected StructDefinition"),
        }
    }

    #[test]
    fn test_parse_sum_type_alias() {
        let input = "Shape :: | Circle(radius number) | Rectangle(width number, height number)";
        let (result, _) = parse_type_alias_input(input, Rule::type_alias);

        assert_eq!(result.name, "Shape");
        assert!(result.params.is_none());

        match *result.definition {
            ast::TypeDefinition::Enum(sum) => {
                assert_eq!(sum.variants.len(), 2);

                // Check the first variant
                let variant1 = &sum.variants[0];
                assert_eq!(variant1.as_name(), "Circle");

                // Check the second variant
                let variant2 = &sum.variants[1];
                assert_eq!(variant2.as_name(), "Rectangle");
            }
            _ => panic!("Expected SumType"),
        }
    }

    #[test]
    fn test_parse_trait_type_alias() {
        let input = "MyTrait :: (Self).(method() -> Self)";
        let (result, _) = parse_type_alias_input(input, Rule::type_alias);

        assert_eq!(result.name, "MyTrait");
        assert!(result.params.is_none());

        match *result.definition {
            ast::TypeDefinition::Trait(trait_def) => {
                assert_eq!(trait_def.name, "Self");
                assert_eq!(trait_def.body.fields.len(), 1);

                let field = &trait_def.body.fields[0];
                assert_eq!(field.as_name(), "method");
            }
            _ => panic!("Expected TraitDefinition"),
        }
    }

    #[test]
    fn test_parse_type_alias_with_duplicate_params() {
        let input = "Box[T, T] :: (value T)";
        let (result, errors) = parse_type_alias_input(input, Rule::type_alias);

        assert_eq!(result.name, "Box");
        assert!(result.params.is_some());
        let params = result.params.unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], "T");
        assert_eq!(params[1], "T");

        // Check for errors
        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.message.contains("Duplicate type parameter name")));
    }

    #[test]
    fn test_parse_struct_with_duplicate_fields() {
        let input = "MyStruct :: (field: number, field: string)";
        let (result, errors) = parse_type_alias_input(input, Rule::type_alias);

        assert_eq!(result.name, "MyStruct");
        assert!(result.params.is_none());

        match *result.definition {
            ast::TypeDefinition::Struct(def) => {
                assert_eq!(def.fields.len(), 2);

                // Check for errors
                assert!(!errors.is_empty());
                assert!(errors
                    .iter()
                    .any(|e| e.message.contains("Duplicate field name")));
            }
            _ => panic!("Expected StructDefinition"),
        }
    }
}
