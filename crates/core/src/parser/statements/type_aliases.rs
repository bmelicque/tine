use pest::iterators::Pair;

use crate::{
    ast::{self, EnumDefinition},
    parser::{parser::Rule, utils::is_pascal_case},
};

use super::ParserEngine;

impl ParserEngine {
    pub fn parse_type_alias(&mut self, pair: Pair<'_, Rule>) -> ast::TypeAlias {
        assert!(pair.as_rule() == Rule::type_alias);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();

        let name = inner.next().unwrap().as_str().to_string();

        let mut params = None;
        let mut op = None;
        let mut definition = None;
        for pair in inner {
            match pair.as_rule() {
                Rule::type_params => {
                    params = Some(self.parse_type_params(pair));
                }
                Rule::type_def_op => op = Some(pair.as_str().to_string().into()),
                Rule::type_def => {
                    definition = Some(self.parse_type_definition(pair));
                }
                _ => unreachable!(),
            }
        }

        ast::TypeAlias {
            loc,
            name,
            params,
            op: op.unwrap(),
            definition: Box::new(definition.unwrap()),
        }
    }

    fn parse_type_params(&mut self, pair: Pair<'_, Rule>) -> Vec<String> {
        assert_eq!(pair.as_rule(), Rule::type_params);
        let mut type_params = Vec::new();
        let mut type_param_names = std::collections::HashSet::new();
        let inner = pair.into_inner();
        for param_pair in inner {
            let param_name = self.parse_type_param(&param_pair);
            if !type_param_names.insert(param_name.clone()) {
                let message = format!("Duplicate type parameter name '{}'", param_name);
                let loc = self.localize(param_pair.as_span());
                self.error(message, loc);
            }
            type_params.push(param_name);
        }
        type_params
    }

    fn parse_type_param(&mut self, pair: &Pair<'_, Rule>) -> String {
        assert_eq!(pair.as_rule(), Rule::type_identifier);
        let param_name = pair.as_str().to_string();
        if !is_pascal_case(&param_name) {
            let message = format!(
                "Type parameter name '{}' should be in Pascal case",
                param_name
            );
            let loc = self.localize(pair.as_span());
            self.error(message, loc);
        }
        param_name
    }

    fn parse_type_definition(&mut self, pair: Pair<'_, Rule>) -> ast::TypeDefinition {
        assert_eq!(pair.as_rule(), Rule::type_def);
        let pair = pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::struct_body => self.parse_struct_body(pair).into(),
            Rule::sum_type => self.parse_enum_definition(pair).into(),
            _ => unreachable!(),
        }
    }

    fn parse_struct_body(&mut self, pair: Pair<'_, Rule>) -> ast::StructDefinition {
        assert_eq!(pair.as_rule(), Rule::struct_body);
        let loc = self.localize(pair.as_span());

        let fields: Vec<ast::StructDefinitionField> = pair
            .into_inner()
            .map(|pair| self.parse_struct_field(pair))
            .collect();

        let mut field_names = std::collections::HashSet::new();
        fields.iter().for_each(|field| {
            if !field_names.insert(field.as_name()) {
                self.error(
                    format!("Duplicate field name '{}'", field.as_name()),
                    field.loc(),
                );
            }
        });

        ast::StructDefinition { loc, fields }
    }

    fn parse_struct_field(&mut self, pair: Pair<'_, Rule>) -> ast::StructDefinitionField {
        assert_eq!(pair.as_rule(), Rule::field_declaration);
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::mandatory_field => self.parse_mandatory_field(inner).into(),
            Rule::optional_field => self.parse_optionnal_field(inner).into(),
            _ => unreachable!(),
        }
    }

    fn parse_mandatory_field(&mut self, pair: Pair<'_, Rule>) -> ast::StructMandatoryField {
        assert_eq!(pair.as_rule(), Rule::mandatory_field);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();

        let name = inner.next().unwrap().as_str().to_string();
        let definition = self.parse_type(inner.next().unwrap());

        ast::StructMandatoryField {
            loc,
            name,
            definition,
        }
    }

    fn parse_optionnal_field(&mut self, pair: Pair<'_, Rule>) -> ast::StructOptionalField {
        assert_eq!(pair.as_rule(), Rule::mandatory_field);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();

        let name = inner.next().unwrap().as_str().to_string();
        let default = self.parse_expression(inner.next().unwrap());

        ast::StructOptionalField { loc, name, default }
    }

    pub fn parse_enum_definition(&mut self, pair: Pair<'_, Rule>) -> EnumDefinition {
        let loc = self.localize(pair.as_span());

        let variants: Vec<ast::VariantDefinition> = pair
            .into_inner()
            .map(|pair| self.parse_variant_definition(pair))
            .collect();

        let mut variant_names = std::collections::HashSet::new();
        for variant in variants.iter() {
            if !variant_names.insert(variant.as_name()) {
                self.error(
                    format!("Duplicate constructor name '{}'", variant.as_name()),
                    loc,
                );
            }
        }

        ast::EnumDefinition { loc, variants }
    }

    fn parse_variant_definition(&mut self, pair: Pair<'_, Rule>) -> ast::VariantDefinition {
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();

        let name = inner.next().unwrap().as_str().to_string();

        let Some(body) = inner.next() else {
            return ast::UnitVariant { loc, name }.into();
        };

        match body.as_rule() {
            // TODO: parse multiple elements
            Rule::sum_param => ast::TupleVariant {
                loc,
                name,
                elements: vec![self.parse_type(body)],
            }
            .into(),
            Rule::struct_body => ast::StructVariant {
                loc,
                name,
                def: self.parse_struct_body(body),
            }
            .into(),
            rule => unreachable!("Unexpected rule '{:?}' as variant body", rule),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parser::{TineParser, ParseError, Rule};
    use pest::Parser;

    fn parse_type_alias_input(
        input: &'static str,
        rule: Rule,
    ) -> (ast::TypeAlias, Vec<ParseError>) {
        let pair = TineParser::parse(rule, input)
            .unwrap()
            .next()
            .unwrap();
        let mut parser_engine = ParserEngine::new(0);
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
        let input = "MyStruct :: (field number, field string)";
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
