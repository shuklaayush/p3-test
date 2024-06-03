extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
#[cfg(feature = "trace-writer")]
use quote::format_ident;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, GenericParam};
#[cfg(feature = "trace-writer")]
use syn::{Ident, Type};

// TODO: Check if serde_json is easier
#[cfg(feature = "trace-writer")]
fn generate_header_expr(
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

#[proc_macro_derive(Columns)]
pub fn columns_derive(input: TokenStream) -> TokenStream {
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
    let non_first_generics = input
        .generics
        .params
        .iter()
        .skip(1)
        .filter_map(|param| match param {
            GenericParam::Type(type_param) => Some(&type_param.ident),
            GenericParam::Const(const_param) => Some(&const_param.ident),
            _ => None,
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

    let stream = quote! {
        impl #impl_generics #name #type_generics #where_clause {
            pub const fn num_cols() -> usize {
                core::mem::size_of::<#name<u8 #(, #non_first_generics)*>>()
            }

            pub fn col_map() -> #name<usize #(, #non_first_generics)*> {
                let num_cols = Self::num_cols();
                let indices_arr = (0..num_cols).collect::<Vec<usize>>();

                let mut cols = std::mem::MaybeUninit::<#name<usize #(, #non_first_generics)*>>::uninit();
                let ptr = cols.as_mut_ptr() as *mut usize;
                unsafe {
                    ptr.copy_from_nonoverlapping(indices_arr.as_ptr(), num_cols);
                    cols.assume_init()
                }
            }

            #header_impl
        }

        impl<#(#non_first_generics),*> #name<usize #(, #non_first_generics)*> #where_clause {
            pub fn range(&self) -> std::ops::Range<usize> {
                let num_cols = Self::num_cols();
                let ptr = self as *const _ as *const usize;
                let start = unsafe { *ptr };
                start..start + num_cols
            }
        }

        impl #impl_generics core::borrow::Borrow<#name #type_generics> for [#type_generic] #where_clause {
            fn borrow(&self) -> &#name #type_generics {
                debug_assert_eq!(self.len(), std::mem::size_of::<#name<u8 #(, #non_first_generics)*>>());
                let (prefix, shorts, _suffix) = unsafe { self.align_to::<#name #type_generics>() };
                debug_assert!(prefix.is_empty(), "Alignment should match");
                debug_assert_eq!(shorts.len(), 1);
                &shorts[0]
            }
        }

        impl #impl_generics core::borrow::BorrowMut<#name #type_generics> for [#type_generic] #where_clause {
            fn borrow_mut(&mut self) -> &mut #name #type_generics {
                debug_assert_eq!(self.len(), std::mem::size_of::<#name<u8 #(, #non_first_generics)*>>());
                let (prefix, shorts, _suffix) = unsafe { self.align_to_mut::<#name #type_generics>() };
                debug_assert!(prefix.is_empty(), "Alignment should match");
                debug_assert_eq!(shorts.len(), 1);
                &mut shorts[0]
            }
        }
    };

    TokenStream::from(stream)
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
        #[cfg(feature = "trace-writer")]
        use p3_air_util::TraceWriter;
        use p3_field::{ExtensionField, Field, PrimeField32};
        use p3_interaction::{Interaction, InteractionAir, InteractionAirBuilder, Rap};
        use p3_machine::chip::MachineChip;
        use p3_matrix::dense::RowMajorMatrix;
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

        impl<F: Field> InteractionAir<F> for #enum_name {
            fn receives(&self) -> Vec<Interaction<F>> {
                match self {
                    #(#enum_name::#variant_names(chip) => <#variant_field_types as InteractionAir<F>>::receives(chip),)*
                }
            }

            fn sends(&self) -> Vec<Interaction<F>> {
                match self {
                    #(#enum_name::#variant_names(chip) => <#variant_field_types as InteractionAir<F>>::sends(chip),)*
                }
            }
        }

        impl<AB: InteractionAirBuilder> Rap<AB> for #enum_name {
            fn preprocessed_width(&self) -> usize {
                match self {
                    #(#enum_name::#variant_names(chip) => <#variant_field_types as Rap<AB>>::preprocessed_width(chip),)*
                }
            }
        }

        #[cfg(feature = "trace-writer")]
        impl<F: PrimeField32, EF: ExtensionField<F>> TraceWriter<F, EF> for #enum_name {
            fn preprocessed_headers(&self) -> Vec<String> {
                match self {
                    #(#enum_name::#variant_names(chip) => <#variant_field_types as TraceWriter<F, EF>>::preprocessed_headers(chip),)*
                }
            }

            fn headers(&self) -> Vec<String> {
                match self {
                    #(#enum_name::#variant_names(chip) => <#variant_field_types as TraceWriter<F, EF>>::headers(chip),)*
                }
            }
        }

        impl<SC: StarkGenericConfig> MachineChip<SC> for #enum_name where Val<SC>: PrimeField32 {}
    }
}
