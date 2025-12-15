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
            let body_type = s.visit_function_body(&node.body);
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

    pub fn visit_function_body(&mut self, node: &ast::FunctionBody) -> TypeId {
        let block = match node {
            ast::FunctionBody::Expression(node) => return self.visit_expression(node),
            ast::FunctionBody::TypedBlock(node) => node,
        };

        let ty = if let Some(ref type_annotation) = block.type_annotation {
            self.visit_type(type_annotation)
        } else {
            TypeStore::UNIT
        };
        self.visit_block_expression(&block.block);
        self.check_returns(block, ty);
        ty
    }

    fn check_returns(&mut self, body: &ast::TypedBlock, expected: TypeId) {
        let mut returns = Vec::<ast::ReturnStatement>::new();
        body.block.find_returns(&mut returns);

        if returns.len() == 0 && expected != TypeStore::UNIT {
            self.error(
                "A function with return annotation needs a return value".into(),
                body.block.loc,
            );
        }

        for ret in returns {
            let ty = match ret.value {
                Some(value) => self.get_type_at(value.loc()).unwrap(),
                None => TypeStore::UNIT,
            };
            self.check_assigned_type(expected, ty, ret.loc);
        }
    }
}
