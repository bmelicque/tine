use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_symbols::symbols::*;
use tine_types::types;

use crate::codegen::{
    statements::{assignments::assignment, utils::declare_const},
    utils::{ident_from_str, internal_method_call, member},
    CodeGenerator,
};

impl CodeGenerator<'_, '_> {
    pub(crate) fn get_field(&mut self, field: MemberSymbolId) -> swc::Expr {
        let ty = self.symbol_type_id(field);
        match self.resolve(ty) {
            types::Type::Array(_) | types::Type::Tuple(_) => {
                internal_method_call("cloneArray", vec![self.this_field(field).into()]).into()
            }
            types::Type::Param(_) => {
                internal_method_call("clone", vec![self.this_field(field).into()]).into()
            }
            ty if is_primitive(&ty) => self.this_field(field),
            _ => self.get_this_field(field).into(),
        }
    }

    pub(crate) fn set_fields(&mut self, fields: &[MemberSymbolId]) -> Vec<swc::Stmt> {
        fields
            .iter()
            .map(|f| {
                let target = self.this_field(*f);
                let src = member(ident_from_str("src").into(), self.symbol_name(*f)).into();
                self.set_field(target, src, self.symbol_type_id(*f))
            })
            .collect()
    }

    pub(crate) fn set_field(
        &mut self,
        target: swc::Expr,
        src: swc::Expr,
        ty: types::TypeId,
    ) -> swc::Stmt {
        match self.resolve(ty).clone() {
            types::Type::Array(a) => self.set_array_field(target, src, a).into(),
            types::Type::Param(_) => assignment(
                target.clone(),
                internal_method_call("set", vec![target.into(), src.into()]).into(),
            ),
            types::Type::Tuple(t) => self.set_tuple_field(target, src, t).into(),
            ty if is_primitive(&ty) => assignment(target, src),
            _ => swc::Stmt::Expr(swc::ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(swc::Expr::Call(swc::CallExpr {
                    callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(target),
                        prop: swc::MemberProp::Ident(ident_from_str("$set").into()),
                    }))),
                    args: vec![src.into()],
                    ..Default::default()
                })),
            }),
        }
    }

    /// ```js
    /// const t = TARGET
    /// const s = SRC
    /// t.length = s.length
    /// for (let i = 0; i < t.length; i++) {
    ///     /* SET INNER FIELDS */
    /// }
    /// ```
    fn set_array_field(
        &mut self,
        target: swc::Expr,
        src: swc::Expr,
        ty: types::ArrayType,
    ) -> swc::BlockStmt {
        let target_declaration = declare_const("t", target).into();
        let src_declaration = declare_const("s", src).into();
        let t = ident_from_str("t");
        let s = ident_from_str("s");
        let i = ident_from_str("i");
        let set_length = assignment(
            length_of(t.clone().into()).into(),
            length_of(s.clone().into()).into(),
        );
        let loop_range = length_of(t.clone().into()).into();
        let loop_body = self.set_field(
            computed(t.into(), i.clone().into()).into(),
            computed(s.into(), i.into()).into(),
            ty.element,
        );
        let for_loop = make_loop(loop_range, loop_body).into();

        swc::BlockStmt {
            stmts: vec![target_declaration, src_declaration, set_length, for_loop],
            ..Default::default()
        }
    }

    fn set_tuple_field(
        &mut self,
        target: swc::Expr,
        src: swc::Expr,
        ty: types::TupleType,
    ) -> swc::BlockStmt {
        let target_declaration = declare_const("t", target).into();
        let src_declaration = declare_const("s", src).into();
        let t = swc::Expr::from(ident_from_str("t"));
        let s = swc::Expr::from(ident_from_str("s"));
        let stmts = ty
            .elements
            .iter()
            .enumerate()
            .map(|(i, ty)| {
                let i = format!("_{}", i);
                let target = member(t.clone(), &i).into();
                let src = member(s.clone(), &i).into();
                self.set_field(target, src, *ty)
            })
            .collect();
        swc::BlockStmt {
            stmts: vec![vec![target_declaration, src_declaration], stmts].concat(),
            ..Default::default()
        }
    }

    /// `this.FIELD_NAME`
    pub fn this_field<S>(&self, field: S) -> swc::Expr
    where
        S: Into<SymbolId>,
    {
        let name = self.symbol_name(field);
        swc::Expr::Member(swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(swc::Expr::This(swc::ThisExpr { span: DUMMY_SP })),
            prop: swc::MemberProp::Ident(ident_from_str(name).into()),
        })
    }

    /// `this.FIELD_NAME.$get()`
    fn get_this_field<S>(&self, field: S) -> swc::CallExpr
    where
        S: Into<SymbolId>,
    {
        swc::CallExpr {
            callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(self.this_field(field)),
                prop: swc::MemberProp::Ident(ident_from_str("$get").into()),
            }))),
            ..Default::default()
        }
    }
}

fn is_primitive(ty: &types::Type) -> bool {
    match ty {
        types::Type::Boolean | types::Type::Float | types::Type::Integer | types::Type::String => {
            true
        }
        _ => false,
    }
}

fn make_loop(range: swc::Expr, body: swc::Stmt) -> swc::ForStmt {
    // `let i = 0`
    let loop_init = Some(swc::VarDeclOrExpr::VarDecl(Box::new(swc::VarDecl {
        kind: swc::VarDeclKind::Let,
        decls: vec![swc::VarDeclarator {
            span: DUMMY_SP,
            name: swc::Pat::Ident(ident_from_str("i").into()),
            init: Some(Box::new(swc::Expr::Lit(swc::Lit::Num(swc::Number {
                span: DUMMY_SP,
                value: 0.,
                raw: None,
            })))),
            definite: false,
        }],
        ..Default::default()
    })));

    // `i < range`
    let loop_test = Some(Box::new(swc::Expr::Bin(swc::BinExpr {
        span: DUMMY_SP,
        op: swc::BinaryOp::Lt,
        left: Box::new(ident_from_str("i").into()),
        right: Box::new(range),
    })));

    // `i++`
    let loop_update = Some(Box::new(swc::Expr::Update(swc::UpdateExpr {
        arg: Box::new(ident_from_str("i").into()),
        ..Default::default()
    })));

    swc::ForStmt {
        span: DUMMY_SP,
        init: loop_init,
        test: loop_test,
        update: loop_update,
        body: Box::new(body),
    }
}

fn length_of(expr: swc::Expr) -> swc::MemberExpr {
    swc::MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(expr),
        prop: swc::MemberProp::Ident(ident_from_str("length").into()),
    }
}

fn computed(object: swc::Expr, prop: swc::Expr) -> swc::MemberExpr {
    swc::MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(object),
        prop: swc::MemberProp::Computed(swc::ComputedPropName {
            span: DUMMY_SP,
            expr: Box::new(prop),
        }),
    }
}

pub fn member_assignment(object: swc::Expr, prop: swc::Ident, value: swc::Expr) -> swc::Stmt {
    swc::Stmt::Expr(swc::ExprStmt {
        expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
            left: swc::AssignTarget::Simple(swc::SimpleAssignTarget::Member(swc::MemberExpr {
                obj: Box::new(object),
                prop: swc::MemberProp::Ident(prop.into()),
                ..Default::default()
            })),
            right: Box::new(value),
            ..Default::default()
        })),
        ..Default::default()
    })
}

pub fn this_assignment(this_prop: swc::Ident, value: swc::Expr) -> swc::Stmt {
    member_assignment(
        swc::Expr::This(swc::ThisExpr { span: DUMMY_SP }),
        this_prop,
        value,
    )
}
