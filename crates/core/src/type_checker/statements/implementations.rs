use crate::{
    ast,
    type_checker::TypeChecker,
    types::{self, FunctionType, GenericType, Type, TypeId},
    DiagnosticKind, SymbolData, SymbolKind, SymbolRef, TypeStore,
};

impl TypeChecker<'_> {
    pub(super) fn visit_implementation(&mut self, node: &ast::Implementation) -> TypeId {
        let implemented_type = node.implemented_type.as_ref();
        let owner = implemented_type.and_then(|t| self.lookup(&t.name));
        let mut owner_args = implemented_type
            .and_then(|t| t.args.clone())
            .unwrap_or(vec![])
            .iter()
            .map(|arg| self.visit_type(arg))
            .collect();
        if owner.is_none() && node.implemented_type.is_some() {
            let ast::NamedType { name, loc, .. } = node.implemented_type.as_ref().unwrap();
            self.error(
                DiagnosticKind::CannotFindName { name: name.clone() },
                loc.clone(),
            );
        }

        let mut receiver_type = owner.as_ref().map_or(TypeStore::UNKNOWN, |o| o.borrow().ty);
        if let types::Type::Generic(g) = self.resolve(receiver_type) {
            receiver_type = self.visit_generic_instance_with_args(
                &g,
                &mut owner_args,
                implemented_type.unwrap().loc,
            )
        }

        let Some(body) = &node.body else {
            return TypeStore::UNIT;
        };
        for item in &body.items {
            self.visit_impl_item(&owner, receiver_type, &owner_args, item);
        }
        TypeStore::UNIT
    }

    fn visit_impl_item(
        &mut self,
        owner: &Option<SymbolRef>,
        receiver_type: TypeId,
        owner_args: &Vec<TypeId>,
        item: &ast::ImplementationItem,
    ) {
        let docs = item.docs().as_ref().map(|d| d.text.clone());
        let (ty, dependencies) =
            match item {
                ast::ImplementationItem::Method(m) => self
                    .with_dependencies(|checker| checker.visit_method_definition(m, receiver_type)),
                ast::ImplementationItem::StaticMethod(m) => self
                    .with_dependencies(|checker| checker.visit_function_expression(&m.definition)),
            };

        let Some(owner) = owner else { return };
        let owner = owner.clone();
        let Some(name) = item.name() else { return };
        let name = name.clone();
        let has_receiver = matches!(item, ast::ImplementationItem::Method(_));

        let params = match item {
            ast::ImplementationItem::Method(m) => &m.params,
            ast::ImplementationItem::StaticMethod(m) => &m.definition.params,
        };

        self.ctx.register_symbol(SymbolData {
            docs,
            name: name.as_str().to_string(),
            ty,
            kind: SymbolKind::Method {
                owner: owner.clone(),
                owner_args: owner_args.clone(),
                has_receiver,
                param_names: params
                    .as_ref()
                    .map(|p| &p.params)
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|param| param.name.text.clone())
                    .collect(),
            },
            defined_at: name.loc,
            dependencies: dependencies.clone(),
            ..Default::default()
        });
    }

    pub fn visit_method_definition(
        &mut self,
        node: &ast::MethodDefinition,
        receiver: TypeId,
    ) -> TypeId {
        let ((params, return_type), type_params) = self.with_type_params(&node.type_params, |s| {
            let param_types = vec![vec![receiver], s.visit_function_params(&node.params)].concat();
            let body_type = s.visit_function_body(&node.return_type, &node.body);
            (param_types, body_type)
        });

        let ty = self.intern(Type::Function(FunctionType {
            params,
            return_type,
        }));
        let ty = match type_params.len() {
            0 => ty,
            _ => self.intern(Type::Generic(GenericType {
                params: type_params,
                definition: ty,
            })),
        };

        self.ctx.save_expression_type(node.loc, ty)
    }
}
