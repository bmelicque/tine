use std::error::Error;
use swc_common::{sync::Lrc, SourceMap, DUMMY_SP};
use swc_ecma_ast as ast;
use swc_ecma_codegen::{text_writer::JsWriter, Config, Emitter};

use crate::ast::{AstNode, Node};

use super::statements::node_to_swc_stmt;

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

pub struct CodeGenerator {
    pub source_map: Lrc<SourceMap>,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            source_map: Lrc::new(SourceMap::new(Default::default())),
        }
    }

    pub fn generate_js(&self, node: AstNode) -> Result<String, Box<dyn Error>> {
        let program = self.node_to_swc_module(node.node)?;

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

        Ok(String::from_utf8(buf)?)
    }

    fn node_to_swc_module(&self, node: Node) -> Result<ast::Module, Box<dyn Error>> {
        match node {
            Node::Program(statements) => {
                let mut swc_stmts = Vec::new();

                for stmt in statements {
                    if let Some(swc_stmt) = node_to_swc_stmt(&self, stmt.node)? {
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
}
