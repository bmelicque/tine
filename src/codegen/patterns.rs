use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::ast;

use super::{utils::create_ident, CodeGenerator};

impl CodeGenerator {
    pub fn pattern_to_swc(&mut self, node: ast::Pattern) -> swc::Pat {
        match node {
            ast::Pattern::Identifier(pattern) => swc::Pat::Ident(swc::BindingIdent {
                id: create_ident(pattern.span.as_str()),
                type_ann: None,
            }),
            ast::Pattern::StructPattern(pattern) => self.struct_pattern_to_swc(pattern).into(),
            ast::Pattern::Tuple(pattern) => self.tuple_pattern_to_swc(pattern).into(),
        }
    }

    fn struct_pattern_to_swc(&mut self, node: ast::StructPattern) -> swc::ObjectPat {
        swc::ObjectPat {
            span: DUMMY_SP,
            props: node
                .fields
                .into_iter()
                .map(|field| self.struct_pattern_field_to_swc(field))
                .collect(),
            optional: false,
            type_ann: None,
        }
    }

    fn struct_pattern_field_to_swc(&mut self, node: ast::StructPatternField) -> swc::ObjectPatProp {
        match node.pattern {
            Some(pattern) => swc::KeyValuePatProp {
                key: create_ident(&node.identifier).into(),
                value: Box::new(self.pattern_to_swc(pattern)),
            }
            .into(),
            None => swc::AssignPatProp {
                span: DUMMY_SP,
                key: create_ident(&node.identifier).into(),
                value: None,
            }
            .into(),
        }
    }

    fn tuple_pattern_to_swc(&mut self, node: ast::TuplePattern) -> swc::ArrayPat {
        swc::ArrayPat {
            span: DUMMY_SP,
            elems: node
                .elements
                .into_iter()
                .map(|element| Some(self.pattern_to_swc(element)))
                .collect(),
            optional: false,
            type_ann: None,
        }
    }
}
