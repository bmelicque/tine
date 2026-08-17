mod binary;
mod block;
mod dom;
mod ifs;
mod unary;
mod utils;

use super::{utils::ident_from_str, CodeGenerator};
use crate::{
    codegen::{
        expressions::utils::{assign_if_last_expressions, ident_to_declaration},
        statements::types::enums::TAG_SYMBOL,
        utils::{create_block_stmt, create_str, internal_construct, internal_method_call},
    },
    ownership_analyser::OwnershipAction,
};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_checker::substitutions::Substitutions;
use tine_common::locations::Location;
use tine_ir as ir;
use tine_symbols::symbols::*;
use tine_types::types;

pub struct ExpressionResult {
    /// All the the statements needed to be ran before actually evaluating the expression
    pub prelim_stmts: Vec<swc::Stmt>,
    /// The actual expression
    pub expr: swc::Expr,
}
impl ExpressionResult {
    pub fn map<F>(mut self, f: F) -> Self
    where
        F: FnOnce(swc::Expr) -> swc::Expr,
    {
        self.expr = f(self.expr);
        self
    }
}
impl<T> From<T> for ExpressionResult
where
    T: Into<swc::Expr>,
{
    fn from(value: T) -> Self {
        Self {
            prelim_stmts: vec![],
            expr: value.into(),
        }
    }
}

impl CodeGenerator<'_, '_> {
    pub fn handle_expression(&mut self, node: ir::Expression) -> ExpressionResult {
        use ir::Expression::*;
        match node {
            Array(a) => self.handle_array(a.elements),
            Binary(b) => self.handle_binary_expression(b),
            BooleanLiteral(b) => ExpressionResult::from(self.handle_boolean_literal(b)),
            Block(b) => self.handle_block(b),
            Call(c) => self.handle_call(c),
            Element(e) => self.handle_element_expression(e),
            FloatLiteral(f) => ExpressionResult::from(self.handle_float_literal(f)),
            For(f) => self.handle_for_expression(f),
            ForIn(f) => self.handle_for_in_expression(f),
            Function(f) => self.handle_function_expression(f).into(),
            Identifier(i) => self.handle_identifier(i).into(),
            If(i) => self.handle_if_expression(i),
            Index(i) => self.handle_index_expression(i),
            IntLiteral(i) => ExpressionResult::from(self.handle_int_literal(i)),
            IntrinsicCall(i) => self.handle_intrinsic_call(i),
            IntrinsicConstruct(i) => self.handle_intrinsic_construct(i),
            Match(m) => self.handle_match_expression(m),
            Member(m) => self.handle_member_expression(m),
            Method(m) => self.handle_method(m),
            StringLiteral(s) => self.handle_string_literal(s).into(),
            Struct(s) => self.handle_simple_struct(s),
            Unary(u) => self.handle_unary_expression(u),
            Tuple(t) => self.handle_array(t.elements),
            TypeMatch(t) => self.handle_type_match(t),
        }
    }

    pub fn handle_array(&mut self, elements: Vec<ir::Expression>) -> ExpressionResult {
        let elements = elements
            .into_iter()
            .map(|e| self.handle_expression(e))
            .collect::<Vec<_>>();
        let (prelim_stmts, elems) = self.extract_necessary(elements);
        let expr = swc::Expr::Array(swc::ArrayLit {
            span: DUMMY_SP,
            elems: elems.into_iter().map(|e| Some(e.into())).collect(),
        });
        ExpressionResult { prelim_stmts, expr }
    }

    pub fn handle_boolean_literal(&mut self, b: ir::BooleanLiteral) -> swc::Expr {
        swc::Expr::from(swc::Bool {
            span: DUMMY_SP,
            value: b.value,
        })
    }

