use crate::{
    ast,
    type_checker::{analysis_context::type_store::TypeStore, TypeChecker},
    types::{FunctionType, Type, TypeId},
    VariableData,
};

impl TypeChecker {
    pub fn visit_function_expression(&mut self, node: &ast::FunctionExpression) -> TypeId {
        let (params, return_type) = self.with_scope(node.span, |s| {
            let param_types = s.visit_function_params(&node);
            let body_type = s.visit_function_body(&node.body);
            (param_types, body_type)
        });

        let ty = self
            .analysis_context
            .type_store
            .add(Type::Function(FunctionType {
                params,
                return_type,
            }));
        self.analysis_context.save_expression_type(node.span, ty)
    }

    fn visit_function_params(&mut self, node: &ast::FunctionExpression) -> Vec<TypeId> {
        let mut param_types = Vec::with_capacity(node.params.len());
        for param in node.params.iter() {
            let ty = self.visit_type(&param.type_annotation);
            self.analysis_context.register_symbol(VariableData::pure(
                param.name.as_str().into(),
                ty,
                param.name.span,
            ));
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
            TypeStore::VOID
        };
        self.visit_block_expression(&block.block);
        self.check_returns(block, ty);
        ty
    }

    fn check_returns(&mut self, body: &ast::TypedBlock, expected: TypeId) {
        let mut returns = Vec::<ast::ReturnStatement>::new();
        body.block.find_returns(&mut returns);

        if returns.len() == 0 && expected != TypeStore::VOID {
            self.error(
                "A function with return annotation needs a return value".into(),
                body.block.span,
            );
        }

        for ret in returns {
            let ty = match ret.value {
                Some(value) => self.get_type_at(value.as_span()).unwrap(),
                None => TypeStore::VOID,
            };
            self.check_assigned_type(expected, ty, ret.span);
        }
    }
}
