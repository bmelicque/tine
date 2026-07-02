use crate::{
    analysis_context::symbols::{FunctionSymbol, FunctionSymbolId},
    type_checker::{CheckResult, TypeChecker},
    types::{FunctionType, SignalType, Type},
    ModuleId, Session,
};

impl Session {
    pub fn check_signals_module(&mut self, id: ModuleId) -> CheckResult {
        let mut checker = TypeChecker::new(self, id);

        register_state_symbol(&mut checker);
        register_derived_symbol(&mut checker);

        let main_scope = &checker.ctx.scopes[0];

        CheckResult {
            exports: main_scope.bindings.clone(),
            ..Default::default()
        }
    }
}

fn register_state_symbol(checker: &mut TypeChecker) {
    let param_type = checker.add_type_param("Type".to_string());
    let return_type = checker.intern(Type::Signal(SignalType {
        inner: param_type.id,
    }));
    let state_type = checker.intern_unique(Type::Function(FunctionType {
        params: vec![param_type.id],
        type_params: vec![param_type],
        return_type,
    }));
    checker.symbols.insert::<FunctionSymbolId>(FunctionSymbol {
        name: "state".to_string(),
        ty: state_type,
        param_names: vec!["initialValue".to_string()],
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
    let param_type = checker.add_type_param("Type".to_string());
    let derived_type = checker.intern_unique(Type::Function(FunctionType {
        params: vec![param_type.id],
        return_type: param_type.id,
        type_params: vec![param_type],
    }));
    checker.symbols.insert::<FunctionSymbolId>(FunctionSymbol {
        name: "computed$".to_string(),
        ty: derived_type,
        param_names: vec!["expression".to_string()],
        docs: Some(
            r#"Creates a derived reactive variable from the given expression.

Dependencies are tracked and handled at the compiler level.

Just like states, the underlying value can be accessed using the dereference operator `*`.

# Example
```tine
// Here `counter` will automatically be tracked as a dependency.
let nextCounter = computed$(*counter + 1)
let nextValue = *nextCounter
*nextValue = 0 // This is not allowed and will result in a compiler error.
```
"#
            .to_string(),
        ),
        ..Default::default()
    });
}
