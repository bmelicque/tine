use super::sort::Scope;
use crate::codegen::utils::create_ident;
use swc_common::{sync::Lrc, SourceMap, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::{ast, types, Location, ModuleId, ModulePath, Session, SymbolRef};

pub struct CodeGenerator<'sess> {
    scope: Scope,
    _source_map: Lrc<SourceMap>,
    current_block: Vec<Vec<swc::Stmt>>,

    session: &'sess Session,
    pub(crate) module: ModuleId,
}

impl CodeGenerator<'_> {
    pub fn new<'sess>(session: &'sess Session, module: ModuleId) -> CodeGenerator<'sess> {
        CodeGenerator {
            session,
            module,
            scope: Scope::new(),
            _source_map: Lrc::new(SourceMap::new(Default::default())),
            current_block: vec![],
        }
    }

    pub fn program_to_swc_module(&mut self) -> swc::Module {
        let node = self.session.get_ast(self.module);
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

    pub fn get_filename(&self) -> &ModulePath {
        let module = self.session.read_module(self.module);
        &module.name
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

    pub fn find_symbol(&self, loc: Location) -> Option<SymbolRef> {
        self.session
            .symbols()
            .iter()
            .find(|s| s.uses().into_iter().find(|&l| l == loc).is_some())
            .cloned()
    }

    pub fn get_reactive_dependencies(&self, loc: Location) -> Vec<SymbolRef> {
        let Some(deps) = self.session.get_dependencies(loc) else {
            return vec![];
        };
        deps.iter()
            .filter(|dep| self.session.get_type(dep.borrow().get_type()).is_reactive())
            .cloned()
            .collect()
    }

    pub fn get_expr_type(&self, node: &ast::Expression) -> Option<types::Type> {
        self.session.get_type_at(node.loc())
    }
    pub fn get_type_at(&self, loc: Location) -> Option<types::Type> {
        self.session.get_type_at(loc)
    }
}
