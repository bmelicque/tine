use super::sort::Scope;
use crate::codegen::utils::create_ident;
use mylang_core::{ast, types, CheckedModule, SymbolRef, Token};
use pest::Span;
use swc_common::{sync::Lrc, FileName, SourceMap, DUMMY_SP};
use swc_ecma_ast as swc;

pub struct CodeGenerator {
    filename: FileName,
    scope: Scope,
    _source_map: Lrc<SourceMap>,
    current_block: Vec<Vec<swc::Stmt>>,
    pub(crate) module: CheckedModule,
}

impl CodeGenerator {
    pub fn new(filename: FileName, metadata: CheckedModule) -> Self {
        Self {
            filename,
            scope: Scope::new(),
            _source_map: Lrc::new(SourceMap::new(Default::default())),
            current_block: vec![],
            module: metadata,
        }
    }

    pub fn program_to_swc_module(&mut self, node: &ast::Program) -> swc::Module {
        self.enter_block();
        let items: Vec<swc::ModuleItem> = self.with_scope(|s| {
            node.items
                .iter()
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

    pub fn get_filename(&self) -> &FileName {
        &self.filename
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

    pub fn get_info(&self, span: Span<'static>) -> Option<SymbolRef> {
        self.module
            .metadata
            .tokens
            .get(&span)
            .map(|token| match token {
                Token::Member(_) => None,
                Token::Symbol(symbol) => Some(symbol.symbol.clone()),
            })
            .flatten()
    }

    pub fn get_reactive_dependencies(&self, span: Span<'static>) -> Vec<SymbolRef> {
        let Some(deps) = self.module.metadata.dependencies.get(&span) else {
            return vec![];
        };
        deps.iter()
            .filter(|dep| {
                self.module
                    .metadata
                    .resolve_type(dep.borrow().get_type())
                    .is_reactive()
            })
            .cloned()
            .collect()
    }

    pub fn get_expr_type(&self, node: &ast::Expression) -> Option<types::Type> {
        let type_id = self.module.metadata.expressions.get(&node.as_span());
        type_id
            .map(|id| self.module.metadata.type_store.get(*id))
            .cloned()
    }
}
