use std::collections::HashMap;

use crate::{
    ast,
    parser::parser::ParseError,
    types::{StructField, Type},
};

use super::{scopes::VariableInfo, TypeChecker};

impl TypeChecker {
    pub fn visit_expression(&mut self, node: &ast::Expression) -> Type {
        match node {
            ast::Expression::Binary(node) => self.visit_binary_expression(node),
            ast::Expression::BooleanLiteral(_) => Type::Boolean,
            ast::Expression::CompositeLiteral(node) => self.visit_composite_literal(node),
            ast::Expression::Empty => Type::Void,
            ast::Expression::FieldAccess(node) => self.visit_field_access_expression(node),
            ast::Expression::Function(node) => self.visit_function_expression(node),
            ast::Expression::Identifier(node) => self.visit_identifier(node),
            ast::Expression::NumberLiteral(_) => Type::Number,
            ast::Expression::StringLiteral(_) => Type::String,
            ast::Expression::TupleIndexing(_) => {
                // FIXME:
                Type::Unknown
            }
        }
    }

    pub fn visit_expression_or_anonymous(&mut self, node: &ast::ExpressionOrAnonymous) -> Type {
        match node {
            ast::ExpressionOrAnonymous::Array(node) => self.visit_anonymous_array_literal(node),
            ast::ExpressionOrAnonymous::Expression(node) => self.visit_expression(node),
            ast::ExpressionOrAnonymous::Struct(node) => self.visit_anonymous_struct_literal(node),
        }
    }

    fn visit_binary_expression(&mut self, node: &ast::BinaryExpression) -> Type {
        let left_type = self.visit_expression(&node.left);
        let right_type = self.visit_expression(&node.right);

        let mut push_error = |ty: Type| {
            self.errors.push(ParseError {
                message: format!(
                    "Operator '{}' cannot be applied to type {:?}",
                    node.operator, ty
                ),
                span: node.span,
            })
        };

        match node.operator {
            ast::BinaryOperator::Add
            | ast::BinaryOperator::Sub
            | ast::BinaryOperator::Mul
            | ast::BinaryOperator::Div
            | ast::BinaryOperator::Mod
            | ast::BinaryOperator::Pow
            | ast::BinaryOperator::Geq
            | ast::BinaryOperator::Grt
            | ast::BinaryOperator::Leq
            | ast::BinaryOperator::Less => {
                if !matches!(left_type, Type::Unknown | Type::Number) {
                    push_error(left_type);
                };
                if !matches!(right_type, Type::Unknown | Type::Number) {
                    push_error(right_type);
                };
            }
            ast::BinaryOperator::Eq | ast::BinaryOperator::Neq => {
                if left_type != right_type
                    && left_type != Type::Unknown
                    && right_type != Type::Unknown
                {
                    self.errors.push(ParseError {
                        message: format!(
                            "Types {:?} and {:?} cannot be compared",
                            left_type, right_type
                        ),
                        span: node.span,
                    });
                    return Type::Unknown;
                }
            }
            ast::BinaryOperator::LAnd | ast::BinaryOperator::LOr => {
                if !matches!(left_type, Type::Unknown | Type::Boolean) {
                    push_error(left_type);
                };
                if !matches!(right_type, Type::Unknown | Type::Boolean) {
                    push_error(right_type);
                };
            }
        };

        match node.operator {
            ast::BinaryOperator::Add
            | ast::BinaryOperator::Sub
            | ast::BinaryOperator::Mul
            | ast::BinaryOperator::Div
            | ast::BinaryOperator::Mod
            | ast::BinaryOperator::Pow => Type::Number,
            ast::BinaryOperator::Eq
            | ast::BinaryOperator::Geq
            | ast::BinaryOperator::Grt
            | ast::BinaryOperator::LAnd
            | ast::BinaryOperator::Leq
            | ast::BinaryOperator::Less
            | ast::BinaryOperator::LOr
            | ast::BinaryOperator::Neq => Type::Boolean,
        }
    }

