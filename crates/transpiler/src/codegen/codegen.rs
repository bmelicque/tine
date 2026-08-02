use crate::{
    codegen::utils::ident_from_str,
    ownership_analyser::{analyse_program, OwnershipAction, OwnershipMap},
};
use swc_common::{sync::Lrc, SourceMap, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_common::module_path::ModulePath;
use tine_ir as ir;
use tine_symbols::{symbols::*, table::SymbolTable};
use tine_types::{store::TypeStore, types};

pub struct CodeGenerator<'ty, 'sym> {
    _source_map: Lrc<SourceMap>,

    pub types: &'ty TypeStore,
    pub symbols: &'sym SymbolTable,
    ownership: OwnershipMap,
    pub(super) name: ModulePath,
    /// Should the `break` statements be converted to `return` statements.
    /// This is used when generating `for` and `for ... in` expressions, which are translated to IIFEs.
    next_temp_id: usize,

    // Used when replacing `break X` by `TARGET = X; break`
    pub(crate) break_target: Option<swc::Ident>,

    pub(crate) this_stack: Vec<VariableSymbolId>,
}

impl CodeGenerator<'_, '_> {
    pub fn new<'ty, 'sym>(
        name: ModulePath,
        types: &'ty TypeStore,
        symbols: &'sym SymbolTable,
    ) -> CodeGenerator<'ty, 'sym> {
        CodeGenerator {
            _source_map: Lrc::new(SourceMap::new(Default::default())),

            types,
            symbols,
            ownership: OwnershipMap::default(),
            name,
            next_temp_id: 0,
            break_target: None,
            this_stack: vec![],
        }
    }

    pub fn program_to_swc_module(&mut self, ir: ir::Program) -> swc::Module {
        self.ownership = analyse_program(&ir, self.types, self.symbols);
        let items: Vec<swc::ModuleItem> = ir
            .statements
            .into_iter()
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

    pub(super) fn symbol_name<I>(&self, id: I) -> &str
    where
        I: Into<SymbolId>,
    {
        self.symbols.get_symbol(id.into()).name()
    }

    pub(super) fn symbol_type_id<I>(&self, id: I) -> types::TypeId
    where
        I: Into<SymbolId>,
    {
        self.symbols.get_symbol(id.into()).ty()
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

    pub(crate) fn resolve(&self, ty: types::TypeId) -> &types::Type {
        self.types.get(ty)
    }

    pub(crate) fn with_this<F, T>(&mut self, this: VariableSymbolId, callback: F) -> T
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
            ir::Expression::Identifier(id) => {
                Some(id.symbol) == self.this_stack.last().copied().map(Into::into)
            }
            _ => false,
        }
    }

    pub(crate) fn identifier_ownership(&self, id: &ir::Identifier) -> OwnershipAction {
        self.ownership.action_for(id.loc, OwnershipAction::Clone)
    }
    pub(crate) fn call_ownership(&self, call: &ir::CallExpression) -> OwnershipAction {
        self.ownership.action_for(call.loc, OwnershipAction::Move)
    }
    pub(crate) fn method_ownership(&self, call: &ir::MethodExpression) -> OwnershipAction {
        self.ownership.action_for(call.loc, OwnershipAction::Move)
    }
}
