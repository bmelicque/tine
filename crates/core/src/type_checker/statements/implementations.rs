use crate::{
    ast::{self, ImplementationBody},
    ir,
    type_checker::{substitutions::Substitutions, symbols::*, TypeChecker},
    types::{self, TraitMethod, TypeId},
    DiagnosticKind, Location,
};

impl TypeChecker {
    pub(super) fn visit_implementation(&mut self, node: ast::Implementation) -> Vec<ir::Statement> {
        let Some((loc, host, actual_type)) = self.visit_impl_host(node.implemented_type) else {
            self.fallback_check_impl_body(node.body);
            return vec![];
        };

        let substitutions = self.get_host_substitutions(host, actual_type);

        let Some(body) = node.body else {
            return vec![];
        };

        let host = (loc, host);
        body.items
            .into_iter()
            .filter_map(|item| self.visit_impl_item(Some(host), &substitutions, item))
            .collect()
    }

    /// Visit the ast node representing the implementation's host.
    ///
    /// Returns (its location, its corresponding type symbol, its type id).
    ///
    /// Note that the type id could be different from the one present in the
    /// symbol because of type arguments.
    fn visit_impl_host(
        &mut self,
        host: Option<ast::NamedType>,
    ) -> Option<(Location, TypeSymbolId, TypeId)> {
        let host = host?;
        let loc = host.loc;

        let Some(symbol) = self.get_symbol_id(host.name.as_str()) else {
            let name = host.name.text;
            self.error(DiagnosticKind::CannotFindName { name }, loc);
            return None;
        };
        let Some(symbol) = symbol.as_type_symbol_id() else {
            self.error(DiagnosticKind::ExpectedTypeGotValue, loc);
            return None;
        };

        let receiver_type = self.visit_named_type(host);

        Some((loc, symbol, receiver_type))
    }

    /// If the actual type is a concrete instance of the host's symbol type,
    /// returns the needed type substitutions.
    fn get_host_substitutions(&self, host: TypeSymbolId, actual_type: TypeId) -> Substitutions {
        let owner_type = self.symbol_type(host);
        let type_params = owner_type.as_params().unwrap_or(&[]);
        let type_args = match self.resolve(actual_type) {
            types::Type::Ref(r) => r.args,
            _ => vec![],
        };
        Substitutions::with_initial(type_params, &type_args)
    }

    fn visit_impl_item(
        &mut self,
        host: Option<(Location, TypeSymbolId)>,
        host_args: &Substitutions,
        item: ast::ImplementationItem,
    ) -> Option<ir::Statement> {
        match item {
            ast::ImplementationItem::Method(m) => self
                .visit_method_definition(m, host, host_args)
                .map(Into::into),
            ast::ImplementationItem::StaticMethod(m) => self
                .visit_static_definition(m, host, host_args)
                .map(Into::into),
        }
    }

    fn visit_method_definition(
        &mut self,
        node: ast::MethodDefinition,
        host: Option<(Location, TypeSymbolId)>,
        host_args: &Substitutions,
    ) -> Option<ir::MethodDefinition> {
        let ((params, visited_body), type_params) = self.with_type_params(&node.type_params, |s| {
            let params = s.visit_function_params(node.params);
            let visited_body = s.visit_function_body(node.return_type, node.body);
            (params, visited_body)
        });
        let (_, host_symbol) = host?;

        let (params, (return_type, body)) = (params?, visited_body?);
        let param_types = params
            .iter()
            .map(|(_, s)| self.symbol_type_id(*s))
            .collect::<Vec<_>>();

        let ty = self.intern(types::FunctionType {
            type_params,
            params: param_types,
            return_type,
        });

        let name = node.name?;
        if self.has_member(host_symbol, &name.text) {
            let error = DiagnosticKind::DuplicateFieldName { name: name.text };
            self.error(error, name.loc);
            return None;
        }
        let receiver = if node.receiver.mutable {
            MethodReceiverKind::Mutable
        } else {
            MethodReceiverKind::Immutable
        };
        let method_symbol = MethodSymbol {
            name: name.text,
            owner: host_symbol,
            owner_args: host_args.clone().into(),
            receiver,
            param_names: params
                .iter()
                .map(|p| self.symbol_name(p.1).to_string())
                .collect(),
            ty,
            docs: node.docs.map(|d| d.text),
            defined_at: name.loc,
            ..Default::default()
        };
        let symbol = self.attach_method(method_symbol, host_symbol, name.loc)?;

        Some(ir::MethodDefinition {
            loc: node.loc,
            receiver: host?,
            mutating: node.receiver.mutable,
            name: (name.loc, symbol),
            params,
            body,
            ty,
        })
    }

