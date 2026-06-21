use crate::{
    codegen::utils::ident_from_str,
    ownership_analyser::{analyse_program, OwnershipAction, OwnershipMap},
};
use swc_common::{sync::Lrc, SourceMap, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::{ir, types, ModuleId, ModulePath, Session, SymbolRef};

pub struct CodeGenerator<'sess> {
    _source_map: Lrc<SourceMap>,

    ownership: OwnershipMap,
    pub(super) session: &'sess Session,
    pub(crate) module: ModuleId,
    /// Should the `break` statements be converted to `return` statements.
    /// This is used when generating `for` and `for ... in` expressions, which are translated to IIFEs.
    next_temp_id: usize,

    // Used when replacing `break X` by `TARGET = X; break`
    pub(crate) break_target: Option<swc::Ident>,

    pub(crate) this_stack: Vec<SymbolRef>,
}

impl CodeGenerator<'_> {
    pub fn new<'sess>(session: &'sess Session, module: ModuleId) -> CodeGenerator<'sess> {
        CodeGenerator {
            ownership: OwnershipMap::default(),
            session,
            module,
            _source_map: Lrc::new(SourceMap::new(Default::default())),
            next_temp_id: 0,
            break_target: None,
            this_stack: vec![],
        }
    }

    pub fn program_to_swc_module(&mut self) -> swc::Module {
        let node = self.session.get_ir(self.module);
        self.ownership = analyse_program(node, self.session);
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
                        local: ident_from_str("$"),
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
        let ident = ident_from_str(&format!("$_{}", self.next_temp_id));
        self.next_temp_id += 1;
        ident
    }

    pub(crate) fn resolve(&self, ty: types::TypeId) -> types::Type {
        self.session.get_type(ty)
    }

    pub(crate) fn with_this<F, T>(&mut self, this: SymbolRef, callback: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.this_stack.push(this);
        let ret = callback(self);
        self.this_stack.pop();
        ret
    }
    pub(crate) fn is_current_this(&self, expr: &ir::Expression) -> bool {
        match expr {
            ir::Expression::Identifier(id) => Some(&id.symbol) == self.this_stack.last(),
            _ => false,
        }
    }

    pub(crate) fn identifier_ownership(&self, id: &ir::Identifier) -> OwnershipAction {
        self.ownership.action_for(id.loc, OwnershipAction::Clone)
    }
    pub(crate) fn call_ownership(&self, call: &ir::CallExpression) -> OwnershipAction {
        self.ownership.action_for(call.loc, OwnershipAction::Move)
    }
}
