use std::collections::HashSet;

use tine_ast as ast;
use tine_common::{diagnostics::DiagnosticKind, locations::Locatable};
use tine_ir::{self as ir, Typed};
use tine_symbols::symbols::{MemberSymbolId, SymbolId};
use tine_types::types;

use crate::{substitutions::Substitutions, PathContext, TypeChecker};

#[derive(Debug, Default, Clone, Copy)]
struct Ctx {
    indirected: bool,
}
impl Ctx {
    fn to_indirected(&self) -> Self {
        Self { indirected: true }
    }
}

impl TypeChecker {
    pub fn visit_assignment(&mut self, node: ast::Assignment) -> Option<ir::Assignment> {
        let assignee = node
            .pattern
            .and_then(|a| self.visit_assignee(a, Ctx::default()));
        let value = node.value.and_then(|v| self.visit_expression(v));
        let (assignee, value) = match (assignee, value) {
            (Some(assignee), Some(value)) => (assignee, value),
            _ => return None,
        };
        if !self.can_be_assigned_to(value.ty(), assignee.ty(), false) {
            let left_name = self.types.display(assignee.ty());
            let right_name = self.types.display(value.ty());
            let error = DiagnosticKind::MismatchedTypes {
                left_name,
                right_name,
            };
            self.error(error, node.loc);
            return None;
        }

        Some(ir::Assignment {
            loc: node.loc,
            pattern: assignee,
            value,
        })
    }

    fn visit_assignee(&mut self, assignee: ast::Assignee, ctx: Ctx) -> Option<ir::Expression> {
        use ast::Assignee::*;
        match assignee {
            Invalid(_) => None,
            Path(a) => self.visit_path_assignee(a, ctx),
            Indirection(a) => self.visit_indirection_assignee(a, ctx),
            Struct(a) => self.visit_struct_assignee(a, ctx),
            Tuple(a) => self.visit_tuple_assignee(a, ctx),
        }
    }

    fn visit_path_assignee(
        &mut self,
        assignee: ast::PathExpression,
        ctx: Ctx,
    ) -> Option<ir::Expression> {
        let assignee_loc = assignee.loc;
        let expr = self.visit_path_expression(assignee, PathContext::Expr)?;
        let (symbol, mutable) = match self.get_path_root(&expr) {
            Ok(Some(ir::Identifier {
                symbol: SymbolId::Variable(s),
                ..
            })) => (Some(*s), self.symbols.get(*s).mutable),
            // `None` this is already reported as an error in `visit_path_expression`
            Ok(None) => (None, self.mutable_this.unwrap_or(true)),
            _ => {
                self.error(DiagnosticKind::InvalidAssignTarget, expr.loc());
                return None;
            }
        };
        if !ctx.indirected && !mutable {
            let error = match symbol {
                Some(symbol) => {
                    let name = self.symbol_name(symbol).to_string();
                    DiagnosticKind::AssignmentToConstant { name }
                }
                None => DiagnosticKind::HostMutation,
            };
            self.error(error, assignee_loc);
            return None;
        }
        Some(expr)
    }

    fn get_path_root<'a>(
        &mut self,
        assignee: &'a ir::Expression,
    ) -> Result<Option<&'a ir::Identifier>, ()> {
        use ir::Expression::*;
        match assignee {
            Identifier(i) => Ok(Some(i)),
            Member(m) => m.root_identifier(),
            _ => Err(()),
        }
    }

    fn visit_indirection_assignee(
        &mut self,
        node: ast::IndirectionAssignee,
        ctx: Ctx,
    ) -> Option<ir::Expression> {
        let operand = node
            .inner
            .and_then(|i| self.visit_assignee(*i, ctx.to_indirected()))?;
        let ty = match self.deref_type(operand.ty()) {
            Ok(ty) => ty?,
            Err(e) => {
                self.error(e, node.loc);
                return None;
            }
        };
        Some(ir::Expression::Unary(ir::UnaryExpression {
            loc: node.loc,
            operator: ir::UnaryOperator::Star,
            operand: Box::new(operand),
            ty,
        }))
    }

    fn visit_struct_assignee(
        &mut self,
        node: ast::StructAssignee,
        ctx: Ctx,
    ) -> Option<ir::Expression> {
        self.visit_struct_like(
            node.loc,
            node.constructor,
            node.fields,
            |t, f, m, e, s| t.visit_struct_assignee_field(f, m, e, s, ctx),
            |this, field| {
                if let Some(v) = field.value {
                    this.visit_assignee(v, ctx);
                }
            },
        )
        .map(Into::into)
    }

    fn visit_struct_assignee_field(
        &mut self,
        field: ast::StructAssigneeField,
        members: &[MemberSymbolId],
        encountered_field_names: &mut HashSet<String>,
        substitutions: &mut Substitutions,
        ctx: Ctx,
    ) -> Option<ir::StructLiteralField> {
        let Some(ast::StructAssigneeFieldKey::Identifier(key)) = field.key else {
            field.value.and_then(|v| self.visit_assignee(v, ctx));
            return None;
        };

        let Some(&symbol) = members.iter().find(|&&f| self.symbol_name(f) == key.text) else {
            let member = key.as_str().to_string();
            let error = DiagnosticKind::UnknownMember { member };
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
                let loc = v.loc();
                let got = self.visit_assignee(v, ctx)?;
                substitutions.unify(self, self.symbol_type_id(symbol), got.ty(), loc);
                got
            }
            None => self.visit_identifier(key).map(Into::into)?,
        };

        Some(ir::StructLiteralField {
            loc: field.loc,
            name: (key_loc, symbol),
            value,
        })
    }

    fn visit_tuple_assignee(
        &mut self,
        node: ast::TupleAssignee,
        ctx: Ctx,
    ) -> Option<ir::Expression> {
        let elements = node
            .elements
            .into_iter()
            .map(|a| self.visit_assignee(a, ctx))
            .collect::<Option<Vec<_>>>()?;
        let ty = self.intern(types::TupleType {
            elements: elements.iter().map(|e| e.ty()).collect(),
            ..Default::default()
        });
        Some(ir::Expression::Tuple(ir::TupleExpression {
            loc: node.loc,
            elements,
            ty,
        }))
    }
}

