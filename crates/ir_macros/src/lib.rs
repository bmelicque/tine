use proc_macro::TokenStream;

mod ir_struct;

// tine_ir_macros/src/lib.rs
#[proc_macro_attribute]
pub fn ir_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    ir_struct::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
