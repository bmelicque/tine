use bitflags::bitflags;
use pest::Span;
use std::error::Error;
use swc_common::{sync::Lrc, SourceMap, DUMMY_SP};
use swc_ecma_ast as swc;
use swc_ecma_codegen::{text_writer::JsWriter, Config, Emitter};

use crate::{
    ast,
    codegen::builtin::{create_element, reactive, reference},
    type_checker::{self, Symbol},
};

use super::{sort::Scope, utils::get_option_class};

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

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TranspilerFlags: u32 {
        const None = 0;
        const OptionType = 1;
        const CreateElement = 2;
        const Reference = 4;
        const Reactive = 8;
    }
}

pub struct CodeGenerator {
    scope: Scope,
    pub source_map: Lrc<SourceMap>,
    flags: TranspilerFlags,
    current_block: Vec<Vec<swc::Stmt>>,
    analysis_context: type_checker::AnalysisContext,
}

impl CodeGenerator {
    pub fn new(context: type_checker::AnalysisContext) -> Self {
        Self {
            scope: Scope::new(),
            source_map: Lrc::new(SourceMap::new(Default::default())),
            flags: TranspilerFlags::None,
            current_block: vec![],
            analysis_context: context,
        }
    }

    pub fn generate_js(&mut self, node: ast::Program) -> Result<String, Box<dyn Error>> {
        let program = self.node_to_swc_module(node);

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

    fn node_to_swc_module(&mut self, node: ast::Program) -> swc::Module {
        self.enter_block();
        for stmt in node.statements {
            let stmts = self.stmt_to_swc(stmt);
            for stmt in stmts {
                self.push_to_block(stmt);
            }
        }
        let mut swc_stmts = self.exit_block();

        for flag in self.flags {
            match flag {
                TranspilerFlags::OptionType => swc_stmts.push(get_option_class().into()),
                TranspilerFlags::CreateElement => swc_stmts.append(create_element().as_mut()),
                TranspilerFlags::Reference => swc_stmts.append(reference().as_mut()),
                TranspilerFlags::Reactive => {
                    let _ = reactive();
                    todo!()
                }
                _ => {}
            }
        }

        swc::Module {
            span: DUMMY_SP,
            body: swc_stmts.into_iter().map(|stmt| stmt.into()).collect(),
            shebang: None,
        }
    }

    pub fn enter_block(&mut self) {
        self.push_scope();
        self.current_block.push(Vec::<swc::Stmt>::new());
    }
    pub fn exit_block(&mut self) -> Vec<swc::Stmt> {
        self.drop_scope();
        self.current_block.pop().unwrap()
    }
    pub fn push_to_block(&mut self, stmt: swc::Stmt) {
        self.current_block.last_mut().unwrap().push(stmt);
    }

    pub fn add_flag(&mut self, flag: TranspilerFlags) {
        self.flags = self.flags | flag;
    }

    pub fn add_to_scope(&mut self, name: String, fields: Vec<String>) {
        self.scope.register(name, fields);
    }
    pub fn push_scope(&mut self) {
        self.scope.enter();
    }
    pub fn drop_scope(&mut self) {
        self.scope.exit();
    }
    pub fn find(&self, name: &String) -> Option<&Vec<String>> {
        self.scope.find(name)
    }

    pub fn get_info(&self, name: &str) -> Option<&Symbol> {
        self.analysis_context.lookup(name)
    }

    pub fn get_expression_dependencies(&self, span: Span<'static>) -> Vec<&Symbol> {
        let Some(list) = self.analysis_context.other_dependencies.get(&span) else {
            return Vec::new();
        };
        list.into_iter()
            .map(|id| &self.analysis_context.symbols[*id])
            .collect()
    }
}
