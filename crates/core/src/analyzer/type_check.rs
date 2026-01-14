use std::{collections::HashMap, rc::Rc};

use crate::{
    analyzer::{session::Session, ModuleId, ModulePath},
    locations::Span,
    type_checker::{CheckResult, TypeChecker},
    types::{DuckType, FunctionType, Type, TypeId},
    SymbolData, SymbolKind, SymbolRef, Token, TypeStore,
};

#[derive(Debug, Clone)]
pub struct ModuleTypeData {
    pub type_store: Rc<TypeStore>,
    pub exports: Vec<SymbolRef>,
    pub expressions: HashMap<Span, u32>,
    pub tokens: HashMap<Span, Token>,
    pub dependencies: HashMap<Span, Vec<SymbolRef>>,
}

impl ModuleTypeData {
    pub fn resolve_type(&self, id: TypeId) -> &Type {
        self.type_store.get(id)
    }
}

impl Session {
    pub fn check_project(&mut self, sorted_modules: &[ModuleId]) {
        for &module_id in sorted_modules {
            let mut result = self.check_module(module_id);
            self.diagnostics
                .get_mut(&module_id)
                .unwrap()
                .append(&mut result.diagnostics);
            self.symbols.append(&mut result.symbols);
            self.exports.insert(module_id, result.exports);
            self.add_expressions(result.expressions);
            self.add_dependencies(result.dependencies);
        }
    }

    fn check_module(&mut self, id: ModuleId) -> CheckResult {
        let module = &self.module_graph.nodes[id];
        match module.name.clone() {
            ModulePath::Real(_) => {
                let checker = TypeChecker::new(&self, id);
                checker.check()
            }
            ModulePath::Virtual(name) => self.check_virtual_module(id, &name),
        }
    }

    fn check_virtual_module(&mut self, id: ModuleId, name: &str) -> CheckResult {
        match name {
            "dom" => self.check_dom_module(id),
            _ => panic!("unexpected module name '{}'", name),
        }
    }

    fn check_dom_module(&mut self, id: ModuleId) -> CheckResult {
        let mut checker = TypeChecker::new(&self, id);
        let element_trait = checker.intern(Type::Duck(DuckType {
            like: TypeStore::ELEMENT,
        }));
        let render_type = checker.intern(Type::Function(FunctionType {
            params: vec![TypeStore::STRING, element_trait],
            return_type: TypeStore::UNIT,
        }));

        checker.ctx.register_symbol(SymbolData {
            name: "render".into(),
            docs: Some(
                r#"Renders a UI element into a target container in the DOM

# Example
```tine
render("body", <article>Content</article>)
```

In this example, the `<article>` element is rendered inside the document's body.
        "#
                .into(),
            ),
            ty: render_type,
            kind: SymbolKind::Function {
                param_names: vec!["selector".into(), "element".into()],
            },
            ..Default::default()
        });

        let main_scope = &checker.ctx.scopes[0];

        CheckResult {
            symbols: checker.ctx.symbols,
            exports: main_scope.bindings.clone(),
            expressions: HashMap::new(),
            dependencies: HashMap::new(),
            diagnostics: vec![],
        }
    }
}