    fn visit_field_access_expression(&mut self, node: &ast::FieldAccessExpression) -> Type {
        let mut ty = self.visit_expression(&node.object);
        let type_str = format!("{}", ty.clone());
        while let Type::Named { ref name, args } = ty {
            let mut substitutions = HashMap::new();
            let params = self.type_registry.get_type_params(name);
            for (i, param) in params.iter().enumerate() {
                let substitute = match args.get(i) {
                    Some(arg) => arg.clone(),
                    None => Type::Dynamic,
                };
                substitutions.insert(param, substitute);
            }
            let raw = self.type_registry.lookup(name).unwrap();
            ty = substitute_type(&raw, &substitutions);
        }

        let prop = node.prop.as_str();
        match ty {
            Type::Struct { fields } => match fields.iter().find(|field| field.name == prop) {
                Some(field) => field.def.clone(),
                None => {
                    self.errors.push(ParseError {
                        message: format!(
                            "Property '{}' does not exist on type '{}'",
                            prop, type_str
                        ),
                        span: node.span,
                    });
                    Type::Unknown
                }
            },
            _ => {
                self.errors.push(ParseError {
                    message: format!("Property '{}' does not exist on type '{}'", prop, type_str),
                    span: node.span,
                });
                Type::Unknown
            }
        }
    }

    fn visit_function_expression(&mut self, node: &ast::FunctionExpression) -> Type {
        self.symbols.enter_scope();

        let mut param_types = Vec::new();
        for param in node.params.iter() {
            let ty = self.resolve_type(&param.ty);
            self.symbols.define(param.name.as_str(), ty.clone(), false);
            param_types.push(ty);
        }
        let body_type = self.visit_function_body(&node.body);

        self.symbols.exit_scope();

        Type::Function {
            params: param_types,
            return_type: Box::new(body_type),
        }
    }

    fn visit_function_body(&mut self, node: &ast::FunctionBody) -> Type {
        let block = match node {
            ast::FunctionBody::Expression(node) => return self.visit_expression(node),
            ast::FunctionBody::TypedBlock(node) => node,
        };

        let ty = self.visit_type(&block.ty);
        self.visit_block_statement(&block.block);
        ty
    }

    fn visit_identifier(&mut self, node: &ast::Identifier) -> Type {
        match self.symbols.lookup(node.as_str()) {
            Some(VariableInfo { ty, .. }) => ty.clone(),
            None => {
                self.errors.push(ParseError {
                    message: format!("Undefined variable: {}", node.as_str()),
                    span: node.span,
                });
                Type::Unknown
            }
        }
    }

    // pub fn visit_tuple_indexing(&mut self, node: &AstNode) -> Type {
    //     // TODO: visit left
    //     // TODO: resolve left type to check if it is a tuple
    //     // TODO: check if number literal is in range
    //     // TODO: return type à tuple's index
    // }
}

fn substitute_type(ty: &Type, substitutions: &HashMap<&String, Type>) -> Type {
    match ty {
        // If the type is a named type, substitute it if it's in the map
        Type::Named { name, args: _ } => {
            if let Some(substituted) = substitutions.get(name) {
                substituted.clone()
            } else {
                // FIXME: substitute args
                // for arg in args {
                // }
                ty.clone()
            }
        }

        // Handle arrays recursively
        Type::Array(inner) => Type::Array(Box::new(substitute_type(inner, substitutions))),

        // Handle options recursively
        Type::Option(inner) => Type::Option(Box::new(substitute_type(inner, substitutions))),

        // Handle maps recursively
        Type::Map { key, value } => Type::Map {
            key: Box::new(substitute_type(key, substitutions)),
            value: Box::new(substitute_type(value, substitutions)),
        },

        // Handle structs recursively
        Type::Struct { fields } => Type::Struct {
            fields: fields
                .iter()
                .map(|field| StructField {
                    name: field.name.clone(),
                    def: substitute_type(&field.def, substitutions),
                    optional: field.optional,
                })
                .collect(),
        },

        // Handle other types (e.g., primitives) as is
        _ => ty.clone(),
    }
}