    fn has_member(&self, symbol: TypeSymbolId, field: &str) -> bool {
        if self.has_method(symbol, field) {
            return true;
        }
        match symbol {
            TypeSymbolId::Enum(_) => false,
            TypeSymbolId::Struct(s) => match &self.symbols.get(s).body {
                TypeSymbolBody::Struct(s) => s.iter().find(|(n, _)| n == field).is_some(),
                _ => false,
            },
        }
    }

    fn has_method(&self, symbol: TypeSymbolId, field: &str) -> bool {
        match symbol {
            TypeSymbolId::Enum(s) => self
                .symbols
                .get(s)
                .methods
                .iter()
                .find(|m| self.symbol_name(**m) == field)
                .is_some(),
            TypeSymbolId::Struct(s) => self
                .symbols
                .get(s)
                .methods
                .iter()
                .find(|m| self.symbol_name(**m) == field)
                .is_some(),
        }
    }

    fn visit_static_definition(
        &mut self,
        node: ast::FunctionDefinition,
        host: Option<(Location, TypeSymbolId)>,
        host_args: &Substitutions,
    ) -> Option<ir::FunctionDefinition> {
        let ((params, visited_body), type_params) =
            self.with_type_params(&node.definition.type_params, |s| {
                let params = s.visit_function_params(node.definition.params);
                let visited_body =
                    s.visit_function_body(node.definition.return_type, node.definition.body);
                (params, visited_body)
            });

        let (_, host_symbol) = host?;
        let (params, (return_type, body)) = (params?, visited_body?);

        let ty = self.intern(types::FunctionType {
            type_params,
            params: params
                .iter()
                .map(|p| self.symbol_type_id(p.1))
                .collect::<Vec<_>>(),
            return_type,
        });

        let name = node.definition.name?;
        let method_symbol = MethodSymbol {
            name: name.text,
            owner: host_symbol,
            owner_args: host_args.clone().into(),
            receiver: MethodReceiverKind::Static,
            param_names: params
                .iter()
                .map(|p| self.symbol_name(p.1).to_string())
                .collect(),
            ty,
            docs: node.docs.map(|d| d.text),
            defined_at: name.loc,
            ..Default::default()
        };
        let symbol = self.attach_method(method_symbol, host_symbol, name.loc)?;

        Some(ir::FunctionDefinition {
            loc: node.definition.loc,
            name: (name.loc, symbol.into()),
            params,
            body,
            ty,
        })
    }

    fn attach_method(
        &mut self,
        symbol: MethodSymbol,
        host: TypeSymbolId,
        at: Location,
    ) -> Option<MethodSymbolId> {
        if self.has_method(host, &symbol.name) {
            let name = symbol.name;
            self.error(DiagnosticKind::DuplicateMethodName { name }, at);
            return None;
        }

        self.add_method_to_store(&symbol);
        let symbol_id = self.symbols.insert(symbol);
        match host {
            TypeSymbolId::Enum(s) => self.symbols.get_mut(s).methods.push(symbol_id),
            TypeSymbolId::Struct(s) => self.symbols.get_mut(s).methods.push(symbol_id),
        }
        Some(symbol_id)
    }
    fn add_method_to_store(&mut self, symbol: &MethodSymbol) {
        if symbol.is_static() {
            return;
        }
        debug_assert!(matches!(self.resolve(symbol.ty), types::Type::Function(_)));
        self.types.add_method(
            self.symbol_type_id(symbol.owner),
            TraitMethod {
                name: symbol.name.clone(),
                def: symbol.ty,
            },
        );
    }

    fn fallback_check_impl_body(&mut self, body: Option<ImplementationBody>) {
        body.map_or(vec![], |body| body.items)
            .into_iter()
            .for_each(|item| {
                self.visit_impl_item(None, &Substitutions::new(), item);
            });
    }
}
