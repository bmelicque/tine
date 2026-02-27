use anyhow::{anyhow, Result};

use crate::{
    ast,
    type_checker::analysis_context::type_store::TypeStore,
    types::{self, TypeId},
    DiagnosticKind, Location,
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

enum PatternFields {
    Struct(Vec<types::StructField>),
    Tuple(types::TupleType),
}

impl TypeChecker<'_> {
    pub fn match_pattern(
        &mut self,
        pattern: &ast::Pattern,
        against: TypeId,
        variables: &mut TokenList,
    ) {
        match pattern {
            ast::Pattern::Invalid { .. } => {}
            ast::Pattern::Identifier(id) => variables.insert(id.clone().into(), against),
            ast::Pattern::Literal(l) => self.match_literal_pattern(l, against),
            ast::Pattern::Constructor(pattern) => {
                self.match_constructor_pattern(pattern, against, variables)
            }
            ast::Pattern::Tuple(pattern) => self.match_tuple_pattern(pattern, against, variables),
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

    fn match_constructor_pattern(
        &mut self,
        pattern: &ast::ConstructorPattern,
        against: TypeId,
        variables: &mut TokenList,
    ) {
        let expected_fields = match &pattern.constructor {
            ast::Constructor::Invalid(_) => Err(anyhow!("")),
            ast::Constructor::Map(_) => unimplemented!(),
            ast::Constructor::Named(name) => {
                self.validate_pattern_name(name, None, against, pattern.loc)
            }
            ast::Constructor::Variant(variant) => self.validate_pattern_name(
                &variant.enum_name,
                variant.variant_name.as_ref().map(|i| i.as_str()),
                against,
                pattern.loc,
            ),
        };

        match expected_fields {
            Ok(Some(PatternFields::Struct(st))) => match &pattern.body {
                Some(ast::ConstructorPatternBody::Struct(pat)) => {
                    self.match_struct_pattern_fields(&pat.fields, &st, variables);
                }
                _ => {}
            },
            Ok(Some(PatternFields::Tuple(t))) => match &pattern.body {
                Some(ast::ConstructorPatternBody::Tuple(pat)) => {
                    let against = self.intern(t.into());
                    self.match_tuple_pattern(pat, against, variables);
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn validate_pattern_name(
        &mut self,
        name: &ast::NamedType,
        variant: Option<&str>,
        against: TypeId,
        loc: Location,
    ) -> Result<Option<PatternFields>> {
        let Some(symbol) = self.lookup(&name.name) else {
            let error = DiagnosticKind::CannotFindName {
                name: name.name.to_string(),
            };
            self.error(error, name.loc);
            return Err(anyhow!(""));
        };

        let got = symbol.borrow().get_type();
        if against != got {
            // TODO: handle generics
            self.error(DiagnosticKind::InvalidPattern, loc);
        }

        match self.resolve(against) {
            types::Type::Struct(st) => Ok(Some(PatternFields::Struct(st.fields))),
            types::Type::Tuple(t) => Ok(Some(PatternFields::Tuple(t))),
            types::Type::Enum(e) => {
                let Some(variant) = variant else {
                    self.error(DiagnosticKind::InvalidPattern, loc);
                    return Err(anyhow!(""));
                };
                let Some(variant) = e.variants.iter().find(|v| v.name == *variant) else {
                    let error = DiagnosticKind::UnknownVariant {
                        variant: variant.to_string(),
                        enum_name: name.name.clone(),
                    };
                    self.error(error, loc);
                    return Err(anyhow!(""));
                };
                match self.resolve(variant.def) {
                    types::Type::Struct(st) => Ok(Some(PatternFields::Struct(st.fields))),
                    types::Type::Tuple(t) => Ok(Some(PatternFields::Tuple(t))),
                    types::Type::Unit => Ok(None),
                    _ => unreachable!(),
                }
            }
            _ => {
                self.error(DiagnosticKind::InvalidPattern, loc);
                return Err(anyhow!(""));
            }
        }
    }

    fn match_struct_pattern_fields(
        &mut self,
        pattern_fields: &Vec<ast::StructPatternField>,
        against_fields: &Vec<types::StructField>,
        variables: &mut TokenList,
    ) {
        for field in pattern_fields.iter().filter(|f| f.identifier.is_some()) {
            let Some(identifier) = &field.identifier else {
                continue;
            };
            let Some(against) = against_fields
                .iter()
                .find(|f| f.name == identifier.as_str())
            else {
                let error = DiagnosticKind::UnknownMember {
                    member: identifier.text.clone(),
                };
                self.error(error, field.loc);
                continue;
            };
            match field.pattern {
                Some(ref sub_pattern) => {
                    self.match_pattern(sub_pattern, against.def.clone(), variables)
                }
                None => variables.insert(identifier.clone(), against.def.clone()),
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
}
