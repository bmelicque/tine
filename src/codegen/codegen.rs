use bitflags::bitflags;
use std::{collections::HashMap, error::Error};
use swc_common::{sync::Lrc, SourceMap, DUMMY_SP};
use swc_ecma_ast as swc;
use swc_ecma_codegen::{text_writer::JsWriter, Config, Emitter};

use crate::ast;

use super::utils::get_option_class;

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
    }
}

pub struct CodeGenerator {
    class_defs: HashMap<String, swc::ClassDecl>,
    pub source_map: Lrc<SourceMap>,
    flags: TranspilerFlags,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            class_defs: HashMap::new(),
            source_map: Lrc::new(SourceMap::new(Default::default())),
            flags: TranspilerFlags::None,
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
        let mut swc_stmts: Vec<swc::ModuleItem> = node
            .statements
            .into_iter()
            .filter_map(|stmt| self.stmt_to_swc(stmt))
            .map(|stmt| stmt.into())
            .collect();

        for flag in self.flags {
            match flag {
                TranspilerFlags::OptionType => swc_stmts.push(get_option_class().into()),
                _ => {}
            }
        }

        swc::Module {
            span: DUMMY_SP,
            body: swc_stmts,
            shebang: None,
        }
    }

    pub fn add_flag(&mut self, flag: TranspilerFlags) {
        self.flags = self.flags | flag;
    }

    pub fn add_class_def(&mut self, name: String, class_decl: swc::ClassDecl) {
        self.class_defs.insert(name, class_decl);
    }

    pub fn get_class_def(&self, name: &str) -> Option<&swc::ClassDecl> {
        self.class_defs.get(name)
    }
}
