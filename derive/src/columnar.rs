#[cfg(feature = "trace-writer")]
use quote::format_ident;
use quote::quote;
#[cfg(feature = "trace-writer")]
use syn::{Ident, Type};

// TODO: Check if serde_json is easier
#[cfg(feature = "trace-writer")]
pub fn generate_header_expr(
    base_generic: &Ident,
    field_type: &Type,
    prefix: &str,
    depth: u32,
) -> proc_macro2::TokenStream {
    match field_type {
        Type::Array(array) => {
            let elem_type = &array.elem;
            let len_expr = &array.len;

            let inner_expr = generate_header_expr(base_generic, elem_type, prefix, depth + 1);
            let idepth = format_ident!("i{}", depth);
            quote! {
                for #idepth in 0..#len_expr {
                    #inner_expr
                }
            }
        }
        Type::Path(type_path) => {
            let last_segment = type_path.path.segments.last().unwrap();
            if last_segment.ident == *base_generic {
                let expr = (0..depth).fold(prefix.to_string(), |acc, _| format!("{}[{{}}]", acc));
                let is = (0..depth).map(|i| format_ident!("i{}", i));
                quote! {
                    headers.push(format!(#expr, #(#is),*));
                }
            } else {
                // Assuming it's a struct with a headers() method
                let name = &last_segment.ident;
                let generic_args = &last_segment.arguments;
                quote! {
                    for header in #name::#generic_args::headers() {
                        headers.push(format!("{}.{}", #prefix, header));
                    }
                }
            }
        }
        _ => unreachable!(),
    }
}