#[cfg(test)]
mod tests {
    use tine_common::locations::Location;
    use tine_symbols::symbols::{VariableSymbol, VariableSymbolId};
    use tine_types::store::TypeStore;

    use super::*;

    fn dummy_assignment() -> ast::Assignment {
        ast::Assignment {
            loc: Location::dummy(),
            pattern: Some(ast::Identifier::new("a".into(), Location::dummy()).into()),
            value: Some(ast::Expression::IntLiteral(ast::IntLiteral::new(
                1,
                Location::dummy(),
            ))),
        }
    }

    #[test]
    fn visit_assignment_simple() {
        let mut checker = TypeChecker::new();
        let id = checker.symbols.insert::<VariableSymbolId>(VariableSymbol {
            name: "a".to_string(),
            ty: TypeStore::INTEGER,
            mutable: true,
            ..Default::default()
        });
        checker.current_scope().bind("a".to_string(), id.into());

        checker.visit_assignment(dummy_assignment());
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn visit_assignment_to_constant() {
        let mut checker = TypeChecker::new();
        let id = checker.symbols.insert::<VariableSymbolId>(VariableSymbol {
            name: "a".to_string(),
            ty: TypeStore::INTEGER,
            ..Default::default()
        });
        checker.current_scope().bind("a".to_string(), id.into());

        checker.visit_assignment(dummy_assignment());
        assert_eq!(checker.diagnostics.len(), 1);
        assert!(matches!(
            &checker.diagnostics[&0][0].kind,
            DiagnosticKind::AssignmentToConstant { .. }
        ))
    }

    #[test]
    fn visit_assignment_bad_type() {
        let mut checker = TypeChecker::new();
        let id = checker.symbols.insert::<VariableSymbolId>(VariableSymbol {
            name: "a".to_string(),
            ty: TypeStore::FLOAT,
            mutable: true,
            ..Default::default()
        });
        checker.current_scope().bind("a".to_string(), id.into());

        checker.visit_assignment(dummy_assignment());
        assert_eq!(checker.diagnostics.len(), 1);
        assert!(matches!(
            &checker.diagnostics[&0][0].kind,
            DiagnosticKind::MismatchedTypes { .. }
        ),);
    }

    #[test]
    fn visit_indirect_simple() {
        let mut checker = TypeChecker::new();
        let ty = checker.intern(types::SignalType {
            inner: TypeStore::INTEGER,
        });
        let id = checker.symbols.insert::<VariableSymbolId>(VariableSymbol {
            name: "a".to_string(),
            ty,
            ..Default::default()
        });
        checker.current_scope().bind("a".to_string(), id.into());

        checker.visit_assignment(ast::Assignment {
            loc: Location::dummy(),
            pattern: Some(ast::Assignee::Indirection(ast::IndirectionAssignee {
                loc: Location::dummy(),
                inner: Some(Box::new(
                    ast::Identifier::new("a".into(), Location::dummy()).into(),
                )),
            })),
            value: Some(ast::Expression::IntLiteral(ast::IntLiteral::new(
                1,
                Location::dummy(),
            ))),
        });
        assert!(
            checker.diagnostics.is_empty(),
            "expected no diagnostics, got {:?}",
            checker.diagnostics
        );
    }

    #[test]
    fn visit_assignment_unknown_variable() {
        let mut checker = TypeChecker::new();
        checker.visit_assignment(dummy_assignment());
        assert_eq!(checker.diagnostics.len(), 1);
        assert!(matches!(
            &checker.diagnostics[&0][0].kind,
            DiagnosticKind::CannotFindName { .. }
        ));
    }
}
