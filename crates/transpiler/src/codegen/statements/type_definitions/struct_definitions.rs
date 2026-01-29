use crate::codegen::{
    statements::type_definitions::utils::{object_assign_this, this_assignment},
    utils::{create_block_stmt, create_ident},
    CodeGenerator,
};
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::ast;

impl CodeGenerator<'_> {
    /// Emits a JavaScript class from a struct definition
    ///
    /// Struct fields become constructor parameters, which are then assigned to `this`.
    ///
    /// For example:
    /// ```tine
    /// struct Box<Type>(Type)
    ///
    /// struct Vector {
    ///     x float
    ///     y float
    /// }
    /// ```
    /// Becomes:
    /// ```javascript
    /// class Box {
    ///     constructor(...args) {
    ///         Object.assign(this, args)
    ///     }
    /// }
    ///
    /// class Vector {
    ///     constructor(x, y) {
    ///         this.x = x
    ///         this.y = y
    ///     }
    /// }
    /// ```
    pub fn struct_def_to_swc_class(&mut self, node: &ast::StructDefinition) -> swc::ClassDecl {
        let body = match &node.body {
            ast::TypeBody::Struct(body) => {
                self.register_struct(&node.name, &body.fields);
                self.struct_body_to_swc_constructor(body)
            }
            ast::TypeBody::Tuple(_) => self.tuple_body_to_swc_constructor(),
        };

        swc::ClassDecl {
            declare: false,
            ident: create_ident(&node.name),
            class: Box::new(swc::Class {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                body: vec![body.into()],
                super_class: None,
                super_type_params: None,
                decorators: vec![],
                type_params: None,
                is_abstract: false,
                implements: vec![],
            }),
        }
    }

    /// Emits the JavaScript class constructor from a struct body with named fields.
    ///
    /// ```tine
    /// struct {
    ///     x float
    ///     y float
    ///     z = 0.
    /// }
    /// ```
    /// Becomes:
    /// ```javascript
    /// constructor(x, y, z = 0) {
    ///     this.x = x
    ///     this.y = y
    ///     this.z = z
    /// }
    /// ```
    fn struct_body_to_swc_constructor(&mut self, node: &ast::StructBody) -> swc::Constructor {
        let mandatory_fields: Vec<&ast::StructDefinitionField> = node
            .fields
            .iter()
            .filter(|field| !field.is_optional())
            .collect();
        let optional_fields: Vec<&ast::StructDefinitionField> = node
            .fields
            .iter()
            .filter(|field| field.is_optional())
            .collect();
        let params = mandatory_fields
            .iter()
            .chain(optional_fields.iter())
            .map(self.struct_field_to_swc_param())
            .collect::<Vec<_>>();

        swc::Constructor {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            key: create_ident("constructor").into(),
            is_optional: false,
            params,
            body: Some(create_block_stmt(struct_to_swc_constructor_stmts(
                &node.fields,
            ))),
            accessibility: None,
        }
    }

    fn struct_field_to_swc_param(
        &mut self,
    ) -> Box<dyn for<'b> FnMut(&'b &ast::StructDefinitionField) -> swc::ParamOrTsParamProp + '_>
    {
        Box::new(move |field| {
            let pattern = match field {
                ast::StructDefinitionField::Mandatory(field) => {
                    swc::Pat::Ident(swc::BindingIdent {
                        id: create_ident(&field.name),
                        type_ann: None,
                    })
                }
                ast::StructDefinitionField::Optional(field) => swc::Pat::Assign(swc::AssignPat {
                    span: DUMMY_SP,
                    left: Box::new(swc::Pat::Ident(swc::BindingIdent {
                        id: create_ident(&field.name),
                        type_ann: None,
                    })),
                    right: Box::new(self.expr_to_swc(&field.default)),
                }),
            };

            swc::ParamOrTsParamProp::Param(swc::Param {
                span: DUMMY_SP,
                decorators: vec![],
                pat: pattern,
            })
        })
    }

    fn tuple_body_to_swc_constructor(&mut self) -> swc::Constructor {
        let params = vec![swc::ParamOrTsParamProp::Param(swc::Param {
            span: DUMMY_SP,
            decorators: vec![],
            pat: swc::Pat::Rest(swc::RestPat {
                span: DUMMY_SP,
                dot3_token: DUMMY_SP,
                arg: Box::new(swc::Pat::Ident(create_ident("_").into())),
                type_ann: None,
            }),
        })];
        let body = Some(create_block_stmt(vec![swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(object_assign_this("_")),
        })]));

        swc::Constructor {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            key: create_ident("constructor").into(),
            is_optional: false,
            params,
            body,
            accessibility: None,
        }
    }
}

fn struct_to_swc_constructor_stmts(fields: &Vec<ast::StructDefinitionField>) -> Vec<swc::Stmt> {
    fields
        .iter()
        .map(|field| this_assignment(&field.as_name()))
        .collect()
}
