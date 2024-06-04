extern crate proc_macro;
extern crate quote;
extern crate syn;

#[cfg(feature = "trace-writer")]
mod columnar;
mod enum_dispatch;

use proc_macro::TokenStream;
use quote::quote;
#[cfg(feature = "trace-writer")]
use syn::Fields;
use syn::{parse_macro_input, Data, DeriveInput, GenericParam};

#[cfg(feature = "trace-writer")]
use self::columnar::generate_header_expr;
use self::enum_dispatch::generate_trait_impls;

#[proc_macro_derive(Bus)]
pub fn bus_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let variants = match input.data {
        Data::Enum(data_enum) => data_enum.variants,
        _ => panic!("Bus can only be derived for enums"),
    };
    let variant_names: Vec<_> = variants.iter().map(|variant| &variant.ident).collect();
    let variant_discriminants: Vec<_> = variants
        .iter()
        .map(|variant| {
            if let Some((_, expr)) = &variant.discriminant {
                expr
            } else {
                panic!("All enum variants must have an explicit discriminant");
            }
        })
        .collect();

    let expanded = quote! {
        impl p3_interaction::Bus for #name {
            fn from_usize(value: usize) -> Option<Self> {
                match value {
                    #(#variant_discriminants => Some(Self::#variant_names),)*
                    _ => None,
                }
            }

            fn name(&self) -> &'static str {
                match self {
                    #(#name::#variant_names => stringify!(#variant_names),)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(EnumDispatch)]
pub fn enum_dispatch_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let variants = match input.data {
        Data::Enum(data_enum) => data_enum.variants,
        _ => panic!("EnumDispatch can only be derived for enums"),
    };

    let trait_impls = generate_trait_impls(&name, &variants);

    TokenStream::from(trait_impls)
}

#[proc_macro_derive(Columnar)]
pub fn columnar_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let type_generic = input
        .generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Type(type_param) => &type_param.ident,
            _ => panic!("Expected first generic to be a type"),
        })
        .next()
        .expect("Expected at least one generic");
    let non_first_generics = input.generics.params.iter().skip(1).collect::<Vec<_>>();
    let non_first_generics_idents = non_first_generics
        .iter()
        .map(|param| match param {
            GenericParam::Type(type_param) => &type_param.ident,
            GenericParam::Const(const_param) => &const_param.ident,
            _ => panic!("Expected type or const generic"),
        })
        .collect::<Vec<_>>();
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    #[cfg(feature = "trace-writer")]
    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => fields.named.iter(),
            _ => panic!("Unsupported struct fields"),
        },
        _ => panic!("Unsupported data type"),
    };
    #[cfg(feature = "trace-writer")]
    let mut header_exprs = Vec::new();
    #[cfg(feature = "trace-writer")]
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

    #[cfg(feature = "trace-writer")]
    let header_impl = quote! {
        #[cfg(feature = "trace-writer")]
        pub fn headers() -> Vec<String> {
            let mut headers = Vec::new();
            #(#header_exprs)*
            headers
        }
    };
    #[cfg(not(feature = "trace-writer"))]
    let header_impl = quote! {};

    let expanded = quote! {
        impl #impl_generics #name #type_generics #where_clause {
            pub const fn num_cols() -> usize {
                core::mem::size_of::<#name<u8 #(, #non_first_generics_idents)*>>()
            }

            pub fn col_map() -> #name<usize #(, #non_first_generics_idents)*> {
                let num_cols = Self::num_cols();
                let indices_arr = (0..num_cols).collect::<Vec<usize>>();

                let mut cols = std::mem::MaybeUninit::<#name<usize #(, #non_first_generics_idents)*>>::uninit();
                let ptr = cols.as_mut_ptr() as *mut usize;
                unsafe {
                    ptr.copy_from_nonoverlapping(indices_arr.as_ptr(), num_cols);
                    cols.assume_init()
                }
            }

            #header_impl
        }

        impl<#(#non_first_generics),*> #name<usize #(, #non_first_generics_idents)*> #where_clause {
            pub fn as_vec(&self) -> Vec<usize> {
                let num_cols = Self::num_cols();
                let ptr = self as *const _ as *const usize;
                unsafe {
                    std::slice::from_raw_parts(ptr, num_cols).to_vec()
                }
            }

            pub fn from_vec(indices: Vec<usize>) -> Self {
                let num_cols = Self::num_cols();
                assert_eq!(indices.len(), num_cols, "Expected {} indices, got {}", num_cols, indices.len());
                let mut cols = std::mem::MaybeUninit::<#name<usize #(, #non_first_generics_idents)*>>::uninit();
                let ptr = cols.as_mut_ptr() as *mut usize;
                unsafe {
                    ptr.copy_from_nonoverlapping(indices.as_ptr(), num_cols);
                    cols.assume_init()
                }
            }

            pub fn as_range(&self) -> std::ops::Range<usize> {
                let num_cols = Self::num_cols();
                // TODO: Check if actually a range
                let ptr = self as *const _ as *const usize;
                let start = unsafe { *ptr };
                start..start + num_cols
            }
        }

        impl #impl_generics core::borrow::Borrow<#name #type_generics> for [#type_generic] #where_clause {
            fn borrow(&self) -> &#name #type_generics {
                debug_assert_eq!(self.len(), std::mem::size_of::<#name<u8 #(, #non_first_generics_idents)*>>());
                let (prefix, shorts, _suffix) = unsafe { self.align_to::<#name #type_generics>() };
                debug_assert!(prefix.is_empty(), "Alignment should match");
                debug_assert_eq!(shorts.len(), 1);
                &shorts[0]
            }
        }

        impl #impl_generics core::borrow::BorrowMut<#name #type_generics> for [#type_generic] #where_clause {
            fn borrow_mut(&mut self) -> &mut #name #type_generics {
                debug_assert_eq!(self.len(), std::mem::size_of::<#name<u8 #(, #non_first_generics_idents)*>>());
                let (prefix, shorts, _suffix) = unsafe { self.align_to_mut::<#name #type_generics>() };
                debug_assert!(prefix.is_empty(), "Alignment should match");
                debug_assert_eq!(shorts.len(), 1);
                &mut shorts[0]
            }
        }
    };

    TokenStream::from(expanded)
}
