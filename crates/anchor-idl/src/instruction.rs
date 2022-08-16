use anchor_syn::idl::IdlInstruction;
use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use semver::{Version, VersionReq};

/// Generates a single instruction handler.
pub fn generate_ix_handler(
    ix: &IdlInstruction,
    target_anchor_version: &Version,
) -> TokenStream {
    let ix_name = format_ident!("{}", ix.name.to_snake_case());
    let accounts_name = format_ident!("{}", ix.name.to_pascal_case());

    let args = ix
        .args
        .iter()
        .map(|arg| {
            let name = format_ident!("_{}", arg.name.to_snake_case());
            let type_name = crate::ty_to_rust_type(&arg.ty);
            let stream: proc_macro2::TokenStream = type_name.parse().unwrap();
            quote! {
                #name: #stream
            }
        })
        .collect::<Vec<_>>();

    if VersionReq::parse(">=0.22.0").unwrap().matches(target_anchor_version) {
        quote! {
            pub fn #ix_name(
                _ctx: Context<#accounts_name>,
                #(#args),*
            ) -> Result<()> {
                unimplemented!("This program is a wrapper for CPI.")
            }
        }
    }
    else {
        quote! {
            pub fn #ix_name(
                _ctx: Context<#accounts_name>,
                #(#args),*
            ) -> ProgramResult {
                unimplemented!("This program is a wrapper for CPI.")
            }
        }
    }
}

/// Generates instruction context structs.
pub fn generate_ix_structs(
    ixs: &[IdlInstruction],
    _target_anchor_version: &Version,
) -> TokenStream {
    let defs = ixs.iter().map(|ix| {
        let accounts_name = format_ident!("{}", ix.name.to_pascal_case());

        let (all_structs, all_fields) =
            crate::generate_account_fields(&ix.name.to_pascal_case(), &ix.accounts);

        quote! {
            #all_structs

            #[derive(Accounts)]
            pub struct #accounts_name<'info> {
                #all_fields
            }
        }
    });
    quote! {
        #(#defs)*
    }
}

/// Generates all instruction handlers.
pub fn generate_ix_handlers(
    ixs: &[IdlInstruction],
    target_anchor_version: &Version,
) -> TokenStream {
    let streams = ixs.iter().map(|ix|
        generate_ix_handler(ix, target_anchor_version)
    );
    quote! {
        #(#streams)*
    }
}
