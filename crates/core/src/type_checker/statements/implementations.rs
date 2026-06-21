use crate::{
    ast::{self, ImplementationBody},
    ir,
    type_checker::{
        analysis_context::symbols::MethodReceiverKind, substitutions::Substitutions, TypeChecker,
    },
    types::{self, TraitMethod},
    DiagnosticKind, Location, SymbolData, SymbolKind, SymbolRef,
};

impl TypeChecker<'_> {
    pub(super) fn visit_implementation(&mut self, node: ast::Implementation) -> Vec<ir::Statement> {
        let Some(owner) = node
            .implemented_type
            .as_ref()
            .and_then(|t| self.lookup(&t.name))
        else {
            if node.implemented_type.is_some() {
                let ast::NamedType { name, loc, .. } = node.implemented_type.as_ref().unwrap();
                let name = name.clone();
                self.error(DiagnosticKind::CannotFindName { name }, *loc);
            }
            self.fallback_check_impl_body(node.body);
            return vec![];
        };

        let receiver_loc = node.implemented_type.as_ref().unwrap().loc;
        let Some(receiver_type) = node.implemented_type.map(|t| self.visit_named_type(t)) else {
            self.fallback_check_impl_body(node.body);
            return vec![];
        };

        let owner_type = self.resolve(owner.as_type());
        let type_params = owner_type.as_params().unwrap_or(&[]);
        let type_args = match self.resolve(receiver_type) {
            types::Type::Ref(r) => r.args,
            _ => vec![],
        };

        let substitutions = Substitutions::with_initial(type_params, &type_args);

        let Some(body) = node.body else {
            return vec![];
        };

        let owner = ir::Identifier {
            loc: receiver_loc,
            symbol: owner,
        };
        body.items
            .into_iter()
            .filter_map(|item| self.visit_impl_item(Some(owner.clone()), &substitutions, item))
            .collect()
    }

    fn visit_impl_item(
        &mut self,
        owner: Option<ir::Identifier>,
        owner_args: &Substitutions,
        item: ast::ImplementationItem,
    ) -> Option<ir::Statement> {
        match item {
            ast::ImplementationItem::Method(m) => self
                .visit_method_definition(m, owner, owner_args)
                .map(Into::into),
            ast::ImplementationItem::StaticMethod(m) => self
                .visit_static_definition(m, owner.map(|o| o.symbol), owner_args)
                .map(Into::into),
        }
    }

    fn visit_method_definition(
        &mut self,
        node: ast::MethodDefinition,
        receiver: Option<ir::Identifier>,
        owner_args: &Substitutions,
    ) -> Option<ir::MethodDefinition> {
        let ((params, visited_body), type_params) = self.with_type_params(&node.type_params, |s| {
            let params = s.visit_function_params(node.params);
            let visited_body = s.visit_function_body(node.return_type, node.body);
            (params, visited_body)
        });
        let receiver = receiver?;

        let (params, (return_type, body)) = (params?, visited_body?);
        let param_types = params.iter().map(|p| p.ty()).collect::<Vec<_>>();

        let ty = self.intern(types::FunctionType {
            type_params,
            params: param_types,
            return_type,
        });

        let name = node.name?;
        if receiver.symbol.has_field(&name.text) {
            let error = DiagnosticKind::DuplicateFieldName {
                name: name.text.clone(),
            };
            self.error(error, name.loc);
            return None;
        }
        let data_kind = SymbolKind::Method {
            owner: receiver.symbol.clone(),
            owner_args: owner_args.clone().into(),
            receiver: if node.receiver.mutable {
                MethodReceiverKind::Mutable
            } else {
                MethodReceiverKind::Immutable
            },
            param_names: params.iter().map(|p| p.as_name()).collect(),
        };
        let symbol_data = SymbolData {
            name: name.text,
            ty,
            kind: data_kind,
            defined_at: name.loc,
            docs: node.docs.map(|d| d.text),
            ..Default::default()
        };
        let symbol = self.attach_method(symbol_data, receiver.symbol.clone(), name.loc);
        let name = ir::Identifier {
            loc: name.loc,
            symbol,
        };

        Some(ir::MethodDefinition {
            loc: node.loc,
            receiver,
            mutating: node.receiver.mutable,
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
        owner_args: &Substitutions,
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

        let ty = self.intern(types::FunctionType {
            type_params,
            params: params.iter().map(|p| p.ty()).collect::<Vec<_>>(),
            return_type,
        });

        let name = node.definition.name?;
        let data_kind = SymbolKind::Method {
            owner: owner.clone(),
            owner_args: owner_args.clone().into(),
            receiver: MethodReceiverKind::Static,
            param_names: params.iter().map(|p| p.as_name()).collect(),
        };
        let symbol_data = SymbolData {
            name: name.text,
            ty,
            kind: data_kind,
            defined_at: name.loc,
            docs: node.docs.map(|d| d.text),
            ..Default::default()
        };
        let symbol = self.attach_method(symbol_data, owner, name.loc);
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

    fn attach_method(&mut self, data: SymbolData, receiver: SymbolRef, at: Location) -> SymbolRef {
        let symbol = self.ctx.register_symbol(data);
        if receiver.has_method(&symbol) {
            let name = symbol.as_name();
            self.error(DiagnosticKind::DuplicateMethodName { name }, at);
        } else {
            let receiver = self.session.get_handle(symbol.clone()).unwrap();
            receiver.attach_method(symbol.clone());
            self.add_method_to_store(&symbol);
        }
        symbol
    }
    fn add_method_to_store(&mut self, symbol: &SymbolRef) {
        let SymbolKind::Method { receiver, .. } = &symbol.borrow().kind else {
            panic!()
        };
        if receiver.is_static() {
            return;
        }
        let types::Type::Function(mut f) = self.resolve(symbol.as_type()) else {
            panic!()
        };
        let name = symbol.as_name();
        let receiver = f.params[0];
        f.params = f.params[1..].to_vec();
        let def = self.intern(f);
        self.session
            .types()
            .add_method(receiver, TraitMethod { name, def });
    }

    fn fallback_check_impl_body(&mut self, body: Option<ImplementationBody>) {
        body.map_or(vec![], |body| body.items)
            .into_iter()
            .for_each(|item| {
                self.visit_impl_item(None, &Substitutions::new(), item);
            });
    }
}
