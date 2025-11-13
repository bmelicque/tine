use std::collections::HashMap;

use crate::{ast, type_checker::TypeChecker, types};

impl TypeChecker {
    pub fn visit_member_expression(&mut self, expr: &ast::MemberExpression) -> types::Type {
        let Some(ref member) = expr.prop else {
            self.visit_expression(&expr.object);
            // missing member already reported during parsing phase
            return types::Type::Unknown;
        };
        match member {
            ast::MemberProp::FieldName(_) => self.visit_field_access(expr),
            ast::MemberProp::Index(_) => self.visit_tuple_indexing(expr),
        }
    }

    fn visit_field_access(&mut self, expr: &ast::MemberExpression) -> types::Type {
        let root_type = self.visit_expression(&expr.object);
        let type_str = format!("{}", root_type.clone());
        let root_type = self.resolve_root_type(root_type);

        let Some(ast::MemberProp::FieldName(ref field_name)) = expr.prop else {
            unreachable!()
        };

        let prop = field_name.as_str();
        let types::Type::Struct(ty) = root_type else {
            self.error(
                format!("Property '{}' does not exist on type '{}'", prop, type_str),
                field_name.span,
            );
            return self.set_type_at(expr.span, types::Type::Unknown);
        };
        match ty.fields.iter().find(|field| field.name == prop) {
            Some(field) => self.set_type_at(expr.span, field.def.clone()),
            None => {
                self.error(
                    format!("Property '{}' does not exist on type '{}'", prop, type_str),
                    expr.span,
                );
                self.set_type_at(expr.span, types::Type::Unknown)
            }
        }
    }

    pub fn visit_tuple_indexing(&mut self, expr: &ast::MemberExpression) -> types::Type {
        let root_type = self.visit_expression(&expr.object);
        let types::Type::Tuple(tuple) = self.unwrap_named_type(&root_type) else {
            self.error(
                format!("Expected tuple type, got {}", root_type),
                expr.object.as_span(),
            );
            return self.set_type_at(expr.span, types::Type::Unknown);
        };

        let Some(ast::MemberProp::Index(index)) = &expr.prop else {
            panic!();
        };
        let value = index.value;
        if value != value.round() {
            self.error("Integer expected".into(), index.span);
            return self.set_type_at(expr.span, types::Type::Unknown);
        }
        let value = *value as isize;
        if value < 0 {
            self.error("Index out of range".into(), index.span);
            return self.set_type_at(expr.span, types::Type::Unknown);
        }
        let value = value as usize;
        if value >= tuple.elements.len() {
            self.error("Index out of range".into(), index.span);
            self.set_type_at(expr.span, types::Type::Unknown)
        } else {
            self.set_type_at(expr.span, tuple.elements[value].clone())
        }
    }

    fn resolve_root_type(&self, mut ty: types::Type) -> types::Type {
        while let types::Type::Named(ref named) = ty {
            let mut substitutions = HashMap::new();
            let params = self.type_registry.get_type_params(&named.name);
            for (i, param) in params.iter().enumerate() {
                let substitute = match named.args.get(i) {
                    Some(arg) => arg.clone(),
                    None => types::Type::Dynamic,
                };
                substitutions.insert(param, substitute);
            }
            let raw = self.type_registry.lookup(&named.name).unwrap();
            ty = substitute_type(&raw, &substitutions);
        }
        ty
    }
}

