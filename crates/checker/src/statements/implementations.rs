use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

use tine_ast as ast;
use tine_common::{diagnostics::DiagnosticKind, locations::Location};
use tine_ir as ir;
use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

use crate::{
    substitutions::{SubstitutionTable, Substitutions},
    utils::return_last,
    TypeChecker,
};

#[derive(Clone, Copy)]
pub struct PtrKey<T: ?Sized>(*const T);
impl<T: ?Sized> Hash for PtrKey<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
impl<T: ?Sized> PartialEq for PtrKey<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}
impl<T: ?Sized> Eq for PtrKey<T> {}
pub type MethodMap = HashMap<PtrKey<ast::MethodDefinition>, MethodSymbolId>;

impl TypeChecker {
    pub(super) fn visit_implementation(&mut self, node: ast::Implementation) -> Vec<ir::Statement> {
        let Some((host, actual_type)) = self.visit_impl_host(node.implemented_type) else {
            return vec![];
        };
        let mut tc = self.with_this(actual_type);

        let substitutions = tc.get_host_substitutions(host, actual_type);

        let Some(body) = node.body else {
            return vec![];
        };

        let methods = tc.infer_method_symbols(&body.items, host, substitutions.table());
        tc.symbol_methods_mut(host)
            .extend(methods.values().copied());

        tc.visit_methods(&body.items, methods)
            .into_iter()
            .map(Into::into)
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
    ) -> Option<(TypeSymbolId, types::TypeId)> {
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

        Some((symbol, receiver_type))
    }

    /// If the actual type is a concrete instance of the host's symbol type,
    /// returns the needed type substitutions.
    fn get_host_substitutions(
        &self,
        host: TypeSymbolId,
        actual_type: types::TypeId,
    ) -> Substitutions {
        let owner_type = self.symbol_type(host);
        let type_params = owner_type.as_params().unwrap_or(&[]);
        let type_args = match self.resolve(actual_type) {
            types::Type::Ref(r) => r.args,
            _ => vec![],
        };
        Substitutions::with_initial(type_params, &type_args)
    }

    pub fn infer_method_symbols(
        &mut self,
        nodes: &[ast::MethodDefinition],
        owner: TypeSymbolId,
        owner_args: &SubstitutionTable,
    ) -> MethodMap {
        nodes
            .into_iter()
            .filter_map(|i| Some((PtrKey(i), self.infer_method_symbol(i, owner, owner_args)?)))
            .collect()
    }

    pub fn infer_method_symbol(
        &mut self,
        node: &ast::MethodDefinition,
        owner: TypeSymbolId,
        owner_args: &SubstitutionTable,
    ) -> Option<MethodSymbolId> {
        let ident = node.name.as_ref()?;
        let name = ident.as_str().to_string();
        let receiver = if node.static_ {
            MethodReceiverKind::Static
        } else if node.mut_ {
            MethodReceiverKind::Mutable
        } else {
            MethodReceiverKind::Immutable
        };
        let type_params = node
            .type_params
            .iter()
            .flatten()
            .map(|p| self.add_type_param(p.as_str().into()))
            .collect();
        let (param_names, param_types) = node
            .params
            .iter()
            .map(|p| p.params.iter())
            .flatten()
            .map(|p| self.infer_method_param(p))
            .unzip();
        let return_type = self.infer_method_return_type(node);
        let ty = self.intern(types::FunctionType {
            type_params,
            params: param_types,
            return_type,
        });
        let duplicated = self
            .symbols
            .find::<MethodSymbolId, _>(|s| {
                s.name == name && s.owner == owner && &s.owner_args == owner_args
            })
            .is_some();
        if duplicated {
            let kind = DiagnosticKind::DuplicateMethodName { name: name.clone() };
            self.error(kind, ident.loc);
        }
        Some(self.insert(MethodSymbol {
            name,
            public: node.public,
            owner,
            owner_args: owner_args.clone(),
            receiver,
            param_names,
            ty,
            docs: node.docs.as_ref().map(|d| d.text.clone()),
            defined_at: ident.loc,
            ..Default::default()
        }))
    }
    fn infer_method_param(&mut self, param: &ast::FunctionParam) -> (String, types::TypeId) {
        let name = param.name.as_ref().map_or("", |n| n.as_str()).to_string();
        let ty = param
            .type_annotation
            .as_ref()
            .map_or(TypeStore::UNKNOWN, |t| self.visit_type(t.clone()));
        (name, ty)
    }
    fn infer_method_return_type(&mut self, node: &ast::MethodDefinition) -> types::TypeId {
        node.return_type
            .as_ref()
            .map_or(TypeStore::UNIT, |t| self.visit_type(t.clone()))
    }

