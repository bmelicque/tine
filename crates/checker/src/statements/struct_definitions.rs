use tine_ast as ast;
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
        if self.check_duplicate_identifier(&name) {
            self.fallback_check_struct_body(node.body);
            return vec![];
        }
        let owner_id: StructSymbolId = self.insert(StructSymbol {
            public: node.public,
            docs: node.docs.map(|d| d.text),
            name: name.text.clone(),
            defined_at: name.loc,
            ..Default::default()
        });
        let Some(body) = node.body else {
            let loc = node.loc;
            let symbol = owner_id;
            let def = ir::StructDefinition { loc, symbol };
            return vec![def.into()];
        };

        let (fields, method_nodes) = split_struct_body(body);
        let (mut self_, type_params) = self.with_type_params2(&node.params);

        // Fields
        let this = self_.declare_temporary_type(&name, &type_params);
        let members = self_.visit_struct_definition_fields(fields, owner_id.into());
        let ty = self_.get_struct_type(&members, type_params, this);
        self_.symbols.get_mut(owner_id).ty = ty;
        self_.types.add_alias(ty, name.text);

        // Methods (shallow)
        let mut self_ = self_.with_this(ty);
        let methods =
            self_.infer_method_symbols(&method_nodes, owner_id.into(), &SubstitutionTable::new());
        let symbol = self_.symbols.get_mut(owner_id);
        symbol.members = members.clone();
        symbol.methods.extend(methods.values().copied());

        // Methods (deep)
        let mut stmts: Vec<ir::Statement> = self_
            .visit_methods(&method_nodes, methods)
            .into_iter()
            .map(Into::into)
            .collect();

        stmts.insert(0, ir::StructDefinition::new(owner_id, node.loc).into());
        stmts
    }

    fn visit_struct_definition_fields(
        &mut self,
        fields: Vec<ast::StructDefinitionField>,
        owner: TypeSymbolId,
    ) -> Vec<MemberSymbolId> {
        fields
            .into_iter()
            .filter_map(|f| self.visit_struct_definition_field(owner, f))
            .collect::<Vec<_>>()
    }

    fn visit_struct_definition_field(
        &mut self,
        owner: TypeSymbolId,
        field: ast::StructDefinitionField,
    ) -> Option<MemberSymbolId> {
        let ty = self.visit_type(field.definition?);
        let name = field.name?;
        Some(self.symbols.insert(MemberSymbol {
            docs: field.docs.map(|d| d.text),
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
        placeholder: types::TypeId,
    ) -> types::TypeId {
        let fields = members
            .into_iter()
            .map(|m| self.member_symbol_to_struct_field(*m))
            .collect();
        self.types.overwrite(
            placeholder,
            types::StructType {
                params,
                fields,
                ..Default::default()
            },
        );
        placeholder
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

#[cfg(test)]
mod tests {
    use tine_common::locations::Location;

    use super::*;

    #[test]
    fn visit_self_ref_method() {
        let mut checker = TypeChecker::new();
        let name = ast::Identifier::new("Struct".into(), Location::dummy());
        let node = ast::StructDefinition {
            name: Some(name.clone()),
            body: Some(vec![ast::StructItem::Method(ast::MethodDefinition {
                name: Some(ast::Identifier::new("method".into(), Location::dummy())),
                return_type: Some(ast::Type::Named(name.into())),
                ..Default::default()
            })]),
            ..Default::default()
        };
        checker.visit_struct_definition(node);
        assert!(checker.diagnostics.is_empty());
    }
}
