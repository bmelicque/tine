use crate::codegen::utils::create_ident;
use swc_common::{sync::Lrc, SourceMap, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::{ModuleId, ModulePath, Session};

pub struct CodeGenerator<'sess> {
    _source_map: Lrc<SourceMap>,

    session: &'sess Session,
    pub(crate) module: ModuleId,
    /// Should the `break` statements be converted to `return` statements.
    /// This is used when generating `for` and `for ... in` expressions, which are translated to IIFEs.
    next_temp_id: usize,

    // Used when replacing `break X` by `TARGET = X; break`
    pub(crate) break_target: Option<swc::Ident>,
}

impl CodeGenerator<'_> {
    pub fn new<'sess>(session: &'sess Session, module: ModuleId) -> CodeGenerator<'sess> {
        CodeGenerator {
            session,
            module,
            _source_map: Lrc::new(SourceMap::new(Default::default())),
            next_temp_id: 0,
            break_target: None,
        }
    }

    pub fn program_to_swc_module(&mut self) -> swc::Module {
        let node = self.session.get_ir(self.module);
        let items: Vec<swc::ModuleItem> = node
            .statements
            .iter()
            .flat_map(|item| self.item_to_swc(item))
            .collect();

        let internals_import =
            swc::ModuleItem::ModuleDecl(swc::ModuleDecl::Import(swc::ImportDecl {
                span: DUMMY_SP,
                specifiers: vec![swc::ImportSpecifier::Namespace(
                    swc_ecma_ast::ImportStarAsSpecifier {
                        span: DUMMY_SP,
                        local: create_ident("$"),
                    },
                )],
                src: Box::new(swc::Str {
                    span: DUMMY_SP,
                    value: "$internals".into(),
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

    pub(crate) fn with_break_target<F, T>(&mut self, target: swc::Ident, callback: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        let mem = self.break_target.clone();
        self.break_target = Some(target);
        let ret = callback(self);
        self.break_target = mem;
        ret
    }

    pub(crate) fn get_temp_id(&mut self) -> swc::Ident {
        let ident = create_ident(&format!("$_{}", self.next_temp_id));
        self.next_temp_id += 1;
        ident
    }
}