    pub fn visit_methods(
        &mut self,
        nodes: &[ast::MethodDefinition],
        methods: MethodMap,
    ) -> Vec<ir::MethodDefinition> {
        nodes
            .into_iter()
            .filter_map(|item| self.visit_method(item.clone(), *methods.get(&PtrKey(item))?))
            .map(Into::into)
            .collect()
    }

    pub fn visit_method(
        &mut self,
        node: ast::MethodDefinition,
        symbol_id: MethodSymbolId,
    ) -> Option<ir::MethodDefinition> {
        let mut self_ = self.with_this_mutability(node.mut_);
        let mut self_ = self_.with_local_scope();
        let symbol = self_.symbols.get(symbol_id);
        let ty_id = symbol.ty;
        let ty = self_
            .resolve(symbol.ty)
            .as_function()
            .expect("this should be a function!!")
            .clone();

        self_.handle_method_type_params(&ty.type_params, node.type_params);
        let params = self_.handle_method_params(&ty.params, node.params);
        let body = self_.handle_method_body(node.body, ty.return_type);

        Some(ir::MethodDefinition {
            ty: ty_id,
            loc: node.loc,
            mutating: node.mut_,
            name: (node.name?.loc, symbol_id),
            params,
            body: body?,
        })
    }

    fn handle_method_type_params(
        &mut self,
        types: &Vec<types::TypeParam>,
        nodes: Option<Vec<ast::Identifier>>,
    ) {
        types
            .into_iter()
            .zip(nodes.unwrap_or(vec![]))
            .for_each(|(ty, node)| {
                self.insert::<TypeAliasSymbolId>(TypeAliasSymbol {
                    name: ty.name.clone(),
                    ty: ty.id,
                    defined_at: node.loc,
                    ..Default::default()
                });
            });
    }

    fn handle_method_params(
        &mut self,
        types: &[types::TypeId],
        nodes: Option<ast::FunctionParams>,
    ) -> Vec<(Location, VariableSymbolId)> {
        let nodes = nodes.into_iter().map(|p| p.params).flatten();
        nodes
            .zip(types)
            .filter_map(|(n, &ty)| {
                let name = n.name?;
                let sym = self.insert(VariableSymbol {
                    name: name.as_str().into(),
                    ty,
                    defined_at: name.loc,
                    ..Default::default()
                });
                Some((n.loc, sym))
            })
            .collect()
    }

    fn handle_method_body(
        &mut self,
        node: Option<Box<ast::Expression>>,
        expected_return: types::TypeId,
    ) -> Option<ir::Block> {
        let mut body: ir::Block = self.visit_expression(*node?)?.into();
        self.check_function_body_type(&body, expected_return);
        return_last(&mut body);
        Some(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_no_return_type() {
        let mut checker = TypeChecker::new();
        let node = ast::MethodDefinition {
            return_type: None,
            ..Default::default()
        };
        let ty = checker.infer_method_return_type(&node);
        assert_eq!(ty, TypeStore::UNIT);
    }

    #[test]
    fn infer_return_type() {
        let mut checker = TypeChecker::new();
        let node = ast::MethodDefinition {
            return_type: Some(ast::Type::Named(
                ast::Identifier::new("int".into(), Location::dummy()).into(),
            )),
            ..Default::default()
        };
        let ty = checker.infer_method_return_type(&node);
        assert_eq!(ty, TypeStore::INTEGER);
    }
}
