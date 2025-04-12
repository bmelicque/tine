use std::error::Error;
use swc_common::{sync::Lrc, SourceMap, DUMMY_SP};
use swc_ecma_ast as ast;
use swc_ecma_codegen::{text_writer::JsWriter, Config, Emitter};

use crate::ast::Node;

#[derive(Debug, Clone)]
pub struct TranspilerError {
    pub message: String,
}

impl std::fmt::Display for TranspilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Transpiler error: {}", self.message)
    }
}

impl Error for TranspilerError {}

pub fn node_to_swc_module(
    _source_map: &Lrc<SourceMap>,
    node: Node,
) -> Result<ast::Module, Box<dyn Error>> {
    match node {
        Node::Program(statements) => {
            let mut swc_stmts = Vec::new();

            for stmt in statements {
                if let Some(swc_stmt) = node_to_swc_stmt(stmt)? {
                    swc_stmts.push(swc_stmt);
                }
            }

            Ok(ast::Module {
                span: DUMMY_SP,
                body: swc_stmts,
                shebang: None,
            })
        }
        _ => Err(Box::new(TranspilerError {
            message: "Expected Program node at root".to_string(),
        })),
    }
}

fn node_to_swc_stmt(node: Node) -> Result<Option<ast::ModuleItem>, Box<dyn Error>> {
    match node {
        Node::VariableDeclaration {
            name,
            type_annotation: _,
            initializer,
        } => {
            let init = if let Some(expr) = initializer {
                let swc_expr = node_to_swc_expr(*expr)?;
                Some(Box::new(swc_expr))
            } else {
                None
            };

            let decl = ast::VarDeclarator {
                span: DUMMY_SP,
                name: ast::Pat::Ident(ast::BindingIdent {
                    id: ast::Ident {
                        span: DUMMY_SP,
                        sym: name.into(),
                        optional: false,
                    },
                    type_ann: None,
                }),
                init,
                definite: false,
            };

            Ok(Some(ast::ModuleItem::Stmt(ast::Stmt::Decl(
                ast::Decl::Var(Box::new(ast::VarDecl {
                    span: DUMMY_SP,
                    kind: ast::VarDeclKind::Let,
                    declare: false,
                    decls: vec![decl],
                })),
            ))))
        }
        Node::FunctionDeclaration {
            name,
            params,
            return_type: _,
            body,
        } => {
            let mut swc_params = Vec::new();

            for (param_name, _) in params {
                swc_params.push(ast::Pat::Ident(ast::BindingIdent {
                    id: ast::Ident {
                        span: DUMMY_SP,
                        sym: param_name.into(),
                        optional: false,
                    },
                    type_ann: None,
                }));
            }

            let mut swc_body_stmts = Vec::new();
            for stmt in body {
                if let Some(swc_stmt) = node_to_swc_stmt(stmt)? {
                    if let ast::ModuleItem::Stmt(s) = swc_stmt {
                        swc_body_stmts.push(s);
                    }
                }
            }

            let function = ast::Function {
                params: swc_params
                    .into_iter()
                    .map(|p| ast::Param {
                        span: DUMMY_SP,
                        decorators: vec![],
                        pat: p,
                    })
                    .collect(),
                decorators: vec![],
                span: DUMMY_SP,
                body: Some(ast::BlockStmt {
                    span: DUMMY_SP,
                    stmts: swc_body_stmts,
                }),
                is_generator: false,
                is_async: false,
                type_params: None,
                return_type: None,
            };

            Ok(Some(ast::ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Fn(
                ast::FnDecl {
                    ident: ast::Ident {
                        span: DUMMY_SP,
                        sym: name.into(),
                        optional: false,
                    },
                    declare: false,
                    function: Box::new(function),
                },
            )))))
        }
        Node::ReturnStatement(expr) => {
            let arg = if let Some(e) = expr {
                let swc_expr = node_to_swc_expr(*e)?;
                Some(Box::new(swc_expr))
            } else {
                None
            };

            Ok(Some(ast::ModuleItem::Stmt(ast::Stmt::Return(
                ast::ReturnStmt {
                    span: DUMMY_SP,
                    arg,
                },
            ))))
        }
        _ => Ok(None),
    }
}

fn node_to_swc_expr(node: Node) -> Result<ast::Expr, Box<dyn Error>> {
    match node {
        Node::BinaryExpression {
            left,
            operator,
            right,
        } => {
            let left_expr = node_to_swc_expr(*left)?;
            let right_expr = node_to_swc_expr(*right)?;

            let op = match operator.as_str() {
                "+" => ast::BinaryOp::Add,
                "-" => ast::BinaryOp::Sub,
                "*" => ast::BinaryOp::Mul,
                "/" => ast::BinaryOp::Div,
                "==" => ast::BinaryOp::EqEq,
                "!=" => ast::BinaryOp::NotEq,
                "<" => ast::BinaryOp::Lt,
                ">" => ast::BinaryOp::Gt,
                "<=" => ast::BinaryOp::LtEq,
                ">=" => ast::BinaryOp::GtEq,
                _ => {
                    return Err(Box::new(TranspilerError {
                        message: format!("Unknown operator: {}", operator),
                    }))
                }
            };

            Ok(ast::Expr::Bin(ast::BinExpr {
                span: DUMMY_SP,
                op,
                left: Box::new(left_expr),
                right: Box::new(right_expr),
            }))
        }
        Node::Identifier(name) => Ok(ast::Expr::Ident(ast::Ident {
            span: DUMMY_SP,
            sym: name.into(),
            optional: false,
        })),
        Node::StringLiteral(value) => Ok(ast::Expr::Lit(ast::Lit::Str(ast::Str {
            span: DUMMY_SP,
            value: value.into(),
            raw: None,
        }))),
        Node::NumberLiteral(value) => Ok(ast::Expr::Lit(ast::Lit::Num(ast::Number {
            span: DUMMY_SP,
            value,
            raw: None,
        }))),
        Node::BooleanLiteral(value) => Ok(ast::Expr::Lit(ast::Lit::Bool(ast::Bool {
            span: DUMMY_SP,
            value,
        }))),
        _ => Err(Box::new(TranspilerError {
            message: format!("Unsupported expression: {:?}", node),
        })),
    }
}

pub struct Transpiler {
    pub source_map: Lrc<SourceMap>,
}

impl Transpiler {
    pub fn new() -> Self {
        Self {
            source_map: Lrc::new(SourceMap::new(Default::default())),
        }
    }

    pub fn generate_js(&self, node: Node) -> Result<String, Box<dyn Error>> {
        let program = node_to_swc_module(&self.source_map, node)?;

        let mut buf = Vec::new();
        {
            let writer = JsWriter::new(self.source_map.clone(), "\n", &mut buf, None);
            let mut emitter = Emitter {
                cfg: Config::default(),
                cm: self.source_map.clone(),
                comments: None,
                wr: writer,
            };

            emitter.emit_module(&program)?;
        }

        let js_code = String::from_utf8(buf)?;
        Ok(js_code)
    }
}
