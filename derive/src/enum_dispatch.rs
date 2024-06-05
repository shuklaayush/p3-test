use quote::quote;
use syn::Fields;

pub fn generate_trait_impls(
    name: &syn::Ident,
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
        impl core::fmt::Display for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                match self {
                    #(#name::#variant_names(_) => write!(f, stringify!(#variant_names)),)*
                }
            }
        }

        impl<F: p3_field::Field> p3_air::BaseAir<F> for #name {
            fn width(&self) -> usize {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as p3_air::BaseAir<F>>::width(chip),)*
                }
            }

            fn preprocessed_trace(&self) -> Option<p3_matrix::dense::RowMajorMatrix<F>> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as p3_air::BaseAir<F>>::preprocessed_trace(chip),)*
                }
            }
        }

        impl<AB: p3_air::AirBuilder> p3_air::Air<AB> for #name {
            fn eval(&self, builder: &mut AB) {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as p3_air::Air<AB>>::eval(chip, builder),)*
                }
            }
        }

        impl<F: p3_field::Field> p3_interaction::BaseInteractionAir<F> for #name {
            fn receives_from_indices(&self, preprocessed_indices: &[usize], main_indices: &[usize]) -> alloc::vec::Vec<p3_interaction::Interaction<F>> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as p3_interaction::BaseInteractionAir<F>>::receives_from_indices(chip, preprocessed_indices, main_indices),)*
                }
            }

            fn sends_from_indices(&self, preprocessed_indices: &[usize], main_indices: &[usize]) -> alloc::vec::Vec<p3_interaction::Interaction<F>> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as p3_interaction::BaseInteractionAir<F>>::sends_from_indices(chip, preprocessed_indices, main_indices),)*
                }
            }
        }

        impl<F: p3_field::Field> p3_interaction::InteractionAir<F> for #name {
            fn receives(&self) -> alloc::vec::Vec<p3_interaction::Interaction<F>> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as p3_interaction::InteractionAir<F>>::receives(chip),)*
                }
            }

            fn sends(&self) -> alloc::vec::Vec<p3_interaction::Interaction<F>> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as p3_interaction::InteractionAir<F>>::sends(chip),)*
                }
            }
        }

        impl<AB: p3_interaction::InteractionAirBuilder> p3_interaction::Rap<AB> for #name {
            fn preprocessed_width(&self) -> usize {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as p3_interaction::Rap<AB>>::preprocessed_width(chip),)*
                }
            }
        }

        #[cfg(feature = "air-logger")]
        impl p3_air_util::AirLogger for #name {
            fn preprocessed_headers(&self) -> alloc::vec::Vec<String> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as p3_air_util::AirLogger>::preprocessed_headers(chip),)*
                }
            }

            fn main_headers(&self) -> alloc::vec::Vec<String> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as p3_air_util::AirLogger>::main_headers(chip),)*
                }
            }

            #[cfg(feature = "schema")]
            fn preprocessed_headers_and_types(&self) -> alloc::vec::Vec<(String, String, core::ops::Range<usize>)> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as p3_air_util::AirLogger>::preprocessed_headers_and_types(chip),)*
                }
            }

            #[cfg(feature = "schema")]
            fn main_headers_and_types(&self) -> alloc::vec::Vec<(String, String, core::ops::Range<usize>)> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as p3_air_util::AirLogger>::main_headers_and_types(chip),)*
                }
            }
        }

        impl p3_machine::chip::Chip for #name {}
    }
}
