use pest::Span;

use crate::{
    ast,
    type_checker::analysis_context::type_store::TypeStore,
    types::{self, Type, TypeId},
};

use super::TypeChecker;

#[derive(Debug, Clone)]
pub struct TokenList(pub Vec<(Span<'static>, TypeId)>);
impl TokenList {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, span: Span<'static>, ty: TypeId) {
        self.0.push((span, ty));
    }
}

impl TypeChecker {
    pub fn match_pattern(
        &mut self,
        pattern: &ast::Pattern,
        against: TypeId,
        variables: &mut TokenList,
    ) {
        match pattern {
            ast::Pattern::Identifier(id) => variables.push(id.span, against),
            ast::Pattern::Literal(l) => self.match_literal_pattern(l, against),
            ast::Pattern::Struct(pattern) => self.match_struct_pattern(pattern, against, variables),
            ast::Pattern::Tuple(pattern) => self.match_tuple_pattern(pattern, against, variables),
            ast::Pattern::Variant(pattern) => {
                self.match_variant_pattern(pattern, against, variables)
            }
        }
    }

    fn match_literal_pattern(&mut self, pattern: &ast::LiteralPattern, against: TypeId) {
        let against = self.analysis_context.type_store.get(against);
        let got = match pattern {
            ast::LiteralPattern::Boolean(_) => types::Type::Boolean,
            ast::LiteralPattern::Number(_) => types::Type::Number,
            ast::LiteralPattern::String(_) => types::Type::String,
        };
        if *against != types::Type::Unknown && *against != got {
            self.error(
                format!("Cannot match {} literal against {}", got, *against),
                pattern.as_span(),
            );
        }
    }

    /// Try to match the given struct pattern against the given type id.
    ///
    /// For example, matching a pattern like `User(name, age)` against a value of type `User`.
    /// This will regiter errors:
    /// - if the matched type is not a struct
    /// - if some
    pub fn match_struct_pattern(
        &mut self,
        pattern: &ast::StructPattern,
        against: TypeId,
        variables: &mut TokenList,
    ) {
        let Some(ty) = self.analysis_context.lookup(&pattern.ty.name) else {
            self.error(
                format!("cannot find type '{}'", &pattern.ty.name),
                pattern.ty.span,
            );
            return;
        };
        let Type::Struct(pattern_type) = self.resolve(ty.borrow().ty).clone() else {
            self.error(
                format!("type '{}' is not a structured type", &pattern.ty.name),
                pattern.span,
            );
            return;
        };

        let ty = match self.resolve(against) {
            Type::Struct(st) if st.id == pattern_type.id => st.clone(),
            _ => {
                self.error("pattern doesn't match expected type".into(), pattern.span);
                return;
            }
        };

        self.match_struct_pattern_fields(&pattern.fields, &ty.fields, &pattern.ty.name, variables);
    }

    fn match_struct_pattern_fields(
        &mut self,
        pattern_fields: &Vec<ast::StructPatternField>,
        against_fields: &Vec<types::StructField>,
        type_name: &str,
        variables: &mut TokenList,
    ) {
        for field in pattern_fields.iter() {
            let Some(against) = against_fields
                .iter()
                .find(|f| f.name == field.identifier.as_str())
            else {
                let error = format!(
                    "Property '{}' does not exist on type '{}'",
                    field.identifier.as_str(),
                    type_name
                );
                self.error(error, field.span);
                continue;
            };
            match field.pattern {
                Some(ref sub_pattern) => {
                    self.match_pattern(sub_pattern, against.def.clone(), variables)
                }
                None => variables.0.push((field.identifier, against.def.clone())),
            }
        }
    }

    pub fn match_tuple_pattern(
        &mut self,
        pattern: &ast::TuplePattern,
        against: TypeId,
        variables: &mut TokenList,
    ) {
        let types::Type::Tuple(ty) = self.resolve(against).clone() else {
            self.error("Expected tuple type".into(), pattern.span);
            return;
        };

        if pattern.elements.len() != ty.elements.len() {
            self.error(
                format!(
                    "Expected {} elements, got {}",
                    ty.elements.len(),
                    pattern.elements.len()
                ),
                pattern.span,
            );
        }

        for (index, pattern) in pattern.elements.iter().enumerate() {
            let against = ty
                .elements
                .get(index)
                .unwrap_or(&TypeStore::UNKNOWN)
                .clone();
            self.match_pattern(pattern, against, variables);
        }
    }

    /// Try to match the given variant pattern against the given type.
    ///
    /// For example in statements like `Role.Admin(acl) := role`
    fn match_variant_pattern(
        &mut self,
        pattern: &ast::VariantPattern,
        against: TypeId,
        variables: &mut TokenList,
    ) {
        let Some(ty) = self.analysis_context.lookup(&pattern.ty.name) else {
            self.error(
                format!("cannot find type '{}'", &pattern.ty.name),
                pattern.ty.span,
            );
            return;
        };
        let Type::Enum(pattern_type) = self.resolve(ty.borrow().ty).clone() else {
            self.error(
                format!("type '{}' is not an enum", &pattern.ty.name),
                pattern.span,
            );
            return;
        };

        let ty = match self.resolve(against) {
            Type::Enum(e) if e.id == pattern_type.id => e.clone(),
            _ => {
                self.error("pattern doesn't match expected type".into(), pattern.span);
                return;
            }
        };

        let Some(variant) = ty.variants.iter().find(|var| var.name == pattern.name) else {
            self.error(
                format!(
                    "Variant '{}' does not exist on type {}",
                    pattern.name, pattern.ty.name,
                ),
                pattern.span,
            );
            return;
        };

        match self.resolve(variant.def).clone() {
            Type::Struct(ty) => self.match_struct_variant(pattern, &ty.fields, variables),
            Type::Tuple(def) => {
                let id = self
                    .analysis_context
                    .type_store
                    .find_id(&def.into())
                    .unwrap();
                self.match_tuple_variant(pattern, id, variables)
            }
            Type::Unit => self.match_unit_variant(pattern),
            ty => unreachable!("Unexpected type {}", ty),
        }
    }

    fn match_struct_variant(
        &mut self,
        pattern: &ast::VariantPattern,
        fields: &Vec<types::StructField>,
        variables: &mut TokenList,
    ) {
        let Some(ast::VariantPatternBody::Struct(body)) = &pattern.body else {
            self.error("Structured variant expected".to_string(), pattern.span);
            return;
        };
        self.match_struct_pattern_fields(
            body,
            fields,
            &format!("{}.{}", pattern.ty.name, pattern.name),
            variables,
        );
    }

    fn match_tuple_variant(
        &mut self,
        pattern: &ast::VariantPattern,
        def: TypeId,
        variables: &mut TokenList,
    ) {
        let Some(ast::VariantPatternBody::Tuple(ref body)) = pattern.body else {
            self.error("Tuple variant expected".to_string(), pattern.span);
            return;
        };
        self.match_tuple_pattern(body, def, variables);
    }

    fn match_unit_variant(&mut self, pattern: &ast::VariantPattern) {
        if pattern.body.is_some() {
            self.error(
                "No body expected for unit variant".to_string(),
                pattern.span,
            );
        }
    }
}
