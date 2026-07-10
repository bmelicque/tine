use std::collections::HashMap;

use tine_common::module_path::ModuleId;
use tine_symbols::symbols::*;
use tine_types::types;

use crate::TypeChecker;

pub fn check_signals_module(module: ModuleId, tc: &mut TypeChecker) {
    let mut exports = HashMap::new();

    let state_symbol = register_state_symbol(tc);
    exports.insert("state".to_string(), state_symbol.into());

    let computed_symbol = register_computed_symbol(tc);
    exports.insert("computed$".to_string(), computed_symbol.into());

    tc.add_exports(module, exports);
}

fn register_state_symbol(tc: &mut TypeChecker) -> FunctionSymbolId {
    let param_type = tc.add_type_param("Type".to_string());
    let return_type = tc.intern(types::SignalType {
        inner: param_type.id,
    });
    let state_type = tc.intern_unique(types::FunctionType {
        params: vec![param_type.id],
        type_params: vec![param_type],
        return_type,
    });
    tc.symbols.insert::<FunctionSymbolId>(FunctionSymbol {
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
    })
}

fn register_computed_symbol(tc: &mut TypeChecker) -> FunctionSymbolId {
    let param_type = tc.add_type_param("Type".to_string());
    let derived_type = tc.intern_unique(types::FunctionType {
        params: vec![param_type.id],
        return_type: param_type.id,
        type_params: vec![param_type],
    });
    tc.symbols.insert::<FunctionSymbolId>(FunctionSymbol {
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
    })
}
