use crate::{
    ast,
    type_checker::analysis_context::type_store::TypeStore,
    types::{self, Type, TypeId},
    DiagnosticKind,
};

use super::TypeChecker;

#[derive(Debug, Clone)]
pub struct TokenList(pub Vec<(ast::Identifier, TypeId)>);
impl TokenList {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn insert(&mut self, ident: ast::Identifier, ty: TypeId) {
        self.0.push((ident, ty));
    }
}

impl TypeChecker<'_> {
    pub fn match_pattern(
        &mut self,
        pattern: &ast::Pattern,
        against: TypeId,
        variables: &mut TokenList,
    ) {
        match pattern {
            ast::Pattern::Identifier(id) => variables.insert(id.clone().into(), against),
            ast::Pattern::Literal(l) => self.match_literal_pattern(l, against),
            ast::Pattern::Struct(pattern) => self.match_struct_pattern(pattern, against, variables),
            ast::Pattern::Tuple(pattern) => self.match_tuple_pattern(pattern, against, variables),
            ast::Pattern::Variant(pattern) => {
                self.match_variant_pattern(pattern, against, variables)
            }
        }
    }

    fn match_literal_pattern(&mut self, pattern: &ast::LiteralPattern, against_id: TypeId) {
        let against = self.resolve(against_id);
        let got = match pattern {
            ast::LiteralPattern::Boolean(_) => types::Type::Boolean,
            ast::LiteralPattern::Float(_) => types::Type::Float,
            ast::LiteralPattern::Integer(_) => types::Type::Integer,
            ast::LiteralPattern::String(_) => types::Type::String,
        };
        if against != types::Type::Unknown && against != got {
            let error = DiagnosticKind::InvalidPatternMatch {
                expected: self.session.display_type(against_id),
                got: got.to_string(),
            };
            self.error(error, pattern.loc());
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
        let Some(ty) = self.ctx.lookup(&pattern.ty.name) else {
            let error = DiagnosticKind::CannotFindName {
                name: pattern.ty.name.to_string(),
            };
            self.error(error, pattern.ty.loc);
            return;
        };
        let Type::Struct(pattern_type) = self.resolve(ty.borrow().get_type()).clone() else {
            let error = DiagnosticKind::ExpectedStruct {
                got: pattern.ty.name.clone(),
            };
            self.error(error, pattern.loc);
            return;
        };

        let ty = match self.resolve(against) {
            Type::Struct(st) if st.id == pattern_type.id => st.clone(),
            _ => {
                self.error(DiagnosticKind::InvalidPattern, pattern.loc);
                return;
            }
        };

        self.match_struct_pattern_fields(&pattern.fields, &ty.fields, variables);
    }

    fn match_struct_pattern_fields(
        &mut self,
        pattern_fields: &Vec<ast::StructPatternField>,
        against_fields: &Vec<types::StructField>,
        variables: &mut TokenList,
    ) {
        for field in pattern_fields.iter() {
            let Some(against) = against_fields
                .iter()
                .find(|f| f.name == field.identifier.as_str())
            else {
                let error = DiagnosticKind::UnknownMember {
                    member: field.identifier.text.clone(),
                };
                self.error(error, field.loc);
                continue;
            };
            match field.pattern {
                Some(ref sub_pattern) => {
                    self.match_pattern(sub_pattern, against.def.clone(), variables)
                }
                None => variables.insert(field.identifier.clone(), against.def.clone()),
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
            self.error(DiagnosticKind::ExpectedTuplePattern, pattern.loc);
            return;
        };

        if pattern.elements.len() != ty.elements.len() {
            let error = DiagnosticKind::TupleElementCountMismatch {
                expected: ty.elements.len(),
                got: pattern.elements.len(),
            };
            self.error(error, pattern.loc);
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
        let Some(ty) = self.ctx.lookup(&pattern.ty.name) else {
            let error = DiagnosticKind::CannotFindName {
                name: pattern.ty.name.clone(),
            };
            self.error(error, pattern.ty.loc);
            return;
        };
        let Type::Enum(pattern_type) = self.resolve(ty.borrow().get_type()).clone() else {
            let error = DiagnosticKind::ExpectedEnum {
                got: pattern.ty.name.clone(),
            };
            self.error(error, pattern.loc);
            return;
        };

        let ty = match self.resolve(against) {
            Type::Enum(e) if e.id == pattern_type.id => e.clone(),
            _ => {
                self.error(DiagnosticKind::InvalidPattern, pattern.loc);
                return;
            }
        };

        let Some(variant) = ty.variants.iter().find(|var| var.name == pattern.name) else {
            let error = DiagnosticKind::UnknownVariant {
                variant: pattern.name.clone(),
                enum_name: pattern.ty.name.clone(),
            };
            self.error(error, pattern.loc);
            return;
        };

        match self.resolve(variant.def).clone() {
            Type::Struct(ty) => self.match_struct_variant(pattern, &ty.fields, variables),
            Type::Tuple(def) => {
                let id = self.session.find_type(&def.into()).unwrap();
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
            self.error(DiagnosticKind::ExpectedVariantStruct, pattern.loc);
            return;
        };
        self.match_struct_pattern_fields(body, fields, variables);
    }

    fn match_tuple_variant(
        &mut self,
        pattern: &ast::VariantPattern,
        def: TypeId,
        variables: &mut TokenList,
    ) {
        let Some(ast::VariantPatternBody::Tuple(ref body)) = pattern.body else {
            self.error(DiagnosticKind::ExpectedVariantTuple, pattern.loc);
            return;
        };
        self.match_tuple_pattern(body, def, variables);
    }

    fn match_unit_variant(&mut self, pattern: &ast::VariantPattern) {
        if pattern.body.is_some() {
            self.error(DiagnosticKind::ExpectedVariantUnit, pattern.loc);
        }
    }
}