    pub fn handle_call(&mut self, node: ir::CallExpression) -> ExpressionResult {
        let should_clone = self.call_ownership(&node) == OwnershipAction::Clone;
        let mut callee_result = self.handle_expression(*node.callee);
        let args_results = node
            .args
            .into_iter()
            .map(|a| self.handle_expression(a))
            .collect::<Vec<_>>();
        let (prelim_stmts, args) = self.extract_necessary(args_results);
        if !prelim_stmts.is_empty() {
            callee_result = self.to_extracted(callee_result);
        }
        let prelim_stmts = vec![callee_result.prelim_stmts, prelim_stmts].concat();
        let callee = callee_result.expr;

        let mut expr = swc::Expr::Call(swc::CallExpr {
            callee: swc::Callee::Expr(Box::new(callee)),
            args: args.into_iter().map(Into::into).collect(),
            ..Default::default()
        });
        if should_clone {
            expr = swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(expr),
                prop: swc::MemberProp::Ident(ident_from_str("$clone").into()),
            });
        }

        ExpressionResult { prelim_stmts, expr }
    }

    pub fn handle_float_literal(&mut self, node: ir::FloatLiteral) -> swc::Expr {
        swc::Expr::from(swc::Number {
            span: DUMMY_SP,
            value: node.value,
            raw: None,
        })
    }

    fn handle_for_expression(&mut self, node: ir::ForExpression) -> ExpressionResult {
        let temp = self.get_temp_id();
        self.with_break_target(temp.clone(), |self_| ExpressionResult {
            prelim_stmts: self_.for_to_swc_stmt(node),
            expr: temp.into(),
        })
    }

    fn handle_for_in_expression(&mut self, node: ir::ForInExpression) -> ExpressionResult {
        let temp = self.get_temp_id();
        self.with_break_target(temp.clone(), |self_| ExpressionResult {
            prelim_stmts: self_.for_in_to_swc_stmt(node),
            expr: temp.into(),
        })
    }

    fn handle_function_expression(&mut self, node: ir::FunctionExpression) -> swc::ArrowExpr {
        let swc_params = self.function_params_to_swc(node.params);
        let swc_body = self.function_body_to_swc(node.body);

        swc::ArrowExpr {
            params: swc_params,
            body: Box::new(swc_body),
            ..Default::default()
        }
    }

    fn handle_match_expression(&mut self, node: ir::MatchExpression) -> ExpressionResult {
        let temp = self.get_temp_id();
        let decl = ident_to_declaration(temp.clone());
        let mut stmts = self.handle_match_statement(node);
        match stmts.last_mut() {
            Some(swc::Stmt::If(i)) => assign_if_last_expressions(i, temp.clone()),
            _ => panic!(),
        }
        stmts.insert(0, decl);
        ExpressionResult {
            prelim_stmts: stmts,
            expr: temp.into(),
        }
    }

    pub fn function_params_to_swc(
        &mut self,
        params: Vec<(Location, VariableSymbolId)>,
    ) -> Vec<swc::Pat> {
        params
            .into_iter()
            .map(|param| {
                let name = &self.symbols.get(param.1).name;
                swc::Pat::Ident(swc::BindingIdent {
                    id: ident_from_str(name),
                    type_ann: None,
                })
            })
            .collect()
    }

    pub fn function_body_to_swc(&mut self, body: ir::Block) -> swc::BlockStmtOrExpr {
        let stmts = body
            .statements
            .into_iter()
            .flat_map(|stmt| self.stmt_to_swc(stmt))
            .collect();

        swc::BlockStmtOrExpr::BlockStmt(create_block_stmt(stmts))
    }

    pub fn handle_identifier(&mut self, node: ir::Identifier) -> swc::Expr {
        if let SymbolId::Variant(v) = node.symbol {
            return self.handle_variant_construct(v);
        }
        let name = self.symbols.get_symbol(node.symbol).name();
        match self.identifier_ownership(&node) {
            OwnershipAction::Borrow | OwnershipAction::Copy | OwnershipAction::Move => {
                swc::Expr::Ident(ident_from_str(name))
            }
            OwnershipAction::Clone => swc::Expr::Call(swc::CallExpr {
                callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(ident_from_str(name).into()),
                    prop: swc::MemberProp::Ident(ident_from_str("$clone").into()),
                }))),
                ..Default::default()
            }),
        }
    }

    pub fn handle_int_literal(&mut self, node: ir::IntLiteral) -> swc::Expr {
        swc::Expr::from(swc::Number {
            span: DUMMY_SP,
            value: node.value as f64,
            raw: None,
        })
    }

    /// ```js
    /// Enum.Variant()
    /// ```
    fn handle_variant_construct(&mut self, v: VariantSymbolId) -> swc::Expr {
        let variant = self.symbols.get(v);
        let enum_ = variant.owner;
        let enum_name = self.symbol_name(enum_);
        let variant_name = &variant.name;
        let callee = swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(ident_from_str(enum_name).into()),
            prop: swc::MemberProp::Ident(ident_from_str(variant_name).into()),
        })));

        swc::Expr::Call(swc::CallExpr {
            callee,
            ..Default::default()
        })
    }

    fn handle_index_expression(&mut self, node: ir::IndexExpression) -> ExpressionResult {
        let obj_result = self.handle_host(node.object);

        let prop = swc::MemberProp::Computed(swc::ComputedPropName {
            span: DUMMY_SP,
            expr: Box::new(swc::Expr::Lit(swc::Lit::Num(swc::Number {
                span: DUMMY_SP,
                value: node.index.1 as f64,
                raw: None,
            }))),
        });

        let expr = swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(obj_result.expr),
            prop,
        };

        ExpressionResult {
            prelim_stmts: obj_result.prelim_stmts,
            expr: expr.into(),
        }
    }

    fn handle_intrinsic_call(&mut self, node: ir::IntrinsicCall) -> ExpressionResult {
        let (prelim, args): (Vec<Vec<swc::Stmt>>, _) = node
            .args
            .into_iter()
            .map(|a| self.handle_expression(a))
            .map(|r| (r.prelim_stmts, r.expr.into()))
            .unzip();
        let name = self.symbol_name(node.callee);
        let prelim = prelim.into_iter().flatten().collect();
        let call = internal_method_call(name, args);
        ExpressionResult {
            prelim_stmts: prelim,
            expr: call.into(),
        }
    }

    fn handle_intrinsic_construct(&mut self, node: ir::IntrinsicConstruct) -> ExpressionResult {
        // FIXME: handle arguments
        let name = self.symbol_name(node.constructor);
        let call = internal_construct(name);
        ExpressionResult {
            prelim_stmts: vec![],
            expr: call.into(),
        }
    }

    pub fn handle_member_expression(&mut self, node: ir::MemberExpression) -> ExpressionResult {
        let obj_result = self.handle_host(node.object);

        let prop_name = &self.symbols.get(node.member.1).name;
        let prop = swc::MemberProp::Ident(ident_from_str(&prop_name).into());

        let expr = swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(obj_result.expr),
            prop,
        };

        ExpressionResult {
            prelim_stmts: obj_result.prelim_stmts,
            expr: expr.into(),
        }
    }

    pub fn handle_method(&mut self, node: ir::MethodExpression) -> ExpressionResult {
        if let Some(known) = self.wellknown.methods.get(&node.method.1) {
            return known(self, node);
        }

        let should_clone = self.method_ownership(&node) == OwnershipAction::Clone;

        let obj_result = self.handle_host(node.host);
        let method_name = &self.symbols.get(node.method.1).name;
        let prop = swc::MemberProp::Ident(ident_from_str(&method_name).into());
        let callee = swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(obj_result.expr),
            prop,
        };

        let args_results = node
            .args
            .into_iter()
            .map(|a| self.handle_expression(a))
            .collect::<Vec<_>>();
        let (prelim_stmts, args) = self.extract_necessary(args_results);

        let mut expr = swc::Expr::Call(swc::CallExpr {
            callee: swc::Callee::Expr(Box::new(callee.into())),
            args: args.into_iter().map(Into::into).collect(),
            ..Default::default()
        });
        if should_clone {
            expr = swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(expr),
                prop: swc::MemberProp::Ident(ident_from_str("$clone").into()),
            });
        }

        ExpressionResult { prelim_stmts, expr }
    }

    fn handle_host(&mut self, node: Option<Box<ir::Expression>>) -> ExpressionResult {
        if let Some(node) = node {
            return self.handle_expression(*node);
        }
        let expr = swc::Expr::This(swc::ThisExpr { span: DUMMY_SP });
        ExpressionResult {
            prelim_stmts: vec![],
            expr,
        }
    }

    pub fn handle_string_literal(&mut self, node: ir::StringLiteral) -> swc::Str {
        swc::Str {
            span: DUMMY_SP,
            value: node.value.into(),
            raw: None,
        }
    }

    fn handle_simple_struct(&mut self, node: ir::StructExpression) -> ExpressionResult {
        let constructor_name = self.make_constructor(node.ty, node.constructor.1.into());
        let expected_members = &self.symbols.get(node.constructor.1).members;
        let (prelim_stmts, args) = self.handle_struct_like_body(node, expected_members);
        let expr = swc::Expr::New(swc::NewExpr {
            callee: Box::new(constructor_name),
            args: Some(args.into_iter().map(Into::into).collect()),
            ..Default::default()
        });

        ExpressionResult { prelim_stmts, expr }
    }

    fn handle_struct_like_body(
        &mut self,
        node: ir::StructExpression,
        expected: &[MemberSymbolId],
    ) -> (Vec<swc::Stmt>, Vec<swc::Expr>) {
        let mut order = expected
            .into_iter()
            .map(|s| {
                let name = self.symbol_name(*s);
                node.fields
                    .iter()
                    .enumerate()
                    .find(|(_, f)| &self.symbols.get(f.name.1).name == name)
                    .unwrap()
                    .0
            })
            .collect::<Vec<_>>();

        let results = node
            .fields
            .into_iter()
            .map(|field| self.handle_expression(field.value))
            .collect::<Vec<_>>();
        let (prelim, mut args) = self.extract_all(results);

        for i in 0..args.len() {
            while order[i] != i {
                let next = order[i];
                args.swap(i, next);
                order.swap(i, next);
            }
        }
        (prelim, args)
    }

    fn handle_type_match(&mut self, node: ir::TypeMatch) -> ExpressionResult {
        let obj_result = self.handle_expression(*node.expr);

        let variant_name = &self.symbols.get(node.variant).name;
        let expr = swc::BinExpr {
            span: DUMMY_SP,
            op: swc::BinaryOp::EqEqEq,
            left: Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(obj_result.expr),
                prop: swc::MemberProp::Ident(ident_from_str(TAG_SYMBOL).into()),
            })),
            right: Box::new(create_str(variant_name)),
        };

        ExpressionResult {
            prelim_stmts: obj_result.prelim_stmts,
            expr: expr.into(),
        }
    }

    fn make_constructor(&self, expr_ty: types::TypeId, constructor: TypeSymbolId) -> swc::Expr {
        let type_args = match self.types.get(expr_ty) {
            types::Type::Ref(r) => &r.args,
            _ => &vec![],
        };
        let constructor_type = self.symbol_type_id(constructor);
        let params = match self.types.get(constructor_type).as_params() {
            Some(p) => p.to_vec(),
            None => vec![],
        };
        let ty_args = Substitutions::with_initial(&params, type_args).into();

        let methods = match constructor {
            TypeSymbolId::Enum(s) => &self.symbols.get(s).methods,
            TypeSymbolId::Struct(s) => &self.symbols.get(s).methods,
            _ => panic!(),
        };

        let concrete_exists = methods
            .into_iter()
            .any(|m| self.symbols.get(*m).owner_args == ty_args);

        if concrete_exists {
            self.generate_constructor_name(constructor.into(), &ty_args)
        } else {
            let name = self.symbol_name(constructor);
            swc::Expr::Ident(ident_from_str(name))
        }
    }
}
