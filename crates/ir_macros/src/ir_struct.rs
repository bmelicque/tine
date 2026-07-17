// tine_ir_macros/src/ir_node.rs
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_quote, Data, DeriveInput, Expr, Fields, FieldsNamed, Ident, Result, Token,
};

struct IrNodeArgs {
    ty: Option<Expr>,
    untyped: bool,
}

impl Parse for IrNodeArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut ty = None;
        let mut untyped = false;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "ty" => {
                    input.parse::<Token![=]>()?;
                    ty = Some(input.parse()?);
                }
                "untyped" => untyped = true,
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown ir_node arg `{other}`"),
                    ))
                }
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self { ty, untyped })
    }
}

pub fn expand(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let args: IrNodeArgs = syn::parse2(attr)?;
    let mut input: DeriveInput = syn::parse2(item)?;
    let name = input.ident.clone();

    let fields = match &mut input.data {
        Data::Struct(s) => match &mut s.fields {
            Fields::Named(f) => f,
            other => {
                return Err(syn::Error::new_spanned(
                    other.clone(),
                    "#[ir_node] only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "#[ir_node] only supports structs",
            ))
        }
    };

    inject_field(
        fields,
        parse_quote! { pub loc: ::tine_common::locations::Location },
    );
    if args.ty.is_none() && !args.untyped {
        inject_field(fields, parse_quote! { pub ty: ::tine_types::types::TypeId });
    }

    let typed_impl = match &args.ty {
        Some(expr) => quote! {
            impl Typed for #name {
                fn ty(&self) -> ::tine_types::types::TypeId { #expr }
            }
        },
        None if !args.untyped => quote! {
            impl Typed for #name {
                fn ty(&self) -> ::tine_types::types::TypeId { self.ty }
            }
        },
        None => quote! {}, // untyped nodes (e.g. most statements) get no Typed impl
    };

    let pushes = fields
        .named
        .iter()
        .filter(|field| field.attrs.iter().any(|a| a.path().is_ident("child")))
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            quote! {
                ::tine_ir::PushNodes::push_nodes(&self.#ident, stack);
            }
        })
        .collect::<Vec<_>>();

    for field in fields.named.iter_mut() {
        field.attrs.retain(|a| !a.path().is_ident("child"));
    }

    Ok(quote! {
        #input

        #typed_impl

        impl Locatable for #name {
            fn loc(&self) -> ::tine_common::locations::Location { self.loc }
        }

        impl #name {
            pub fn push_children<'a>(&'a self, stack: &mut Vec<::tine_ir::Node<'a>>) {
                #(#pushes)*
            }
        }
    })
}

fn inject_field(fields: &mut FieldsNamed, field: syn::Field) {
    fields.named.insert(0, field);
}
