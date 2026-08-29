use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_ir as ir;
use tine_symbols::symbols::VariantSymbolId;

use crate::codegen::{
    statements::types::enums::TAG_SYMBOL,
    utils::{create_num, ident_from_str, index, member},
    CodeGenerator,
};

pub struct PatternResult {
    pub test: Option<swc::Expr>,
    pub decl: Option<swc::Pat>,
}

impl CodeGenerator<'_, '_> {
    /// `against` should be clonable! no calls, because they might end up evaluated twice.
    pub fn handle_pattern(&mut self, pattern: ir::Pattern, against: swc::Expr) -> PatternResult {
        use ir::Pattern::*;
        match pattern {
            Boolean(p) => self.handle_boolean_pattern(p, against),
            Float(p) => self.handle_float_pattern(p, against),
            Integer(p) => self.handle_int_pattern(p, against),
            String(p) => self.handle_string_pattern(p, against),

            Call(p) => self.handle_call_pattern(p, against),
            Identifier(p) => self.handle_identifier_pattern(p),
            Struct(p) => self.handle_struct_pattern(p, against),
            Tuple(p) => self.handle_tuple_pattern(p, against),
        }
    }

    fn handle_primitive_pattern(
        &mut self,
        pattern: swc::Expr,
        against: swc::Expr,
    ) -> PatternResult {
        let test = Some(swc::Expr::Bin(swc::BinExpr {
            span: DUMMY_SP,
            op: swc::BinaryOp::EqEqEq,
            left: Box::new(against),
            right: Box::new(pattern),
        }));
        PatternResult { test, decl: None }
    }
    fn handle_boolean_pattern(
        &mut self,
        pattern: ir::BooleanLiteral,
        against: swc::Expr,
    ) -> PatternResult {
        let right = self.handle_boolean_literal(pattern);
        self.handle_primitive_pattern(right, against)
    }
    fn handle_float_pattern(
        &mut self,
        pattern: ir::FloatLiteral,
        against: swc::Expr,
    ) -> PatternResult {
        let right = self.handle_float_literal(pattern);
        self.handle_primitive_pattern(right, against)
    }
    fn handle_int_pattern(&mut self, pattern: ir::IntLiteral, against: swc::Expr) -> PatternResult {
        let right = self.handle_int_literal(pattern);
        self.handle_primitive_pattern(right, against)
    }
    fn handle_string_pattern(
        &mut self,
        pattern: ir::StringLiteral,
        against: swc::Expr,
    ) -> PatternResult {
        let right = self.handle_string_literal(pattern).into();
        self.handle_primitive_pattern(right, against)
    }

    fn handle_call_pattern(
        &mut self,
        pattern: ir::CallPattern,
        against: swc::Expr,
    ) -> PatternResult {
        let (mut tests, decls): (Vec<_>, Vec<_>) = pattern
            .arguments
            .into_iter()
            .enumerate()
            .map(|(i, e)| self.handle_pattern(e, member(against.clone(), &format!("_{i}")).into()))
            .map(|r| (r.test, r.decl))
            .unzip();

        let variant = self.get_variant_id(pattern.callee.1);
        let variant_test = swc::Expr::Bin(swc::BinExpr {
            span: DUMMY_SP,
            op: swc::BinaryOp::EqEqEq,
            left: Box::new(member(against, TAG_SYMBOL).into()),
            right: Box::new(create_num(variant as f64)),
        });
        tests.insert(0, Some(variant_test));

        let test = merge_tests(tests);
        let decl = decls
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .map(pats_to_call_body)
            .map(Into::into);

        PatternResult { test, decl }
    }
    fn get_variant_id(&self, variant: VariantSymbolId) -> usize {
        let owner = self.symbols.get(variant).owner;
        let variants = &self.symbols.get(owner).variants;
        variants.iter().position(|&v| v == variant).unwrap()
    }

    fn handle_identifier_pattern(&mut self, pattern: ir::Identifier) -> PatternResult {
        let test = None;
        let name = self.symbols.get_symbol(pattern.symbol).name();
        let decl = Some(ident_from_str(name).into());
        PatternResult { test, decl }
    }

    fn handle_struct_pattern(
        &mut self,
        pattern: ir::StructPattern,
        against: swc::Expr,
    ) -> PatternResult {
        let (tests, fields): (Vec<_>, Vec<_>) = pattern
            .fields
            .into_iter()
            .map(|field| self.handle_struct_pattern_field(field, against.clone()))
            .unzip();
        let test = merge_tests(tests);
        let decl = fields
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .map(|props| swc::Pat::Object(props_to_object(props)));
        PatternResult { test, decl }
    }
    fn handle_struct_pattern_field(
        &mut self,
        field: ir::StructPatternField,
        against_object: swc::Expr,
    ) -> (Option<swc::Expr>, Option<swc::ObjectPatProp>) {
        let name = self.symbol_name(field.identifier.symbol).to_owned();
        let pat = self.handle_pattern(field.pattern, member(against_object, &name).into());
        let prop = pat.decl.map(|value| key_value(&name, value).into());
        (pat.test, prop)
    }

    fn handle_tuple_pattern(
        &mut self,
        pattern: ir::TuplePattern,
        against: swc::Expr,
    ) -> PatternResult {
        let (tests, decls): (Vec<_>, Vec<_>) = pattern
            .elements
            .into_iter()
            .enumerate()
            .map(|(i, e)| self.handle_pattern(e, index(against.clone(), i).into()))
            .map(|r| (r.test, r.decl))
            .unzip();

        let test = merge_tests(tests);
        let decl = decls
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .map(|pats| swc::Pat::Array(pats_to_array(pats)));
        PatternResult { test, decl }
    }
}

fn merge_tests(tests: Vec<Option<swc::Expr>>) -> Option<swc::Expr> {
    tests.into_iter().flatten().reduce(land)
}
fn land(left: swc::Expr, right: swc::Expr) -> swc::Expr {
    swc::Expr::Bin(swc::BinExpr {
        span: DUMMY_SP,
        op: swc::BinaryOp::LogicalAnd,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn key_value(key: &str, value: swc::Pat) -> swc::KeyValuePatProp {
    swc::KeyValuePatProp {
        key: swc::PropName::Ident(ident_from_str(key).into()),
        value: Box::new(value),
    }
}

fn pats_to_call_body(pats: Vec<swc::Pat>) -> swc::ObjectPat {
    let props = pats
        .into_iter()
        .enumerate()
        .map(|(i, p)| key_value(&format!("_{i}"), p).into())
        .collect::<Vec<_>>();
    props_to_object(props)
}

fn props_to_object(props: Vec<swc::ObjectPatProp>) -> swc::ObjectPat {
    swc::ObjectPat {
        span: DUMMY_SP,
        props,
        optional: false,
        type_ann: None,
    }
}
fn pats_to_array(pats: Vec<swc::Pat>) -> swc::ArrayPat {
    swc::ArrayPat {
        span: DUMMY_SP,
        elems: pats.into_iter().map(Some).collect(),
        optional: false,
        type_ann: None,
    }
}
