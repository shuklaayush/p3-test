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

#[proc_macro_derive(AlignedBorrow)]
pub fn aligned_borrow_derive(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    // Get struct name from ast
    let name = &ast.ident;
    let methods = quote! {
        impl<T> core::borrow::Borrow<#name<T>> for [T] {
            fn borrow(&self) -> &#name<T> {
                debug_assert_eq!(self.len(), size_of::<#name<u8>>());
                let (prefix, shorts, _suffix) = unsafe { self.align_to::<#name<T>>() };
                debug_assert!(prefix.is_empty(), "Alignment should match");
                debug_assert_eq!(shorts.len(), 1);
                &shorts[0]
            }
        }

        impl<T> core::borrow::BorrowMut<#name<T>> for [T] {
            fn borrow_mut(&mut self) -> &mut #name<T> {
                debug_assert_eq!(self.len(), size_of::<#name<u8>>());
                let (prefix, shorts, _suffix) = unsafe { self.align_to_mut::<#name<T>>() };
                debug_assert!(prefix.is_empty(), "Alignment should match");
                debug_assert_eq!(shorts.len(), 1);
                &mut shorts[0]
            }
        }
    };
    methods.into()
}

#[proc_macro_derive(EnumDispatch)]
pub fn enum_dispatch_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;

    let variants = match input.data {
        Data::Enum(data_enum) => data_enum.variants,
        _ => panic!("EnumDispatch can only be derived for enums"),
    };

    let trait_impls = generate_trait_impls(&enum_name, &variants);

    TokenStream::from(quote! {
        #trait_impls
    })
}

fn generate_trait_impls(
    enum_name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
) -> proc_macro2::TokenStream {
    let variant_names: Vec<_> = variants.iter().map(|variant| &variant.ident).collect();
    let variant_field_types: Vec<_> = variants
        .iter()
        .map(|variant| match &variant.fields {
            Fields::Unnamed(fields) => &fields.unnamed.first().unwrap().ty,
            _ => panic!("EnumDispatch only supports enum variants with a single unnamed field"),
        })
        .collect();

    quote! {
        use p3_air::{Air, AirBuilder, BaseAir};
        use p3_field::{AbstractField, ExtensionField, Field, PrimeField32};
        use p3_interaction::{Interaction, InteractionAir, InteractionAirBuilder, InteractionChip};
        use p3_matrix::dense::RowMajorMatrix;
        use p3_stark::AirDebug;
        use p3_uni_stark::{StarkGenericConfig, Val};

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    #(#enum_name::#variant_names(_) => write!(f, stringify!(#variant_names)),)*
                }
            }
        }

        impl<F: Field> BaseAir<F> for #enum_name {
            fn width(&self) -> usize {
                match self {
                    #(#enum_name::#variant_names(chip) => <#variant_field_types as BaseAir<F>>::width(chip),)*
                }
            }

            fn preprocessed_trace(&self) -> Option<RowMajorMatrix<F>> {
                match self {
                    #(#enum_name::#variant_names(chip) => <#variant_field_types as BaseAir<F>>::preprocessed_trace(chip),)*
                }
            }
        }

        impl<AB: AirBuilder> Air<AB> for #enum_name {
            fn eval(&self, builder: &mut AB) {
                match self {
                    #(#enum_name::#variant_names(chip) => <#variant_field_types as Air<AB>>::eval(chip, builder),)*
                }
            }
        }

        impl<F: AbstractField> InteractionChip<F> for #enum_name {
            fn sends(&self) -> Vec<Interaction<F>> {
                match self {
                    #(#enum_name::#variant_names(chip) => <#variant_field_types as InteractionChip<F>>::sends(chip),)*
                }
            }

            fn receives(&self) -> Vec<Interaction<F>> {
                match self {
                    #(#enum_name::#variant_names(chip) => <#variant_field_types as InteractionChip<F>>::receives(chip),)*
                }
            }
        }

        impl<AB: InteractionAirBuilder> InteractionAir<AB> for #enum_name {
            fn preprocessed_width(&self) -> usize {
                match self {
                    #(#enum_name::#variant_names(chip) => <#variant_field_types as InteractionAir<AB>>::preprocessed_width(chip),)*
                }
            }
        }

        impl<F: PrimeField32, EF: ExtensionField<F>> AirDebug<F, EF> for #enum_name {
            fn main_headers(&self) -> Vec<String> {
                match self {
                    #(#enum_name::#variant_names(chip) => <#variant_field_types as AirDebug<F, EF>>::main_headers(chip),)*
                }
            }
        }

        impl<SC: StarkGenericConfig> MachineChip<SC> for #enum_name where Val<SC>: PrimeField32 {}
    }
}
