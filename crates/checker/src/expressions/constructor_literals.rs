use std::collections::HashSet;
use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir as ir;
use tine_symbols::symbols::*;

use crate::{expressions::path::PathContext, substitutions::Substitutions};

use super::super::TypeChecker;

impl TypeChecker {
    pub fn visit_struct_expression(
        &mut self,
        node: ast::StructExpression,
    ) -> Option<ir::StructExpression> {
        self.visit_struct_like(
            node.loc,
            node.constructor,
            node.fields,
            TypeChecker::visit_struct_field,
            |this, field| {
                if let Some(v) = field.value {
                    this.visit_expression(v);
                }
            },
        )
    }

    pub fn visit_struct_like<Field>(
        &mut self,
        loc: Location,
        constructor: ast::PathExpression,
        fields: Vec<Field>,
        mut visit_field: impl FnMut(
            &mut Self,
            Field,
            &[MemberSymbolId],
            &mut HashSet<String>,
            &mut Substitutions,
        ) -> Option<ir::StructLiteralField>,
        mut visit_field_on_error: impl FnMut(&mut Self, Field),
    ) -> Option<ir::StructExpression> {
        let path = self.visit_path_expression(constructor, PathContext::Struct)?;
        let ir::Expression::Identifier(ir::Identifier {
            ty,
            symbol: SymbolId::Struct(symbol),
            loc: constructor_loc,
        }) = path
        else {
            fields
                .into_iter()
                .for_each(|f| visit_field_on_error(self, f));
            self.error(DiagnosticKind::InvalidTypeConstructor, path.loc());
            return None;
        };

        let members = self.symbols.get(symbol).members.clone();
        let (struct_ty, mut sub) = self.unwrap_type(ty);

        let mut encountered = HashSet::new();
        let fields = fields
            .into_iter()
            .map(|field| visit_field(self, field, &members, &mut encountered, &mut sub))
            .collect::<Vec<Option<_>>>()
            .into_iter()
            .collect::<Option<Vec<_>>>()?;

        let missing = members
            .into_iter()
            .map(|m| &self.symbols.get(m).name)
            .filter(|n| !encountered.contains(*n))
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            self.error(DiagnosticKind::MissingMembers(missing), constructor_loc);
        }
        let ty = sub.apply(&mut self.types, struct_ty);
        self.errors(sub.produce_diagnostics(&self.types), loc);

        Some(ir::StructExpression {
            loc,
            constructor: (constructor_loc, symbol),
            fields,
            ty,
        })
    }

    fn visit_struct_field(
        &mut self,
        field: ast::StructExprField,
        members: &[MemberSymbolId],
        encountered_field_names: &mut HashSet<String>,
        mut substitutions: &mut Substitutions,
    ) -> Option<ir::StructLiteralField> {
        let key = match field.key {
            Some(ast::StructExprFieldKey::Name(n)) => n,
            Some(_) => {
                self.error(DiagnosticKind::InvalidMember, field.loc);
                field.value.and_then(|v| self.visit_expression(v));
                return None;
            }
            None => {
                field.value.and_then(|v| self.visit_expression(v));
                return None;
            }
        };

        let Some(&symbol) = members.iter().find(|&&f| self.symbol_name(f) == key.text) else {
            let error = DiagnosticKind::UnknownMember {
                member: key.as_str().to_string(),
            };
            self.error(error, key.loc);
            return None;
        };
        if !self.is_visible(symbol.into()) {
            let error = DiagnosticKind::FieldIsPrivate(key.text.clone());
            self.error(error, field.loc);
        }
        encountered_field_names.insert(key.as_str().to_string());

        let key_loc = key.loc;
        let value = match field.value {
            Some(v) => {
                self.check_expression_against(v, self.symbol_type_id(symbol), &mut substitutions)?
            }
            None => self.visit_identifier(key).map(Into::into)?,
        };

        Some(ir::StructLiteralField {
            loc: field.loc,
            name: (key_loc, symbol),
            value,
        })
    }
}
