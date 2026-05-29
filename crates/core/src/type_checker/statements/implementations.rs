use crate::{
    ast, ir,
    type_checker::TypeChecker,
    types::{self, FunctionType, GenericType, Type, TypeId},
    DiagnosticKind, SymbolData, SymbolKind, SymbolRef, TypeStore,
};

impl TypeChecker<'_> {
    pub(super) fn visit_implementation(&mut self, node: ast::Implementation) -> Vec<ir::Statement> {
        let implemented_type = node.implemented_type.as_ref();
        let owner = implemented_type.and_then(|t| self.lookup(&t.name));
        let mut owner_args = implemented_type
            .and_then(|t| t.args.clone())
            .unwrap_or(vec![])
            .into_iter()
            .map(|arg| self.visit_type(arg))
            .collect();
        if owner.is_none() && implemented_type.is_some() {
            let ast::NamedType { name, loc, .. } = implemented_type.unwrap();
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

        let Some(body) = node.body else {
            return vec![];
        };
        body.items
            .into_iter()
            .filter_map(|item| {
                self.visit_impl_item(owner.clone(), receiver_type, &owner_args, item)
            })
            .collect()
    }

    fn visit_impl_item(
        &mut self,
        owner: Option<SymbolRef>,
        receiver_type: TypeId,
        owner_args: &Vec<TypeId>,
        item: ast::ImplementationItem,
    ) -> Option<ir::Statement> {
        match item {
            ast::ImplementationItem::Method(m) => self
                .visit_method_definition(m, owner, receiver_type, owner_args)
                .map(Into::into),
            ast::ImplementationItem::StaticMethod(m) => self
                .visit_static_definition(m, owner, owner_args)
                .map(Into::into),
        }
    }

    pub fn visit_method_definition(
        &mut self,
        node: ast::MethodDefinition,
        receiver: Option<SymbolRef>,
        receiver_type: TypeId,
        owner_args: &Vec<TypeId>,
    ) -> Option<ir::FunctionDefinition> {
        let ((params, visited_body), type_params) = self.with_type_params(&node.type_params, |s| {
            let params = s.visit_function_params(node.params);
            let visited_body = s.visit_function_body(node.return_type, node.body);
            (params, visited_body)
        });
        let receiver = receiver?;

        let (params, (return_type, body)) = (params?, visited_body?);
        let mut param_types = params.iter().map(|p| p.ty()).collect::<Vec<_>>();
        param_types.insert(0, receiver_type);

        let ty = self.intern(FunctionType {
            params: param_types,
            return_type,
        });
        let ty = match type_params.len() {
            0 => ty,
            _ => self.intern(Type::Generic(GenericType {
                params: type_params,
                definition: ty,
            })),
        };

        let name = node.name?;
        let symbol = self.ctx.register_symbol(SymbolData {
            name: name.text,
            ty,
            kind: SymbolKind::Method {
                owner: receiver,
                owner_args: owner_args.clone(),
                has_receiver: true,
                param_names: params.iter().map(|p| p.as_name()).collect(),
            },
            defined_at: name.loc,
            docs: node.docs.map(|d| d.text),
            ..Default::default()
        });
        let name = ir::Identifier {
            loc: name.loc,
            symbol,
        };

        Some(ir::FunctionDefinition {
            loc: node.loc,
            name,
            params,
            body,
            ty,
        })
    }

    fn visit_static_definition(
        &mut self,
        node: ast::FunctionDefinition,
        owner: Option<SymbolRef>,
        owner_args: &Vec<TypeId>,
    ) -> Option<ir::FunctionDefinition> {
        let ((params, visited_body), type_params) =
            self.with_type_params(&node.definition.type_params, |s| {
                let params = s.visit_function_params(node.definition.params);
                let visited_body =
                    s.visit_function_body(node.definition.return_type, node.definition.body);
                (params, visited_body)
            });

        let owner = owner?;
        let (params, (return_type, body)) = (params?, visited_body?);

        let ty = self.intern(FunctionType {
            params: params.iter().map(|p| p.ty()).collect::<Vec<_>>(),
            return_type,
        });
        let ty = match type_params.len() {
            0 => ty,
            _ => self.intern(Type::Generic(GenericType {
                params: type_params,
                definition: ty,
            })),
        };

        let name = node.definition.name?;
        let symbol = self.ctx.register_symbol(SymbolData {
            name: name.text,
            ty,
            kind: SymbolKind::Method {
                owner,
                owner_args: owner_args.clone(),
                has_receiver: false,
                param_names: params.iter().map(|p| p.as_name()).collect(),
            },
            defined_at: name.loc,
            docs: node.docs.map(|d| d.text),
            ..Default::default()
        });
        let name = ir::Identifier {
            loc: name.loc,
            symbol,
        };

        Some(ir::FunctionDefinition {
            loc: node.definition.loc,
            name,
            params,
            body,
            ty,
        })
    }
}
