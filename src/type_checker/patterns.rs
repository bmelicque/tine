use crate::{ast, parser::parser::ParseError, types};

use super::TypeChecker;

impl TypeChecker {
    pub fn match_pattern(
        &mut self,
        pattern: &ast::Pattern,
        against: types::Type,
        variables: &mut Vec<(String, types::Type)>,
    ) {
        match pattern {
            ast::Pattern::Identifier(id) => variables.push((id.span.as_str().into(), against)),
            ast::Pattern::Literal(l) => self.match_literal_pattern(l, against),
            ast::Pattern::Struct(pattern) => self.match_struct_pattern(pattern, against, variables),
            ast::Pattern::Tuple(pattern) => self.match_tuple_pattern(pattern, against, variables),
            ast::Pattern::Variant(pattern) => {
                self.match_variant_pattern(pattern, against, variables)
            }
        }
    }

    fn match_literal_pattern(&mut self, pattern: &ast::LiteralPattern, against: types::Type) {
        let got = match pattern {
            ast::LiteralPattern::Boolean(_) => types::Type::Boolean,
            ast::LiteralPattern::Number(_) => types::Type::Number,
            ast::LiteralPattern::String(_) => types::Type::String,
        };
        if against != types::Type::Unknown && against != got {
            self.errors.push(ParseError {
                message: format!("Cannot match {} literal against {}", got, against),
                span: pattern.as_span(),
            });
        }
    }

    /// `src` is the type against which the pattern will be compared
    pub fn match_struct_pattern(
        &mut self,
        pattern: &ast::StructPattern,
        against: types::Type,
        variables: &mut Vec<(String, types::Type)>,
    ) {
        let Some(against_name) = self.against_name(&pattern.ty, &against) else {
            return;
        };

        let types::Type::Struct(st) = self.unwrap_named_type(&against) else {
            self.errors.push(ParseError {
                message: "Expected structured type".into(),
                span: pattern.span,
            });
            return;
        };

        self.match_struct_pattern_fields(&pattern.fields, &st.fields, against_name, variables);
    }

    fn match_struct_pattern_fields(
        &mut self,
        pattern_fields: &Vec<ast::StructPatternField>,
        against_fields: &Vec<types::StructField>,
        type_name: String,
        variables: &mut Vec<(String, types::Type)>,
    ) {
        for field in pattern_fields.iter() {
            let Some(against) = against_fields.iter().find(|f| f.name == field.identifier) else {
                self.errors.push(ParseError {
                    message: format!(
                        "Property '{}' does not exist on type '{}'",
                        field.identifier, type_name
                    ),
                    span: field.span,
                });
                continue;
            };
            match field.pattern {
                Some(ref sub_pattern) => {
                    self.match_pattern(sub_pattern, against.def.clone(), variables)
                }
                None => variables.push((field.identifier.clone(), against.def.clone())),
            }
        }
    }

    pub fn match_tuple_pattern(
        &mut self,
        pattern: &ast::TuplePattern,
        against: types::Type,
        variables: &mut Vec<(String, types::Type)>,
    ) {
        let types::Type::Tuple(ty) = self.unwrap_named_type(&against) else {
            self.errors.push(ParseError {
                message: "Expected tuple type".into(),
                span: pattern.span,
            });
            return;
        };

        if pattern.elements.len() != ty.elements.len() {
            self.errors.push(ParseError {
                message: format!(
                    "Expected {} elements, got {}",
                    ty.elements.len(),
                    pattern.elements.len()
                ),
                span: pattern.span,
            });
        }

        for (index, pattern) in pattern.elements.iter().enumerate() {
            let against = ty.elements.get(index).unwrap_or(&types::Type::Unknown);
            self.match_pattern(pattern, against.clone(), variables);
        }
    }

    fn match_variant_pattern(
        &mut self,
        pattern: &ast::VariantPattern,
        against: types::Type,
        variables: &mut Vec<(String, types::Type)>,
    ) {
        let Some(against_name) = self.against_name(&pattern.ty, &against) else {
            return;
        };

        let types::Type::Enum(ty) = self.unwrap_named_type(&against) else {
            self.errors.push(ParseError {
                message: format!("Cannot match a variant against type {}", against),
                span: pattern.span,
            });
            return;
        };

        let Some(variant) = ty.variants.iter().find(|var| var.name == pattern.name) else {
            self.errors.push(ParseError {
                message: format!(
                    "Variant '{}' does not exist on type {}",
                    pattern.name, against_name
                ),
                span: pattern.span,
            });
            return;
        };

        match &variant.def {
            types::Type::Struct(ty) => self.match_struct_variant(pattern, &ty.fields, variables),
            types::Type::Tuple(def) => self.match_tuple_variant(pattern, &def.elements, variables),
            types::Type::Unit => self.match_unit_variant(pattern),
            ty => unreachable!("Unexpected type {}", ty),
        }
    }

    fn match_struct_variant(
        &mut self,
        pattern: &ast::VariantPattern,
        fields: &Vec<types::StructField>,
        variables: &mut Vec<(String, types::Type)>,
    ) {
        let Some(ast::VariantPatternBody::Struct(body)) = &pattern.body else {
            self.errors.push(ParseError {
                message: "Tuple variant expected".to_string(),
                span: pattern.span,
            });
            return;
        };
        self.match_struct_pattern_fields(
            body,
            fields,
            format!("{}.{}", pattern.ty.name, pattern.name),
            variables,
        );
    }

    fn match_tuple_variant(
        &mut self,
        pattern: &ast::VariantPattern,
        def: &Vec<types::Type>,
        variables: &mut Vec<(String, types::Type)>,
    ) {
        let Some(ast::VariantPatternBody::Tuple(ref body)) = pattern.body else {
            self.errors.push(ParseError {
                message: "Tuple variant expected".to_string(),
                span: pattern.span,
            });
            return;
        };
        self.match_tuple_pattern(
            body,
            types::Type::Tuple(types::TupleType {
                elements: def.clone(),
            })
            .into(),
            variables,
        );
    }

    fn match_unit_variant(&mut self, pattern: &ast::VariantPattern) {
        if pattern.body.is_some() {
            self.errors.push(ParseError {
                message: "No body expected for unit variant".to_string(),
                span: pattern.span,
            });
        }
    }

    fn against_name(&mut self, ty: &ast::NamedType, against: &types::Type) -> Option<String> {
        match against {
            types::Type::Named(ref named) if *named.name == ty.name => Some(named.name.clone()),
            _ => {
                self.errors.push(ParseError {
                    message: format!("Type '{}' does not match type '{}'", ty.name, against),
                    span: ty.span,
                });
                None
            }
        }
    }
}
