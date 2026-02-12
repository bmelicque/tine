use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(EnumFrom)]
pub fn derive_enum_from(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let enum_name = input.ident;

    let Data::Enum(data_enum) = input.data else {
        return syn::Error::new_spanned(enum_name, "EnumFrom can only be derived for enums")
            .to_compile_error()
            .into();
    };

    let mut impls = Vec::new();

    for variant in data_enum.variants {
        let variant_name = variant.ident;

        let field_ty = match variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                &fields.unnamed.first().unwrap().ty.clone()
            }
            _ => {
                return syn::Error::new_spanned(
                    variant_name,
                    "EnumFrom only supports variants like Variant(T)",
                )
                .to_compile_error()
                .into();
            }
        };

        impls.push(quote! {
            impl From<#field_ty> for #enum_name {
                fn from(value: #field_ty) -> Self {
                    #enum_name::#variant_name(value)
                }
            }
        });
    }

    TokenStream::from(quote! {
        #(#impls)*
    })
}
