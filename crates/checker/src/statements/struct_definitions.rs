use tine_ast as ast;
use tine_common::diagnostics::DiagnosticKind;
use tine_ir as ir;
use tine_symbols::symbols::*;
use tine_types::types;

use crate::{substitutions::SubstitutionTable, TypeChecker};

impl TypeChecker {
    pub fn visit_struct_definition(&mut self, node: ast::StructDefinition) -> Vec<ir::Statement> {
        let Some(name) = node.name else {
            self.fallback_check_struct_body(node.body);
            return vec![];
        };
        if self.current_scope().has(name.as_str()) {
            let error = DiagnosticKind::DuplicateIdentifier { name: name.text };
            self.error(error, name.loc);
            self.fallback_check_struct_body(node.body);
            return vec![];
        }
        let owner_id: StructSymbolId = self.insert(StructSymbol {
            name: name.text.clone(),
            defined_at: name.loc,
            ..Default::default()
        });
        let Some(body) = node.body else {
            let def = ir::StructDefinition {
                loc: node.loc,
                symbol: owner_id,
            };
            return vec![def.into()];
        };

        let tc = &mut self.with_this(owner_id.into()).tc;

        let (fields, method_nodes) = split_struct_body(body);
        let ((members, methods), params) = tc.with_type_params(&node.params, |self_| {
            let members = fields
                .into_iter()
                .filter_map(|f| self_.visit_struct_definition_field(owner_id.into(), f))
                .collect::<Vec<_>>();
            let methods = self_.infer_method_symbols(
                &method_nodes,
                owner_id.into(),
                &SubstitutionTable::new(),
            );
            let symbol = self_.symbols.get_mut(owner_id);
            symbol.members = members.clone();
            symbol.methods.extend(methods.values().copied());
            (members, methods)
        });

        let ty = tc.get_struct_type(&members, params);
        let owner = tc.symbols.get_mut(owner_id);
        owner.ty = ty;
        tc.types.add_alias(ty, name.text);

        let def = ir::StructDefinition {
            loc: node.loc,
            symbol: owner_id.into(),
        };
        let mut stmts: Vec<ir::Statement> = tc
            .visit_methods(&method_nodes, methods)
            .into_iter()
            .map(Into::into)
            .collect();
        stmts.insert(0, def.into());
        stmts
    }

    fn visit_struct_definition_field(
        &mut self,
        owner: TypeSymbolId,
        field: ast::StructDefinitionField,
    ) -> Option<MemberSymbolId> {
        let ty = self.visit_type(field.definition?);
        let name = field.name?;
        Some(self.symbols.insert(MemberSymbol {
            name: name.text,
            public: field.public,
            ty,
            owner,
            defined_at: name.loc,
            ..Default::default()
        }))
    }

    fn get_struct_type(
        &mut self,
        members: &[MemberSymbolId],
        params: Vec<types::TypeParam>,
    ) -> types::TypeId {
        let fields = members
            .into_iter()
            .map(|m| self.member_symbol_to_struct_field(*m))
            .collect();
        self.intern_unique(types::StructType {
            params,
            fields,
            ..Default::default()
        })
    }
    fn member_symbol_to_struct_field(&mut self, m: MemberSymbolId) -> types::StructField {
        types::StructField {
            name: self.symbol_name(m).to_string(),
            def: self.symbol_type_id(m),
        }
    }

    fn fallback_check_struct_body(&mut self, body: Option<Vec<ast::StructItem>>) {
        let Some(body) = body else { return };
        body.into_iter().for_each(|def| {
            self.fallback_check_struct_item(def);
        })
    }

    fn fallback_check_struct_item(&mut self, item: ast::StructItem) {
        match item {
            ast::StructItem::Field(f) => {
                f.definition.map(|d| self.visit_type(d));
            }
            ast::StructItem::Method(_) => {}
        }
    }
}

fn split_struct_body(
    body: Vec<ast::StructItem>,
) -> (Vec<ast::StructDefinitionField>, Vec<ast::MethodDefinition>) {
    let mut fields = Vec::new();
    let mut methods = Vec::new();

    for item in body {
        match item {
            ast::StructItem::Field(f) => fields.push(f),
            ast::StructItem::Method(m) => methods.push(m),
        }
    }

    (fields, methods)
}
