use quote::format_ident;
use quote::quote;
use syn::Expr;
use syn::{Data, Fields, Ident, Type};

pub fn get_type_array_lengths<'a>(
    type_generic: &Ident,
    field_type: &'a Type,
    mut lengths: Vec<&'a Expr>,
) -> Option<Vec<&'a Expr>> {
    match field_type {
        Type::Array(array) => {
            let elem_type = &array.elem;
            let len_expr = &array.len;

            lengths.push(len_expr);
            get_type_array_lengths(type_generic, elem_type, lengths)
        }
        Type::Path(type_path) => {
            let last_segment = type_path.path.segments.last().unwrap();
            if last_segment.ident == *type_generic {
                Some(lengths)
            } else {
                None
            }
        }
        _ => unreachable!(),
    }
}

// TODO: Check if serde_json is easier
pub fn generate_header_expr(
    type_generic: &Ident,
    field_type: &Type,
    prefix: &str,
    depth: u32,
) -> proc_macro2::TokenStream {
    match field_type {
        Type::Array(array) => {
            let elem_type = &array.elem;
            let len_expr = &array.len;

            let inner_expr = generate_header_expr(type_generic, elem_type, prefix, depth + 1);
            let idepth = format_ident!("i{}", depth);
            quote! {
                for #idepth in 0..#len_expr {
                    #inner_expr
                }
            }
        }
        Type::Path(type_path) => {
            let last_segment = type_path.path.segments.last().unwrap();
            if last_segment.ident == *type_generic {
                let expr = (0..depth).fold(prefix.to_string(), |acc, _| format!("{}[{{}}]", acc));
                let is = (0..depth).map(|i| format_ident!("i{}", i));
                quote! {
                    out.push(format!(#expr, #(#is),*));
                }
            } else {
                // Assuming it's a struct with a headers() method
                let name = &last_segment.ident;
                let generic_args = &last_segment.arguments;
                quote! {
                    for header in #name::#generic_args::headers() {
                        out.push(format!("{}.{}", #prefix, header));
                    }
                }
            }
        }
        _ => unreachable!(),
    }
}

pub fn generate_primitive_header_expr(
    type_generic: &Ident,
    field_type: &Type,
    prefix: &str,
    depth: u32,
) -> proc_macro2::TokenStream {
    let maybe_lengths = get_type_array_lengths(type_generic, field_type, vec![]);

    match maybe_lengths {
        Some(lengths) => {
            if lengths.is_empty() {
                quote! {
                    out.push((#prefix.to_string(), "Field".to_string(), offset..offset+1));
                    offset += 1;
                }
            } else {
                let ty = lengths
                    .iter()
                    .fold("Field".to_string(), |acc, _| format!("{}[{{}}]", acc));
                quote! {
                    let total_len = [#(#lengths,)*].iter().fold(1, |acc, len| acc * len);
                    out.push((#prefix.to_string(), format!(#ty, #(#lengths),*), offset..offset+total_len));
                }
            }
        }
        None => {
            match field_type {
                Type::Array(array) => {
                    let elem_type = &array.elem;
                    let len_expr = &array.len;

                    let inner_expr =
                        generate_primitive_header_expr(type_generic, elem_type, prefix, depth + 1);
                    let idepth = format_ident!("i{}", depth);
                    quote! {
                        for #idepth in 0..#len_expr {
                            #inner_expr
                        }
                    }
                }
                Type::Path(type_path) => {
                    let last_segment = type_path.path.segments.last().unwrap();
                    // Assuming it's a struct with a headers_and_types() method
                    let name = &last_segment.ident;
                    let generic_args = &last_segment.arguments;
                    quote! {
                        for (header, ty, range) in #name::#generic_args::headers_and_types() {
                            out.push((format!("{}.{}", #prefix, header), ty, range.start + offset..range.end + offset));
                        }
                        offset += #name::#generic_args::num_cols();
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}

pub fn generate_headers(data: &Data, type_generic: &Ident) -> proc_macro2::TokenStream {
    let fields = match data {
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
        header_exprs.push(generate_header_expr(
            type_generic,
            field_type,
            &field_name.to_string(),
            0,
        ));
    }

    quote! {
        #[cfg(feature = "air-logger")]
        pub fn headers() -> alloc::vec::Vec<String> {
            let mut out = alloc::vec::Vec::new();
            #(#header_exprs)*
            out
        }
    }
}

pub fn generate_headers_and_types(data: &Data, type_generic: &Ident) -> proc_macro2::TokenStream {
    let fields = match data {
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
        header_exprs.push(generate_primitive_header_expr(
            type_generic,
            field_type,
            &field_name.to_string(),
            0,
        ));
    }

    quote! {
        #[cfg(feature = "air-logger")]
        pub fn headers_and_types() -> alloc::vec::Vec<(String, String, core::ops::Range<usize>)> {
            let mut out = alloc::vec::Vec::new();
            let mut offset = 0;
            #(#header_exprs)*
            out
        }
    }
}
