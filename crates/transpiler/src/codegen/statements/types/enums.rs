use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_ir as ir;
use tine_symbols::symbols::*;

use crate::codegen::{
    statements::types::utils::member_assignment,
    utils::{ident_from_str, member},
    CodeGenerator,
};

const SRC_NAME: &str = "src";

impl CodeGenerator<'_, '_> {
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
    pub(crate) fn enum_def_to_swc(&mut self, node: ir::EnumDefinition) -> swc::ClassDecl {
        let variants = &self.symbols.get(node.symbol).variants;
        let mut body: Vec<swc::ClassMember> = variants
            .into_iter()
            .enumerate()
            .map(|(idx, constructor_id)| {
                let constructor_symbol = self.symbols.get(*constructor_id);
                self.variant_to_constructor(
                    idx,
                    constructor_symbol.name.clone(),
                    constructor_symbol.body.clone(),
                )
            })
            .map(Into::into)
            .collect();

        body.push(self.make_enum_getter(variants).into());
        body.push(self.make_enum_setter(variants).into());

        let methods = &self.symbols.get(node.symbol).methods;
        let child_classes = self.generate_concrete_classes(methods);
        body.extend(child_classes);

        let class = swc::Class {
            span: DUMMY_SP,
            body,
            ..Default::default()
        };

        let name = &self.symbols.get(node.symbol).name;
        swc::ClassDecl {
            ident: ident_from_str(name),
            declare: false,
            class: Box::new(class),
        }
    }

    /// Make the constructor method for a given variant
    ///
    /// ```js
    /// static VARIANT_NAME(...PARAMS) {
    ///     // constructor
    /// }
    /// ```
    fn variant_to_constructor(
        &mut self,
        id: usize,
        name: String,
        body: Vec<MemberSymbolId>,
    ) -> swc::ClassMethod {
        let function = self.make_variant_constructor(id, body);

        swc::ClassMethod {
            span: DUMMY_SP,
            key: swc::PropName::Ident(ident_from_str(&name).into()),
            function: Box::new(function),
            is_static: true,
            ..Default::default()
        }
    }

