use proc_macro::TokenStream;

mod tree_struct;

#[proc_macro_attribute]
pub fn tree_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    tree_struct::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
