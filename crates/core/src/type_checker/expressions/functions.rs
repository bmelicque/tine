use crate::{
    ast,
    type_checker::{analysis_context::type_store::TypeStore, TypeChecker},
    types::{FunctionType, Type, TypeId},
    SymbolData, SymbolKind,
};

impl TypeChecker<'_> {
    pub fn visit_function_expression(&mut self, node: &ast::FunctionExpression) -> TypeId {
        let (params, return_type) = self.with_scope(|s| {
            let param_types = s.visit_function_params(&node);
            let body_type = s.visit_function_body(node);
            (param_types, body_type)
        });

        let ty = self.intern(Type::Function(FunctionType {
            params,
            return_type,
        }));
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_function_params(&mut self, node: &ast::FunctionExpression) -> Vec<TypeId> {
        let mut param_types = Vec::with_capacity(node.params.len());
        for param in node.params.iter() {
            let ty = self.visit_type(&param.type_annotation);
            self.ctx.register_symbol(SymbolData {
                name: param.name.as_str().into(),
                ty,
                kind: SymbolKind::constant(),
                defined_at: param.name.loc,
                ..Default::default()
            });
            param_types.push(ty);
        }
        param_types
    }

    pub fn visit_function_body(&mut self, node: &ast::FunctionExpression) -> TypeId {
        let return_type = node
            .return_type
            .as_ref()
            .map(|ty| self.visit_type(ty))
            .unwrap_or(TypeStore::UNIT);

        let body_type = self.visit_block_expression(&node.body);
        let mut returns = Vec::<ast::ReturnStatement>::new();
        node.body.find_returns(&mut returns);
        for ret in returns {
            let ty = match ret.value {
                Some(value) => self.get_type_at(value.loc()).unwrap(),
                None => TypeStore::UNIT,
            };
            self.check_assigned_type(return_type, ty, ret.loc);
        }

        if let Some(ast::Statement::Expression(expr)) = node.body.statements.last() {
            if return_type != TypeStore::UNIT {
                self.check_assigned_type(return_type, body_type, expr.expression.loc());
            }
        }

        return_type
    }
}
