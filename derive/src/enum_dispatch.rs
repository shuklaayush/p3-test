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
        use p3_air::{Air, AirBuilder, BaseAir};
        #[cfg(feature = "trace-writer")]
        use p3_air_util::TraceWriter;
        use p3_field::{ExtensionField, Field};
        use p3_interaction::{Interaction, InteractionAir, InteractionAirBuilder, Rap};
        use p3_machine::chip::MachineChip;
        use p3_matrix::dense::RowMajorMatrix;
        use p3_uni_stark::{StarkGenericConfig, Val};

        impl std::fmt::Display for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    #(#name::#variant_names(_) => write!(f, stringify!(#variant_names)),)*
                }
            }
        }

        impl<F: Field> BaseAir<F> for #name {
            fn width(&self) -> usize {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as BaseAir<F>>::width(chip),)*
                }
            }

            fn preprocessed_trace(&self) -> Option<RowMajorMatrix<F>> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as BaseAir<F>>::preprocessed_trace(chip),)*
                }
            }
        }

        impl<AB: AirBuilder> Air<AB> for #name {
            fn eval(&self, builder: &mut AB) {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as Air<AB>>::eval(chip, builder),)*
                }
            }
        }

        impl<F: Field> InteractionAir<F> for #name {
            fn receives(&self) -> Vec<Interaction<F>> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as InteractionAir<F>>::receives(chip),)*
                }
            }

            fn sends(&self) -> Vec<Interaction<F>> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as InteractionAir<F>>::sends(chip),)*
                }
            }
        }

        impl<AB: InteractionAirBuilder> Rap<AB> for #name {
            fn preprocessed_width(&self) -> usize {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as Rap<AB>>::preprocessed_width(chip),)*
                }
            }
        }

        #[cfg(feature = "trace-writer")]
        impl<F: Field, EF: ExtensionField<F>> TraceWriter<F, EF> for #name {
            fn preprocessed_headers(&self) -> Vec<String> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as TraceWriter<F, EF>>::preprocessed_headers(chip),)*
                }
            }

            fn headers(&self) -> Vec<String> {
                match self {
                    #(#name::#variant_names(chip) => <#variant_field_types as TraceWriter<F, EF>>::headers(chip),)*
                }
            }
        }

        impl<SC: StarkGenericConfig> MachineChip<SC> for #name {}
    }
}
