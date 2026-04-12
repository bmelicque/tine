use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::{ir, SymbolRef, TypeSymbolBody};

use crate::codegen::{utils::create_ident, CodeGenerator};

impl CodeGenerator<'_> {
    /// eg.
    /// ```tine
    /// enum Option<T> {
    ///     Some(T),
    ///     None
    /// }
    /// ```
    /// maps to
    /// ```js
    /// class Option {
    ///     static Some(_0) {
    ///         const $ = new this
    ///         $.$tag = 0
    ///         $._0 = _0
    ///         return $
    ///     }
    ///     static None() {
    ///         const $ = new this
    ///         $.$tag = 1
    ///         return $
    ///     }
    /// }
    /// ```
    /// This allows specification when generics become concrete types:
    /// ```js
    /// Option.$int = class extends Option {}
    /// Option.$int.Some(x) // works just fine
    /// ```
    pub(super) fn enum_def_to_swc(&mut self, node: &ir::EnumDefinition) -> swc::ClassDecl {
        let body = node
            .variants()
            .into_iter()
            .enumerate()
            .map(|(id, constructor_symbol)| {
                self.variant_to_constructor(
                    id,
                    constructor_symbol.as_name(),
                    constructor_symbol.as_type_body(),
                )
                .into()
            })
            .collect();

        let class = swc::Class {
            span: DUMMY_SP,
            body,
            ..Default::default()
        };

        swc::ClassDecl {
            ident: create_ident(&node.name.as_name()),
            declare: false,
            class: Box::new(class),
        }
    }

    fn variant_to_constructor(
        &mut self,
        id: usize,
        name: String,
        body: Option<TypeSymbolBody>,
    ) -> swc::ClassMethod {
        let body_symbols = match body {
            Some(TypeSymbolBody::Struct(st)) => st.into_iter().map(|(_, s)| s).collect(),
            Some(TypeSymbolBody::Tuple(t)) => t,
            None => vec![],
        };

        let function = self.make_variant_constructor(id, body_symbols);

        swc::ClassMethod {
            span: DUMMY_SP,
            key: swc::PropName::Ident(create_ident(&name).into()),
            function: Box::new(function),
            is_static: true,
            ..Default::default()
        }
    }

    fn make_variant_constructor(&mut self, id: usize, symbols: Vec<SymbolRef>) -> swc::Function {
        let params = symbols
            .iter()
            .map(|s| swc::Param {
                span: DUMMY_SP,
                decorators: vec![],
                pat: swc::Pat::Ident(create_ident(&s.as_name()).into()),
            })
            .collect();

        let mut stmts = Vec::with_capacity(symbols.len() + 3);
        // `const $ = new this`
        stmts.push(swc::Stmt::Decl(swc::Decl::Var(Box::new(swc::VarDecl {
            span: DUMMY_SP,
            kind: swc::VarDeclKind::Const,
            decls: vec![swc::VarDeclarator {
                span: DUMMY_SP,
                name: swc::Pat::Ident(create_ident("$").into()),
                init: Some(Box::new(swc::Expr::New(swc::NewExpr {
                    span: DUMMY_SP,
                    callee: Box::new(swc::Expr::This(swc::ThisExpr { span: DUMMY_SP })),
                    ..Default::default()
                }))),
                definite: false,
            }],
            ..Default::default()
        }))));
        // `$.$tag = ID`
        stmts.push(member_assignment(
            create_ident("$").into(),
            create_ident("$tag"),
            create_ident(&id.to_string()).into(),
        ));
        for symbol in symbols {
            let name = symbol.as_name();
            // `$.KEY = VALUE`
            stmts.push(member_assignment(
                create_ident("$").into(),
                create_ident(&name),
                create_ident(&name).into(),
            ));
        }
        // `return $`
        stmts.push(swc::Stmt::Return(swc::ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(swc::Expr::Ident(create_ident("$")))),
        }));

        swc::Function {
            params,
            span: DUMMY_SP,
            body: Some(swc::BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                stmts,
            }),
            ..Default::default()
        }
    }

    pub(super) fn struct_def_to_swc(&mut self, node: &ir::StructDefinition) -> swc::ClassDecl {
        let body = match node.body() {
            TypeSymbolBody::Struct(st) => self.struct_fields_to_swc_constructor(
                st.into_iter().map(|(_, symbol)| symbol.clone()).collect(),
            ),
            TypeSymbolBody::Tuple(t) => self.struct_fields_to_swc_constructor(t),
        };

        let class = swc::Class {
            span: DUMMY_SP,
            body: vec![body.into()],
            ..Default::default()
        };

        swc::ClassDecl {
            ident: create_ident(&node.name.as_name()),
            declare: false,
            class: Box::new(class),
        }
    }

    fn struct_fields_to_swc_constructor(&mut self, body: Vec<SymbolRef>) -> swc::Constructor {
        let params = body
            .iter()
            .map(|symbol| {
                swc::ParamOrTsParamProp::Param(swc::Param {
                    span: DUMMY_SP,
                    decorators: vec![],
                    pat: swc::Pat::Ident(create_ident(&symbol.as_name()).into()),
                })
            })
            .collect();

        let body = swc::BlockStmt {
            stmts: body
                .into_iter()
                .map(|symbol| {
                    this_assignment(
                        create_ident(&symbol.as_name()),
                        create_ident(&symbol.as_name()).into(),
                    )
                })
                .collect(),
            ..Default::default()
        };

        swc::Constructor {
            key: swc::PropName::Ident(create_ident("constructor").into()),
            params,
            body: Some(body),
            ..Default::default()
        }
    }
}

fn this_assignment(this_prop: swc::Ident, value: swc::Expr) -> swc::Stmt {
    member_assignment(
        swc::Expr::This(swc::ThisExpr { span: DUMMY_SP }),
        this_prop,
        value,
    )
}

fn member_assignment(object: swc::Expr, prop: swc::Ident, value: swc::Expr) -> swc::Stmt {
    swc::Stmt::Expr(swc::ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
            span: DUMMY_SP,
            op: swc::AssignOp::Assign,
            left: swc::AssignTarget::Simple(swc::SimpleAssignTarget::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(object),
                prop: swc::MemberProp::Ident(prop.into()),
            })),
            right: Box::new(value),
        })),
    })
}
