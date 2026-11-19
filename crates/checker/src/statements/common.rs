use tine_ast as ast;
use tine_common::diagnostics::DiagnosticKind;
use tine_types::types::{self, GenericDef};

use crate::TypeChecker;

impl TypeChecker {
    pub fn check_duplicate_identifier(&mut self, ident: &ast::Identifier) -> bool {
        if !self.current_scope().has(ident.as_str()) {
            return false;
        }
        let name = ident.text.clone();
        let error = DiagnosticKind::DuplicateIdentifier { name };
        self.error(error, ident.loc);
        true
    }

    pub fn declare_temporary_type(
        &mut self,
        ident: &ast::Identifier,
        params: &[types::TypeParam],
    ) -> types::TypeId {
        let param = self.declare_type_param(ident.clone());
        self.types.overwrite(
            param.id,
            GenericDef {
                def: param.id,
                params: params.to_vec(),
            },
        );
        param.id
    }
}
