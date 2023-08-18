use std::collections::BTreeMap;

use anchor_syn::idl::{IdlField, IdlTypeDefinition};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{generate_fields, get_field_list_properties, StructOpts};

/// Generates an account state struct.
pub fn generate_account(
    defs: &[IdlTypeDefinition],
    account_name: &str,
    fields: &[IdlField],
    opts: StructOpts,
) -> TokenStream {
    let props = get_field_list_properties(defs, fields);

    let derive_copy = if props.can_copy && !opts.zero_copy.is_some() {
        quote! {
            #[derive(Copy)]
        }
    } else {
        quote! {}
    };
    let derive_default = if props.can_derive_default {
        quote! {
            #[derive(Default)]
        }
    } else {
        quote! {}
    };
    let derive_account = if let Some(zero_copy) = opts.zero_copy {
        let zero_copy_quote = match zero_copy {
            crate::ZeroCopy::Unsafe => quote! {
                #[account(zero_copy(unsafe))]
            },
            crate::ZeroCopy::Safe => quote! {
                #[account(zero_copy)]
            },
        };
        if let Some(repr) = opts.representation {
            let repr_quote = match repr {
                crate::Representation::C => quote! {
                    #[repr(C)]
                },
                crate::Representation::Transparent => quote! {
                    #[repr(transparent)]
                },
                crate::Representation::Packed => quote! {
                    #[repr(packed)]
                },
            };
            quote! {
                #zero_copy_quote
                #repr_quote
            }
        } else {
            zero_copy_quote
        }
    } else {
        quote! {#[account]}
    };

    let doc = format!(" Account: {}", account_name);
    let struct_name = format_ident!("{}", account_name);
    let fields_rendered = generate_fields(fields);
    quote! {
        #derive_account
        #[doc = #doc]
        #derive_copy
        #derive_default
        pub struct #struct_name {
            #fields_rendered
        }
    }
}

/// Generates account state structs.
pub fn generate_accounts(
    typedefs: &[IdlTypeDefinition],
    account_defs: &[IdlTypeDefinition],
    struct_opts: &BTreeMap<String, StructOpts>,
) -> TokenStream {
    let defined = account_defs.iter().map(|def| match &def.ty {
        anchor_syn::idl::IdlTypeDefinitionTy::Struct { fields } => {
            let opts = struct_opts.get(&def.name).copied().unwrap_or_default();
            generate_account(typedefs, &def.name, fields, opts)
        }
        anchor_syn::idl::IdlTypeDefinitionTy::Enum { .. } => {
            panic!("unexpected enum account");
        }
    });
    quote! {
        #(#defined)*
    }
}
