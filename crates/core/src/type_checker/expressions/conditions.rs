use crate::{
    ast,
    type_checker::{analysis_context::type_store::TypeStore, patterns::TokenList, TypeChecker},
    types::{OptionType, Type, TypeId},
    SymbolData, SymbolKind,
};

impl TypeChecker<'_> {
    pub fn visit_if_expression(&mut self, node: &ast::IfExpression) -> TypeId {
        self.visit_condition(&node.condition);
        let ty = self.with_scope(|s| s.visit_block_expression(&node.consequent));
        let ty = if let Some(ref alternate) = node.alternate {
            self.visit_alternate(alternate, ty);
            ty
        } else {
            self.ctx
                .type_store
                .add(Type::Option(OptionType { some: ty }))
        };
        self.ctx.save_expression_type(node.loc, ty)
    }

    pub fn visit_condition(&mut self, node: &ast::Expression) {
        let condition = self.visit_expression(node);
        if condition != TypeStore::BOOLEAN {
            let condition = self.ctx.type_store.get(condition);
            self.error(
                format!("Condition must evaluate to a boolean, got {}", *condition),
                node.loc(),
            );
        }
    }

    pub fn visit_if_decl_expression(&mut self, node: &ast::IfPatExpression) -> TypeId {
        if !node.pattern.is_refutable() {
            self.error("Refutable pattern expected".into(), node.pattern.loc());
        };

        let ty = self.with_scope(|s| {
            let (inferred_type, dependencies) =
                s.with_dependencies(|s| s.visit_expression(&node.scrutinee));
            let mut variables = TokenList::new();
            s.match_pattern(&node.pattern, inferred_type.clone(), &mut variables);
            for (name, ty) in variables.0 {
                s.ctx.register_symbol(SymbolData {
                    name: name.as_str().into(),
                    ty,
                    kind: SymbolKind::constant(),
                    defined_at: node.pattern.loc(),
                    dependencies: dependencies.clone(),
                    ..Default::default()
                });
                s.ctx.save_expression_type(name.loc, ty);
            }
            s.visit_block_expression(&node.consequent)
        });

        let ty = if let Some(ref alternate) = node.alternate {
            self.visit_alternate(alternate, ty);
            ty
        } else {
            self.ctx
                .type_store
                .add(Type::Option(OptionType { some: ty }))
        };
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_alternate(&mut self, alternate: &ast::Alternate, expected: TypeId) {
        let alt_ty = match alternate {
            ast::Alternate::Block(b) => self.visit_block_expression(b),
            ast::Alternate::If(i) => self.visit_if_expression(i),
            ast::Alternate::IfDecl(i) => self.visit_if_decl_expression(i),
        };
        if !self.can_be_assigned_to(alt_ty, expected) {
            let expected = self.ctx.type_store.get(expected);
            let alt_ty = self.ctx.type_store.get(alt_ty);
            let error = format!(
                "Branches' types don't match: expected {}, got {}",
                expected, alt_ty
            );
            self.error(error, alternate.loc())
        }
    }
}
