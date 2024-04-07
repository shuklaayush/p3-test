extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

fn generate_header_expr(field_type: &Type, prefix: &str, depth: u32) -> proc_macro2::TokenStream {
    match field_type {
        Type::Array(array) => {
            let elem_type = &array.elem;
            let len_expr = &array.len;

            let inner_expr = generate_header_expr(elem_type, prefix, depth + 1);
            let idepth = format_ident!("i{}", depth);
            quote! {
                for #idepth in 0..#len_expr {
                    #inner_expr
                }
            }
        }
        _ => {
            let expr = (0..depth).fold(prefix.to_string(), |acc, _| format!("{}[{{}}]", acc));
            let is = (0..depth).map(|i| format_ident!("i{}", i));
            quote! {
                headers.push(format!(#expr, #(#is),*));
            }
        }
    }
}

#[proc_macro_derive(Headers)]
pub fn headers_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;
    let generics = input.generics;

    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => fields.named.iter(),
            _ => panic!("Unsupported struct fields"),
        },
        _ => panic!("Unsupported data type"),
    };

    let mut header_exprs = Vec::new();
    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        header_exprs.push(generate_header_expr(field_type, &field_name.to_string(), 0));
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics  #struct_name #ty_generics #where_clause {
            pub fn headers() -> Vec<String> {
                let mut headers = Vec::new();
                #(#header_exprs)*
                headers
            }
        }
    };

    TokenStream::from(expanded)
}
