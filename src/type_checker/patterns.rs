use crate::{ast, parser::parser::ParseError, types::Type};

use super::TypeChecker;

impl TypeChecker {
    pub fn match_pattern(
        &mut self,
        pattern: &ast::Pattern,
        against: Type,
        variables: &mut Vec<(String, Type)>,
    ) {
        match pattern {
            ast::Pattern::Identifier(id) => variables.push((id.span.as_str().into(), against)),
            ast::Pattern::StructPattern(pattern) => {
                self.match_struct_pattern(pattern, against, variables)
            }
            ast::Pattern::Tuple(pattern) => self.match_tuple_pattern(pattern, against, variables),
        }
    }

    /// `src` is the type against which the pattern will be compared
    pub fn match_struct_pattern(
        &mut self,
        pattern: &ast::StructPattern,
        against: Type,
        variables: &mut Vec<(String, Type)>,
    ) {
        let against_name = match against {
            Type::Named { ref name, .. } if *name == pattern.ty.name => name,
            _ => {
                self.errors.push(ParseError {
                    message: format!(
                        "Type '{}' does not match type '{}'",
                        pattern.ty.name, against
                    ),
                    span: pattern.span,
                });
                return;
            }
        };

        let Type::Struct {
            fields: against_fields,
        } = self.unwrap_named_type(&against)
        else {
            self.errors.push(ParseError {
                message: "Expected structured type".into(),
                span: pattern.span,
            });
            return;
        };

        for field in pattern.fields.iter() {
            let Some(against) = against_fields.iter().find(|f| f.name == field.identifier) else {
                self.errors.push(ParseError {
                    message: format!(
                        "Property '{}' does not exist on type '{}'",
                        field.identifier, against_name
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
        against: Type,
        variables: &mut Vec<(String, Type)>,
    ) {
        let Type::Tuple(elements) = self.unwrap_named_type(&against) else {
            self.errors.push(ParseError {
                message: "Expected tuple type".into(),
                span: pattern.span,
            });
            return;
        };

        if pattern.elements.len() != elements.len() {
            self.errors.push(ParseError {
                message: format!(
                    "Expected {} elements, got {}",
                    elements.len(),
                    pattern.elements.len()
                ),
                span: pattern.span,
            });
        }

        for (index, pattern) in pattern.elements.iter().enumerate() {
            let against = elements.get(index).unwrap_or(&Type::Unknown);
            self.match_pattern(pattern, against.clone(), variables);
        }
    }
}