fn substitute_type(ty: &types::Type, substitutions: &HashMap<&String, types::Type>) -> types::Type {
    match ty {
        types::Type::Named(named) => {
            if let Some(substituted) = substitutions.get(&named.name) {
                substituted.clone()
            } else {
                // FIXME: substitute args
                // for arg in args {
                // }
                ty.clone()
            }
        }
        types::Type::Array(array) => {
            let element = Box::new(substitute_type(&array.element, substitutions));
            types::ArrayType { element }.into()
        }
        types::Type::Option(option) => {
            let some = Box::new(substitute_type(&option.some, substitutions));
            types::OptionType { some }.into()
        }
        types::Type::Map(map) => {
            let key = Box::new(substitute_type(&map.key, substitutions));
            let value = Box::new(substitute_type(&map.value, substitutions));
            types::MapType { key, value }.into()
        }
        types::Type::Struct(st) => {
            let fields = st
                .fields
                .iter()
                .map(|field| types::StructField {
                    name: field.name.clone(),
                    def: substitute_type(&field.def, substitutions),
                    optional: field.optional,
                })
                .collect();
            types::StructType { fields }.into()
        }
        _ => ty.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::{ast, types::*, VariableData};

    fn create_type_checker() -> TypeChecker {
        TypeChecker::new(Vec::new())
    }

    fn dummy_span() -> pest::Span<'static> {
        pest::Span::new("_", 0, 0).unwrap()
    }

    fn span(text: &'static str) -> pest::Span<'static> {
        pest::Span::new(text, 0, text.len()).unwrap()
    }

    #[test]
    fn test_visit_field_access_expression() {
        let mut checker = create_type_checker();
        checker.type_registry.define(
            "User",
            types::StructType {
                fields: vec![
                    StructField {
                        name: "name".to_string(),
                        def: types::Type::String,
                        optional: false,
                    },
                    StructField {
                        name: "age".to_string(),
                        def: types::Type::Number,
                        optional: false,
                    },
                ],
            }
            .into(),
            None,
        );

        let field_access_expression = ast::MemberExpression {
            object: Box::new(ast::Expression::Identifier(ast::Identifier {
                span: span("user"),
            })),
            prop: Some(ast::Identifier { span: span("name") }.into()),
            span: dummy_span(),
        };

        checker.analysis_context.register_symbol(VariableData::pure(
            "user".into(),
            Rc::new(types::Type::Named(types::NamedType {
                name: "User".to_string(),
                args: vec![],
            })),
            dummy_span(),
        ));

        let result = checker.visit_member_expression(&field_access_expression);
        assert!(
            checker.errors.is_empty(),
            "Expected no errors, got {:?}",
            checker.errors
        );
        assert_eq!(result, types::Type::String);
    }

    #[test]
    fn test_visit_tuple_indexing_valid() {
        let mut checker = create_type_checker();
        let tuple_type = types::Type::Tuple(types::TupleType {
            elements: vec![
                types::Type::Number,
                types::Type::String,
                types::Type::Boolean,
            ],
        });

        checker.analysis_context.register_symbol(VariableData::pure(
            "my_tuple".into(),
            tuple_type.clone().into(),
            span("my_tuple"),
        ));

        let tuple_indexing = ast::MemberExpression {
            object: Box::new(ast::Expression::Identifier(ast::Identifier {
                span: span("my_tuple"),
            })),
            prop: Some(ast::MemberProp::Index(ast::NumberLiteral {
                value: ordered_float::OrderedFloat(1.0),
                span: dummy_span(),
            })),
            span: dummy_span(),
        };

        let result = checker.visit_tuple_indexing(&tuple_indexing);
        assert_eq!(result, types::Type::String);
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_tuple_indexing_invalid_type() {
        let mut checker = create_type_checker();
        checker.analysis_context.register_symbol(VariableData::pure(
            "not_a_tuple".into(),
            types::Type::Number.into(),
            span("not_a_tuple"),
        ));

        let tuple_indexing = ast::MemberExpression {
            object: Box::new(ast::Expression::Identifier(ast::Identifier {
                span: span("not_a_tuple"),
            })),
            prop: Some(ast::MemberProp::Index(ast::NumberLiteral {
                value: ordered_float::OrderedFloat(0.0),
                span: dummy_span(),
            })),
            span: dummy_span(),
        };

        let result = checker.visit_tuple_indexing(&tuple_indexing);
        assert_eq!(result, types::Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0]
            .message
            .contains("Expected tuple type, got number"));
    }

    #[test]
    fn test_visit_tuple_indexing_out_of_range() {
        let mut checker = create_type_checker();
        let tuple_type = types::Type::Tuple(types::TupleType {
            elements: vec![Type::Number, types::Type::String],
        });

        checker.analysis_context.register_symbol(VariableData::pure(
            "my_tuple".into(),
            tuple_type.clone().into(),
            span("my_tuple"),
        ));

        let tuple_indexing = ast::MemberExpression {
            object: Box::new(ast::Expression::Identifier(ast::Identifier {
                span: span("my_tuple"),
            })),
            prop: Some(ast::MemberProp::Index(ast::NumberLiteral {
                value: ordered_float::OrderedFloat(2.0),
                span: dummy_span(),
            })),
            span: dummy_span(),
        };

        let result = checker.visit_tuple_indexing(&tuple_indexing);
        assert_eq!(result, types::Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Index out of range"));
    }

    #[test]
    fn test_visit_tuple_indexing_negative_index() {
        let mut checker = create_type_checker();
        let tuple_type = types::Type::Tuple(types::TupleType {
            elements: vec![Type::Number, types::Type::String],
        });

        checker.analysis_context.register_symbol(VariableData::pure(
            "my_tuple".into(),
            tuple_type.clone().into(),
            span("my_tuple"),
        ));

        let tuple_indexing = ast::MemberExpression {
            object: Box::new(ast::Expression::Identifier(ast::Identifier {
                span: span("my_tuple"),
            })),
            prop: Some(ast::MemberProp::Index(ast::NumberLiteral {
                value: ordered_float::OrderedFloat(-1.0),
                span: dummy_span(),
            })),
            span: dummy_span(),
        };

        let result = checker.visit_tuple_indexing(&tuple_indexing);
        assert_eq!(result, types::Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].message.contains("Index out of range"));
    }
}
