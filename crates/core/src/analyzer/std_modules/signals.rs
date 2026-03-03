use std::collections::HashMap;

use crate::{
    type_checker::{CheckResult, TypeChecker},
    types::{FunctionType, GenericType, SignalType, Type, TypeParam},
    ModuleId, Session, SymbolData, SymbolKind,
};

impl Session {
    pub fn check_signals_module(&mut self, id: ModuleId) -> CheckResult {
        let mut checker = TypeChecker::new(&self, id);

        register_state_symbol(&mut checker);
        register_derived_symbol(&mut checker);

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

fn register_state_symbol(checker: &mut TypeChecker) {
    let param_type = checker.intern(Type::Param(TypeParam {
        name: "Type".into(),
        idx: 0,
    }));
    let return_type = checker.intern(Type::Signal(SignalType { inner: param_type }));
    let state_def_type = checker.intern_unique(Type::Function(FunctionType {
        params: vec![param_type],
        return_type,
    }));
    let state_type = checker.intern(Type::Generic(GenericType {
        params: vec![param_type],
        definition: state_def_type,
    }));
    checker.ctx.register_symbol(SymbolData {
        name: "state".to_string(),
        ty: state_type,
        kind: SymbolKind::Function {
            param_names: vec!["initialValue".to_string()],
        },
        docs: Some(
            r#"Creates a reactive state variable.
            
The underlying value can be accessed and modified using the dereference operator `*`.

# Example
```tine
const counter = state(0)
const counterValue = *counter

fn reset() {
    *counter = 0
}
```
"#
            .to_string(),
        ),
        ..Default::default()
    });
}

fn register_derived_symbol(checker: &mut TypeChecker) {
    let param_type = checker.intern(Type::Param(TypeParam {
        name: "Type".into(),
        idx: 0,
    }));
    let derived_def_type = checker.intern_unique(Type::Function(FunctionType {
        params: vec![param_type],
        return_type: param_type,
    }));
    let derived_type = checker.intern(Type::Generic(GenericType {
        params: vec![param_type],
        definition: derived_def_type,
    }));
    checker.ctx.register_symbol(SymbolData {
        name: "derived$".to_string(),
        ty: derived_type,
        kind: SymbolKind::Function {
            param_names: vec!["expression".to_string()],
        },
        docs: Some(
            r#"Creates a derived reactive variable from the given expression.

Dependencies are tracked and handled at the compiler level.

Just like states, the underlying value can be accessed using the dereference operator `*`.

# Example
```tine
// Here `counter` will automatically be tracked as a dependency.
const derivedCounter = derived$(*counter + 1)
const derivedValue = *derivedCounter
*derivedValue = 0 // This is not allowed and will result in a compiler error.
```
"#
            .to_string(),
        ),
        ..Default::default()
    });
}
