use tine_ast as ast;
use tine_common::diagnostics::DiagnosticKind;
use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

use crate::TypeChecker;

impl TypeChecker {
    pub(crate) fn visit_trait_definition(&mut self, node: ast::TraitDefinition) {
        let (methods, type_params) = self.with_type_params(&node.params, |self_| {
            node.methods.and_then(|m| self_.check_method_signatures(m))
        });

        let Some(name) = node.name else {
            return;
        };
        if self.current_scope().has(name.as_str()) {
            let error = DiagnosticKind::DuplicateIdentifier { name: name.text };
            self.error(error, name.loc);
            return;
        }

        let Some(methods) = methods else {
            return;
        };

        let param_names = type_params.iter().map(|p| p.name.clone()).collect();
        let ty = self.intern(types::TraitType {
            params: type_params,
            methods,
        });

        self.symbols.insert::<TraitSymbolId>(TraitSymbol {
            name: name.text,
            public: node.public,
            param_names,
            ty,
            docs: node.docs.map(|d| d.text),
            defined_at: name.loc,
            ..Default::default()
        });
    }

    fn check_method_signatures(
        &mut self,
        methods: Vec<ast::TraitMethod>,
    ) -> Option<Vec<types::TraitMethod>> {
        methods
            .into_iter()
            .map(|m| self.check_method_signature(m))
            .collect()
    }

    fn check_method_signature(&mut self, node: ast::TraitMethod) -> Option<types::TraitMethod> {
        let name = node.name?.text;

        let has_receiver = node.receiver.is_some();
        let type_params = type_params(node.receiver, node.type_params);

        let ((params, return_type), type_params) = self.with_type_params(&type_params, |self_| {
            let params = self_.visit_function_params(node.params, None);
            let return_type = node
                .return_annotation
                .map_or(TypeStore::UNIT, |ann| self_.visit_type(ann));
            (params, return_type)
        });

        let (self_type, type_params) = match has_receiver {
            true => {
                let mut type_params = type_params;
                let tail = type_params.split_off(1);
                let head = type_params.pop().unwrap();
                (Some(head), tail)
            }
            false => (None, type_params),
        };

        let params = params?
            .into_iter()
            .map(|(_, s)| self.symbols.get(s).ty)
            .collect();
        let def = self.intern(types::FunctionType {
            type_params,
            params,
            return_type,
        });

        Some(types::TraitMethod {
            self_type,
            name,
            def,
        })
    }
}

fn type_params(
    receiver: Option<ast::Identifier>,
    type_params: Option<Vec<ast::Identifier>>,
) -> Option<Vec<ast::Identifier>> {
    match (receiver, type_params) {
        (None, None) => None,
        (Some(receiver), None) => Some(vec![receiver]),
        (None, Some(type_params)) => Some(type_params),
        (Some(receiver), Some(mut type_params)) => {
            type_params.insert(0, receiver);
            Some(type_params)
        }
    }
}
