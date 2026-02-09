use pest::iterators::Pair;

use crate::{
    ast,
    diagnostics::DiagnosticKind,
    parser::{parser::Rule, utils::is_pascal_case, ParserEngine},
};

impl ParserEngine {
    pub(super) fn parse_type_params(&mut self, pair: Pair<'_, Rule>) -> Vec<String> {
        debug_assert_eq!(pair.as_rule(), Rule::type_params);
        let mut type_params = Vec::new();
        let mut type_param_names = std::collections::HashSet::new();
        let inner = pair.into_inner();
        for param_pair in inner {
            let param_name = self.parse_type_param(&param_pair);
            if !type_param_names.insert(param_name.clone()) {
                let error = DiagnosticKind::DuplicateIdentifier {
                    name: param_name.clone(),
                };
                let loc = self.localize(param_pair.as_span());
                self.error(error, loc);
            }
            type_params.push(param_name);
        }
        type_params
    }

    fn parse_type_param(&mut self, pair: &Pair<'_, Rule>) -> String {
        debug_assert_eq!(pair.as_rule(), Rule::type_identifier);
        let param_name = pair.as_str().to_string();
        if !is_pascal_case(&param_name) {
            let error = DiagnosticKind::InvalidTypeName;
            let loc = self.localize(pair.as_span());
            self.error(error, loc);
        }
        param_name
    }

    pub(super) fn parse_type_body(&mut self, pair: Pair<'_, Rule>) -> ast::TypeBody {
        debug_assert_eq!(pair.as_rule(), Rule::type_body);
        let inner = pair.into_inner().next().unwrap();
        match inner.as_rule() {
            Rule::struct_body => self.parse_struct_body(inner).into(),
            Rule::tuple_body => self.parse_tuple_body(inner).into(),
            _ => unreachable!(),
        }
    }

    fn parse_struct_body(&mut self, pair: Pair<'_, Rule>) -> ast::StructBody {
        debug_assert_eq!(pair.as_rule(), Rule::struct_body);
        let loc = self.localize(pair.as_span());

        let fields: Vec<ast::StructDefinitionField> = pair
            .into_inner()
            .map(|pair| self.parse_struct_field(pair))
            .collect();

        let mut field_names = std::collections::HashSet::new();
        fields.iter().filter_map(|f| f.as_name()).for_each(|name| {
            if !field_names.insert(name.text.clone()) {
                let error = DiagnosticKind::DuplicateFieldName {
                    name: name.text.clone(),
                };
                self.error(error, name.loc);
            }
        });

        ast::StructBody { loc, fields }
    }

    fn parse_struct_field(&mut self, pair: Pair<'_, Rule>) -> ast::StructDefinitionField {
        assert_eq!(pair.as_rule(), Rule::field_declaration);
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::mandatory_field => self.parse_mandatory_field(inner).into(),
            Rule::optional_field => self.parse_optionnal_field(inner).into(),
            _ => unreachable!(),
        }
    }

    fn parse_mandatory_field(&mut self, pair: Pair<'_, Rule>) -> ast::StructMandatoryField {
        assert_eq!(pair.as_rule(), Rule::mandatory_field);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();

        let name = Some(self.parse_identifier(inner.next().unwrap()));
        let definition = Some(self.parse_type(inner.next().unwrap()));

        ast::StructMandatoryField {
            loc,
            name,
            definition,
        }
    }

    fn parse_optionnal_field(&mut self, pair: Pair<'_, Rule>) -> ast::StructOptionalField {
        assert_eq!(pair.as_rule(), Rule::mandatory_field);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();

        let name = Some(self.parse_identifier(inner.next().unwrap()));
        let default = Some(self.parse_expression(inner.next().unwrap()));

        ast::StructOptionalField { loc, name, default }
    }

    fn parse_tuple_body(&mut self, pair: Pair<'_, Rule>) -> ast::TupleType {
        debug_assert_eq!(pair.as_rule(), Rule::tuple_body);
        let loc = self.localize(pair.as_span());
        let elements: Vec<ast::Type> = pair
            .into_inner()
            .map(|pair| self.parse_type(pair))
            .collect();
        ast::TupleType { loc, elements }
    }
}
