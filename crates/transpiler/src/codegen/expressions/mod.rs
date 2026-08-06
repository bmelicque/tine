mod binary;
mod block;
mod dom;
mod ifs;
mod unary;
mod utils;

use super::{utils::ident_from_str, CodeGenerator};
use crate::{
    codegen::utils::{create_block_stmt, create_str, internal_construct, internal_method_call},
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
            BooleanLiteral(b) => ExpressionResult::from(swc::Bool {
                span: DUMMY_SP,
                value: b.value,
            }),
            Block(b) => self.handle_block(b),
            Call(c) => self.handle_call(c),
            Element(e) => self.handle_element_expression(e),
            FloatLiteral(f) => ExpressionResult::from(swc::Number {
                span: DUMMY_SP,
                value: f.value,
                raw: None,
            }),
            For(f) => self.handle_for_expression(f),
            ForIn(f) => self.handle_for_in_expression(f),
            Function(f) => self.handle_function_expression(f).into(),
            Identifier(i) => self.handle_identifier(i).into(),
            If(i) => self.handle_if_expression(i),
            IntLiteral(i) => ExpressionResult::from(swc::Number {
                span: DUMMY_SP,
                value: i.value as f64,
                raw: None,
            }),
            IntrinsicCall(i) => self.handle_intrinsic_call(i),
            IntrinsicConstruct(i) => self.handle_intrinsic_construct(i),
            Member(m) => self.member_expr_to_swc(m),
            Method(m) => self.handle_method(m),
            StringLiteral(s) => self.string_literal_to_swc(s).into(),
            Struct(s) => self.struct_to_swc(s),
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

    pub fn member_expr_to_swc(&mut self, node: ir::MemberExpression) -> ExpressionResult {
        let obj_result = self.handle_expression(*node.object);

        let prop_name = &self.symbols.get(node.member.1).name;
        let prop = match prop_name.parse::<usize>() {
            Ok(int) => swc::MemberProp::Computed(swc::ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(swc::Expr::Lit(swc::Lit::Num(swc::Number {
                    span: DUMMY_SP,
                    value: int as f64,
                    raw: None,
                }))),
            }),
            Err(_) => swc::MemberProp::Ident(ident_from_str(&prop_name).into()),
        };

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

        let obj_result = self.handle_expression(*node.host);
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

    fn string_literal_to_swc(&mut self, node: ir::StringLiteral) -> swc::Str {
        swc::Str {
            span: DUMMY_SP,
            value: node.value.into(),
            raw: None,
        }
    }

    fn struct_to_swc(&mut self, node: ir::StructLiteral) -> ExpressionResult {
        use ir::StructConstructor::*;
        match node.constructor {
            Enum(_, _, v) => self.handle_variant_struct(node, v),
            Struct(_, _) => self.handle_simple_struct(node),
        }
    }

    fn handle_variant_struct(
        &mut self,
        node: ir::StructLiteral,
        variant: VariantSymbolId,
    ) -> ExpressionResult {
        let constructor_name = self.get_constructor_name(node.ty, &node.constructor);
        let body = &self.symbols.get(variant).body;
        let (prelim_stmts, args) = match &body {
            Some(TypeSymbolBody::Struct(fields)) => self.handle_struct_like_body(node, fields),
            Some(TypeSymbolBody::Tuple(_)) => self.handle_tuple_like_body(node),
            None => (vec![], vec![]),
        };
        let variant_name = &self.symbols.get(variant).name;
        let callee = swc::Expr::Member(swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(constructor_name),
            prop: swc::MemberProp::Ident(ident_from_str(variant_name).into()),
        });
        let expr = swc::Expr::Call(swc::CallExpr {
            callee: swc::Callee::Expr(Box::new(callee)),
            args: args.into_iter().map(Into::into).collect(),
            ..Default::default()
        });
        ExpressionResult { prelim_stmts, expr }
    }

    fn handle_simple_struct(&mut self, node: ir::StructLiteral) -> ExpressionResult {
        let constructor_name = self.get_constructor_name(node.ty, &node.constructor);
        let expected_body = match node.constructor {
            ir::StructConstructor::Enum(_, _, v) => self.symbols.get(v).body.as_ref().unwrap(),
            ir::StructConstructor::Struct(_, s) => &self.symbols.get(s).body,
        };
        let (prelim_stmts, args) = match expected_body {
            TypeSymbolBody::Struct(fields) => self.handle_struct_like_body(node, fields),
            TypeSymbolBody::Tuple(_) => self.handle_tuple_like_body(node),
        };
        let expr = swc::Expr::New(swc::NewExpr {
            callee: Box::new(constructor_name),
            args: Some(args.into_iter().map(Into::into).collect()),
            ..Default::default()
        });

        ExpressionResult { prelim_stmts, expr }
    }

    fn handle_struct_like_body(
        &mut self,
        node: ir::StructLiteral,
        expected: &[(String, MemberSymbolId)],
    ) -> (Vec<swc::Stmt>, Vec<swc::Expr>) {
        let mut order = expected
            .into_iter()
            .map(|(name, _)| {
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

    fn handle_tuple_like_body(
        &mut self,
        node: ir::StructLiteral,
    ) -> (Vec<swc::Stmt>, Vec<swc::Expr>) {
        let results = node
            .fields
            .into_iter()
            .map(|field| self.handle_expression(field.value))
            .collect::<Vec<_>>();
        self.extract_necessary(results)
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
                prop: swc::MemberProp::Ident(ident_from_str("$tag").into()),
            })),
            right: Box::new(create_str(variant_name)),
        };

        ExpressionResult {
            prelim_stmts: obj_result.prelim_stmts,
            expr: expr.into(),
        }
    }

    fn get_constructor_name(
        &self,
        expr_ty: types::TypeId,
        constructor: &ir::StructConstructor,
    ) -> swc::Expr {
        // déterminer les arguments de type de l'appelant
        let type_args = match self.types.get(expr_ty) {
            types::Type::Ref(r) => &r.args,
            _ => &vec![],
        };

        let constructor_type = match constructor {
            ir::StructConstructor::Enum(_, e, _) => self.symbols.get(*e).ty,
            ir::StructConstructor::Struct(_, s) => self.symbols.get(*s).ty,
        };

        let params = match self.types.get(constructor_type).as_params() {
            Some(p) => p.to_vec(),
            None => vec![],
        };
        let ty_args = Substitutions::with_initial(&params, type_args).into();

        let methods = match constructor {
            ir::StructConstructor::Enum(_, e, _) => &self.symbols.get(*e).methods,
            ir::StructConstructor::Struct(_, s) => &self.symbols.get(*s).methods,
        };

        let concrete_exists = methods
            .into_iter()
            .any(|m| self.symbols.get(*m).owner_args == ty_args);

        if concrete_exists {
            let id = match constructor {
                ir::StructConstructor::Enum(_, e, _) => (*e).into(),
                ir::StructConstructor::Struct(_, s) => (*s).into(),
            };
            self.generate_constructor_name(id, &ty_args)
        } else {
            let name = match constructor {
                ir::StructConstructor::Enum(_, e, _) => &self.symbols.get(*e).name,
                ir::StructConstructor::Struct(_, s) => &self.symbols.get(*s).name,
            };
            swc::Expr::Ident(ident_from_str(name))
        }
    }
}
