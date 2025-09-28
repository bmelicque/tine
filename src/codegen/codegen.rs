use pest::Span;
use std::error::Error;
use swc_common::{sync::Lrc, SourceMap, DUMMY_SP};
use swc_ecma_ast as swc;

use crate::{
    ast,
    codegen::utils::create_ident,
    type_checker::{self, Symbol},
    types,
};

use super::sort::Scope;

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
    scope: Scope,
    _source_map: Lrc<SourceMap>,
    current_block: Vec<Vec<swc::Stmt>>,
    analysis_context: type_checker::AnalysisContext,
}

impl CodeGenerator {
    pub fn new(context: type_checker::AnalysisContext) -> Self {
        Self {
            scope: Scope::new(),
            _source_map: Lrc::new(SourceMap::new(Default::default())),
            current_block: vec![],
            analysis_context: context,
        }
    }

    pub fn program_to_swc_module(&mut self, node: ast::Program) -> swc::Module {
        let items: Vec<swc::ModuleItem> = self.with_scope(|s| {
            node.items
                .into_iter()
                .flat_map(|item| s.item_to_swc(item))
                .collect()
        });

        let internals_import =
            swc::ModuleItem::ModuleDecl(swc::ModuleDecl::Import(swc::ImportDecl {
                span: DUMMY_SP,
                specifiers: vec![swc::ImportSpecifier::Namespace(
                    swc_ecma_ast::ImportStarAsSpecifier {
                        span: DUMMY_SP,
                        local: create_ident("__"),
                    },
                )],
                src: Box::new(swc::Str {
                    span: DUMMY_SP,
                    value: "internals".into(),
                    raw: None,
                }),
                type_only: false,
                with: None,
                phase: swc::ImportPhase::Source,
            }));

        let mut body: Vec<swc::ModuleItem> = Vec::new();
        body.push(internals_import);
        body.extend(items);

        swc::Module {
            span: DUMMY_SP,
            body,
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

    pub fn add_to_scope(&mut self, name: String, fields: Vec<String>) {
        self.scope.register(name, fields);
    }
    pub fn push_scope(&mut self) {
        self.scope.enter();
    }
    pub fn drop_scope(&mut self) {
        self.scope.exit();
    }
    pub fn with_scope<F, T>(&mut self, predicate: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.scope.enter();
        let res = predicate(self);
        self.scope.exit();
        res
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

    pub fn get_expr_type(&self, node: &ast::Expression) -> Option<&types::Type> {
        self.analysis_context.types.get(&node.as_span())
    }
}