    fn make_variant_constructor(
        &mut self,
        id: usize,
        members: Vec<MemberSymbolId>,
    ) -> swc::Function {
        let params = members
            .iter()
            .map(|s| {
                let name = self.symbol_name(*s);
                swc::Param::from(swc::Pat::Ident(ident_from_str(name).into()))
            })
            .collect();

        let mut stmts = Vec::with_capacity(members.len() + 3);
        // `const $ = new this`
        stmts.push(swc::Stmt::Decl(swc::Decl::Var(Box::new(swc::VarDecl {
            span: DUMMY_SP,
            kind: swc::VarDeclKind::Const,
            decls: vec![swc::VarDeclarator {
                span: DUMMY_SP,
                name: swc::Pat::Ident(ident_from_str("$").into()),
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
            ident_from_str("$").into(),
            ident_from_str("$tag"),
            ident_from_str(&id.to_string()).into(),
        ));
        for member in members {
            let name = self.symbol_name(member);
            // `$.KEY = VALUE`
            stmts.push(member_assignment(
                ident_from_str("$").into(),
                ident_from_str(name),
                ident_from_str(name).into(),
            ));
        }
        // `return $`
        stmts.push(swc::Stmt::Return(swc::ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(swc::Expr::Ident(ident_from_str("$")))),
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

    /// Build the `$get` method for an enum.
    ///
    /// ```js
    /// $get() {
    ///     switch (this.$tag) {
    ///         ...CASES
    ///     }
    /// }
    /// ```
    fn make_enum_getter(&mut self, variants: &[VariantSymbolId]) -> swc::ClassMethod {
        let discriminant = swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(swc::Expr::This(swc::ThisExpr { span: DUMMY_SP })),
            prop: swc::MemberProp::Ident(ident_from_str("$tag").into()),
        };

        let cases = variants
            .into_iter()
            .enumerate()
            .map(|(id, v)| self.make_variant_getter(id, *v))
            .collect();

        let switch = swc::Stmt::Switch(swc::SwitchStmt {
            span: DUMMY_SP,
            discriminant: Box::new(discriminant.into()),
            cases,
        });

        swc::ClassMethod {
            key: swc::PropName::Ident(ident_from_str("$get").into()),
            function: Box::new(swc::Function {
                body: Some(swc::BlockStmt {
                    stmts: vec![switch],
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Create a getter switch case for enums, in the form of:
    /// ```js
    /// case VARIANT_TAG:
    ///     return new this.constructor.VARIANT_NAME(...ARGS)
    /// ```
    fn make_variant_getter(&mut self, id: usize, variant: VariantSymbolId) -> swc::SwitchCase {
        let body = &self.symbols.get(variant).body;

        let test = Some(Box::new(swc::Expr::Lit(swc::Lit::Num(swc::Number {
            span: DUMMY_SP,
            value: id as f64,
            raw: None,
        }))));

        let args = body
            .iter()
            .map(|s| Some(self.get_field(*s).into()))
            .collect();
        let value = swc::Expr::New(swc::NewExpr {
            callee: Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(swc::Expr::Member(swc::MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(swc::Expr::This(swc::ThisExpr { span: DUMMY_SP })),
                    prop: swc::MemberProp::Ident(ident_from_str("constructor").into()),
                })),
                prop: swc::MemberProp::Ident(ident_from_str(self.symbol_name(variant)).into()),
            })),
            args,
            ..Default::default()
        });
        let cons = swc::Stmt::Return(swc::ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(value)),
        });

        swc::SwitchCase {
            span: DUMMY_SP,
            test, // case VARIANT_ID
            cons: vec![cons],
        }
    }

    ///
    fn make_enum_setter(&mut self, variants: &[VariantSymbolId]) -> swc::ClassMethod {
        let stmt = swc::Stmt::If(swc::IfStmt {
            span: DUMMY_SP,
            test: Box::new(tags_test().into()),
            cons: Box::new(self.make_enum_setter_same_tag(variants).into()),
            alt: Some(Box::new(
                self.make_enum_setter_different_tags(variants).into(),
            )),
        });

        swc::ClassMethod {
            key: swc::PropName::Ident(ident_from_str("$get").into()),
            function: Box::new(swc::Function {
                body: Some(swc::BlockStmt {
                    stmts: vec![stmt],
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn make_enum_setter_same_tag(&mut self, variants: &[VariantSymbolId]) -> swc::SwitchStmt {
        let discriminant = member(swc::Expr::This(swc::ThisExpr { span: DUMMY_SP }), "$tag");

        let cases = variants
            .iter()
            .enumerate()
            .map(|(id, v)| self.make_variant_setter(id, *v))
            .collect();

        swc::SwitchStmt {
            span: DUMMY_SP,
            discriminant: Box::new(discriminant.into()),
            cases,
        }
    }

    /// ```js
    /// case VARIANT_TAG:
    ///     /* set fields */
    ///     return
    /// ```
    fn make_variant_setter(&mut self, id: usize, variant: VariantSymbolId) -> swc::SwitchCase {
        let test = test_variant(id);

        let symbols = self.symbols.get(variant).body.clone();
        let mut cons = self.set_fields(&symbols);
        cons.push(swc::Stmt::Return(swc::ReturnStmt::default()));

        swc::SwitchCase {
            span: DUMMY_SP,
            test,
            cons,
        }
    }

    fn make_enum_setter_different_tags(&mut self, variants: &[VariantSymbolId]) -> swc::BlockStmt {
        let clear = self.make_variants_cleaners(variants).into();
        let fill = self.make_enum_object_assign().into();
        swc::BlockStmt {
            stmts: vec![clear, fill],
            ..Default::default()
        }
    }

    fn make_variants_cleaners(&mut self, variants: &[VariantSymbolId]) -> swc::SwitchStmt {
        let discriminant = member(swc::Expr::This(swc::ThisExpr { span: DUMMY_SP }), "$tag");
        let cases = variants
            .iter()
            .enumerate()
            .map(|(i, v)| self.make_variant_cleaner(i, *v))
            .collect();

        swc::SwitchStmt {
            span: DUMMY_SP,
            discriminant: Box::new(discriminant.into()),
            cases,
        }
    }

    /// Create a `delete` expression for each field of the variant
    fn make_variant_cleaner(&mut self, id: usize, variant: VariantSymbolId) -> swc::SwitchCase {
        let test = test_variant(id);

        let symbols = self.symbols.get(variant).body.clone();
        let mut cons = symbols
            .into_iter()
            .map(|s| self.delete_field(s))
            .collect::<Vec<_>>();
        cons.push(swc::Stmt::Break(swc::BreakStmt::default()));

        swc::SwitchCase {
            span: DUMMY_SP,
            test,
            cons,
        }
    }

    /// `Object.assign(this, src.$get())`
    fn make_enum_object_assign(&mut self) -> swc::ExprStmt {
        let object_assign = member(ident_from_str("Object").into(), "assign");
        let copied_source = swc::CallExpr {
            callee: swc::Callee::Expr(member(ident_from_str("src").into(), "$get").into()),
            ..Default::default()
        };
        let call = swc::CallExpr {
            callee: swc::Callee::Expr(Box::new(object_assign.into())),
            args: vec![
                swc::Expr::This(swc::ThisExpr { span: DUMMY_SP }).into(),
                swc::Expr::Call(copied_source).into(),
            ],
            ..Default::default()
        };
        swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(call.into()),
        }
    }

    fn delete_field(&self, symbol: MemberSymbolId) -> swc::Stmt {
        swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(swc::Expr::Unary(swc::UnaryExpr {
                span: DUMMY_SP,
                op: swc::UnaryOp::Delete,
                arg: Box::new(self.this_field(symbol)),
            })),
        })
    }
}

/// `this.$tag === src.$tag`
fn tags_test() -> swc::BinExpr {
    let this_tag = member(swc::Expr::This(swc::ThisExpr { span: DUMMY_SP }), "$tag");
    let src_tag = member(ident_from_str(SRC_NAME).into(), "$tag");
    swc::BinExpr {
        span: DUMMY_SP,
        op: swc::BinaryOp::EqEqEq,
        left: Box::new(this_tag.into()),
        right: Box::new(src_tag.into()),
    }
}

fn test_variant(id: usize) -> Option<Box<swc::Expr>> {
    Some(Box::new(swc::Expr::Lit(swc::Lit::Num(swc::Number {
        span: DUMMY_SP,
        value: id as f64,
        raw: None,
    }))))
}
