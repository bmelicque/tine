use crate::codegen::utils::create_ident;
use swc_common::{sync::Lrc, SourceMap, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::{Location, ModuleId, ModulePath, Session, SymbolRef};

pub struct CodeGenerator<'sess> {
    _source_map: Lrc<SourceMap>,

    session: &'sess Session,
    pub(crate) module: ModuleId,
    /// Should the `break` statements be converted to `return` statements.
    /// This is used when generating `for` and `for ... in` expressions, which are translated to IIFEs.
    breaks_to_returns: bool,
}

impl CodeGenerator<'_> {
    pub fn new<'sess>(session: &'sess Session, module: ModuleId) -> CodeGenerator<'sess> {
        CodeGenerator {
            session,
            module,
            _source_map: Lrc::new(SourceMap::new(Default::default())),
            breaks_to_returns: false,
        }
    }

    pub fn program_to_swc_module(&mut self) -> swc::Module {
        let node = self.session.get_ir(self.module);
        let items: Vec<swc::ModuleItem> = node
            .statements
            .iter()
            .map(|item| self.item_to_swc(item))
            .collect();

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

    pub(crate) fn with_breaks_to_returns<F, T>(&mut self, callback: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        let old = self.breaks_to_returns;
        self.breaks_to_returns = true;
        let ret = callback(self);
        self.breaks_to_returns = old;
        ret
    }

    pub fn find_symbol(&self, loc: Location) -> Option<SymbolRef> {
        self.session
            .symbols()
            .iter()
            .find(|s| s.uses().into_iter().find(|&l| l == loc).is_some())
            .cloned()
    }
}
