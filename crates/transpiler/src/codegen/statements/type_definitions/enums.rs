use crate::codegen::{
    statements::type_definitions::utils::{object_assign_this, this_assignment},
    utils::{create_block_stmt, create_ident},
    CodeGenerator,
};
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::ast;

impl CodeGenerator<'_> {
    /// Emits a JavaScript class from an enum
    ///
    /// The class will consist of a `__` key which holds the tag, then the properties.
    /// The constructor will switch over the tag to handle props correctly.
    ///
    /// For example:
    /// ```tine
    /// enum Example {
    ///     Nothing
    ///     Something(int)
    ///     SomeThings {
    ///         id   int
    ///         name str
    ///     }
    /// }
    /// ```
    ///
    /// will be translated as follows:
    /// ```javascript
    /// class Example {
    ///     constructor(__, ...args) {
    ///         this.__ = __
    ///         switch (__) {
    ///         case "Nothing":
    ///             break
    ///         case "Something":
    ///             Object.assign(this, args)
    ///             break
    ///         case "SomeThings":
    ///             this.id = args[0]
    ///             this.name = args[1]
    ///             break
    ///         }
    ///     }
    /// }
    ///
    /// // Expecting `Example.Something(42)` to be instantiated as:
    /// new Example("Something", 42)
    /// ```
    pub fn enum_to_swc(&mut self, node: &ast::EnumDefinition) -> swc::ClassDecl {
        for variant in node.variants.iter() {
            if let Some(ast::TypeBody::Struct(ast::StructBody { ref fields, .. })) = variant.body {
                self.register_struct(&format!("{}.{}", node.name, &variant.name), fields);
            }
        }

        swc::ClassDecl {
            declare: false,
            ident: create_ident(&node.name),
            class: Box::new(swc::Class {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                decorators: vec![],
                body: vec![self.enum_constructor(&node.variants).into()],
                implements: vec![],
                super_class: None,
                is_abstract: false,
                type_params: None,
                super_type_params: None,
            }),
        }
    }

    fn enum_constructor(&self, variants: &Vec<ast::VariantDefinition>) -> swc::Constructor {
        let tag_param = swc::ParamOrTsParamProp::Param(swc::Param {
            span: DUMMY_SP,
            decorators: vec![],
            pat: swc::Pat::Ident(create_ident("__").into()),
        });
        let member_params = swc::ParamOrTsParamProp::Param(swc::Param {
            span: DUMMY_SP,
            decorators: vec![],
            pat: swc::Pat::Rest(swc::RestPat {
                span: DUMMY_SP,
                dot3_token: DUMMY_SP,
                arg: Box::new(swc::Pat::Ident(create_ident("args").into())),
                type_ann: None,
            }),
        });

        let body = Some(create_block_stmt(vec![
            this_assignment("__"),
            swc::Stmt::Switch(swc::SwitchStmt {
                span: DUMMY_SP,
                discriminant: Box::new(create_ident("__").into()),
                cases: variants
                    .into_iter()
                    .map(|v| self.variant_to_swc_switch_case(v))
                    .collect(),
            }),
        ]));

        swc::Constructor {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            key: create_ident("constructor").into(),
            params: vec![tag_param, member_params],
            body,
            accessibility: None,
            is_optional: false,
        }
    }

    fn variant_to_swc_switch_case(&self, variant: &ast::VariantDefinition) -> swc::SwitchCase {
        let mut stmts = match &variant.body {
            Some(ast::TypeBody::Struct(body)) => body
                .fields
                .iter()
                .enumerate()
                .map(|(i, field)| self.this_assignement_from_values(&field.as_name(), i as f64))
                .collect::<Vec<_>>(),
            Some(ast::TypeBody::Tuple(_)) => vec![swc::Stmt::Expr(swc::ExprStmt {
                span: DUMMY_SP,
                expr: object_assign_this("__").into(),
            })],
            None => vec![],
        };
        stmts.push(swc::Stmt::Break(swc::BreakStmt {
            span: DUMMY_SP,
            label: None,
        }));

        swc::SwitchCase {
            span: DUMMY_SP,
            test: Some(Box::new(swc::Expr::Lit(swc::Lit::Str(swc::Str {
                span: DUMMY_SP,
                value: variant.name.clone().into(),
                raw: None,
            })))),
            cons: stmts,
        }
    }

    fn this_assignement_from_values(&self, name: &str, index: f64) -> swc::Stmt {
        let span = DUMMY_SP;
        let expr = Box::new(swc::Expr::Assign(swc::AssignExpr {
            span: DUMMY_SP,
            op: swc::AssignOp::Assign,
            left: swc::AssignTarget::Simple(swc::SimpleAssignTarget::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(swc::ThisExpr { span: DUMMY_SP }.into()),
                prop: swc::MemberProp::Ident(create_ident(name).into()),
            })),
            right: Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(create_ident("values").into()),
                prop: swc::MemberProp::Computed(swc::ComputedPropName {
                    span: DUMMY_SP,
                    expr: Box::new(swc::Expr::Lit(swc::Lit::Num(swc::Number {
                        span: DUMMY_SP,
                        value: index,
                        raw: None,
                    }))),
                }),
            })),
        }));
        swc::ExprStmt { span, expr }.into()
    }
}
