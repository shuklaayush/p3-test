extern crate proc_macro;
extern crate quote;
extern crate syn;

#[cfg(feature = "air-logger")]
mod columnar;
mod enum_dispatch;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, GenericParam};

#[cfg(feature = "air-logger")]
use self::columnar::generate_headers;
#[cfg(feature = "air-logger")]
use self::columnar::generate_headers_and_types;
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
        impl core::fmt::Display for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                match self {
                    #(#name::#variant_names => write!(f, stringify!(#variant_names)),)*
                }
            }
        }

        impl From<usize> for #name {
            fn from(value: usize) -> Self {
                match value {
                    #(#variant_discriminants => Self::#variant_names,)*
                    _ => panic!("Invalid value for enum #name"),
                }
            }
        }

        impl p3_interaction::Bus for #name {}
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

    #[cfg(feature = "air-logger")]
    let header_impl = generate_headers(&input.data, type_generic);
    #[cfg(not(feature = "air-logger"))]
    let header_impl = quote! {};

    #[cfg(feature = "air-logger")]
    let header_type_impl = generate_headers_and_types(&input.data, type_generic);
    #[cfg(not(feature = "air-logger"))]
    let header_type_impl = quote! {};

    let expanded = quote! {
        impl #impl_generics #name #type_generics #where_clause {
            pub const fn num_cols() -> usize {
                core::mem::size_of::<#name<u8 #(, #non_first_generics_idents)*>>()
            }

            pub fn col_map() -> #name<usize #(, #non_first_generics_idents)*> {
                let num_cols = Self::num_cols();
                let indices_arr = (0..num_cols).collect::<alloc::vec::Vec<usize>>();

                let mut cols = core::mem::MaybeUninit::<#name<usize #(, #non_first_generics_idents)*>>::uninit();
                let ptr = cols.as_mut_ptr() as *mut usize;
                unsafe {
                    ptr.copy_from_nonoverlapping(indices_arr.as_ptr(), num_cols);
                    cols.assume_init()
                }
            }

            #header_impl

            #header_type_impl
        }

        impl<#(#non_first_generics),*> #name<usize #(, #non_first_generics_idents)*> #where_clause {
            pub fn from_slice(indices: &[usize]) -> Self {
                let num_cols = Self::num_cols();
                debug_assert_eq!(indices.len(), num_cols, "Expected {} indices, got {}", num_cols, indices.len());
                let mut cols = core::mem::MaybeUninit::<#name<usize #(, #non_first_generics_idents)*>>::uninit();
                let ptr = cols.as_mut_ptr() as *mut usize;
                unsafe {
                    ptr.copy_from_nonoverlapping(indices.as_ptr(), num_cols);
                    cols.assume_init()
                }
            }

            pub fn as_slice(&self) -> &[usize] {
                let num_cols = Self::num_cols();
                let ptr = self as *const _ as *const usize;
                unsafe {
                    core::slice::from_raw_parts(ptr, num_cols)
                }
            }

            pub fn as_range(&self) -> core::ops::Range<usize> {
                debug_assert!(self.as_slice().windows(2).all(|w| w[1] == w[0] + 1), "Expected contiguous indices");
                let ptr = self as *const _ as *const usize;
                let start = unsafe { *ptr };
                start..start + Self::num_cols()
            }
        }

        impl #impl_generics core::borrow::Borrow<#name #type_generics> for [#type_generic] #where_clause {
            fn borrow(&self) -> &#name #type_generics {
                debug_assert_eq!(self.len(), core::mem::size_of::<#name<u8 #(, #non_first_generics_idents)*>>());
                let (prefix, shorts, _suffix) = unsafe { self.align_to::<#name #type_generics>() };
                debug_assert!(prefix.is_empty(), "Alignment should match");
                debug_assert_eq!(shorts.len(), 1);
                &shorts[0]
            }
        }

        impl #impl_generics core::borrow::BorrowMut<#name #type_generics> for [#type_generic] #where_clause {
            fn borrow_mut(&mut self) -> &mut #name #type_generics {
                debug_assert_eq!(self.len(), core::mem::size_of::<#name<u8 #(, #non_first_generics_idents)*>>());
                let (prefix, shorts, _suffix) = unsafe { self.align_to_mut::<#name #type_generics>() };
                debug_assert!(prefix.is_empty(), "Alignment should match");
                debug_assert_eq!(shorts.len(), 1);
                &mut shorts[0]
            }
        }
    };

    TokenStream::from(expanded)
}
