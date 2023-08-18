use std::{
    collections::{BTreeMap, HashSet},
    env, fs,
    path::PathBuf,
};

use darling::{util::PathList, FromMeta};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::{
    generate_accounts, generate_ix_handlers, generate_ix_structs, generate_typedefs, GEN_VERSION,
};

#[derive(Default, FromMeta)]
pub struct GeneratorOptions {
    /// Path to the IDL.
    pub idl_path: String,
    /// List of zero copy structs.
    pub zero_copy: Option<PathList>,
    /// List of anchor legacy zero copy structs.
    pub zero_copy_unsafe: Option<PathList>,
    /// List of `repr(C)` structs.
    pub c_representation: Option<PathList>,
    /// List of `repr(transparent)` structs.
    pub transparent_representation: Option<PathList>,
    /// List of `repr(packed)` structs.
    pub packed_representation: Option<PathList>,
}

fn path_list_to_string(list: Option<&PathList>) -> HashSet<String> {
    list.map(|el| {
        el.iter()
            .map(|el| el.get_ident().unwrap().to_string())
            .collect()
    })
    .unwrap_or_default()
}

impl GeneratorOptions {
    pub fn to_generator(&self) -> Generator {
        let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = PathBuf::from(cargo_manifest_dir).join(&self.idl_path);
        let idl_contents = fs::read_to_string(&path).unwrap();
        let idl: anchor_syn::idl::Idl = serde_json::from_str(&idl_contents).unwrap();

        let zero_copy_safe = path_list_to_string(self.zero_copy.as_ref());

        let zero_copy_unsafe = path_list_to_string(self.zero_copy_unsafe.as_ref());

        let c_repr = path_list_to_string(self.c_representation.as_ref());

        let transparent_repr = path_list_to_string(self.transparent_representation.as_ref());

        let packed_repr = path_list_to_string(self.packed_representation.as_ref());

        let repr = c_repr
            .union(&transparent_repr)
            .cloned()
            .collect::<HashSet<_>>()
            .union(&packed_repr)
            .cloned()
            .collect::<HashSet<_>>();

        let zero_copy = zero_copy_safe
            .union(&zero_copy_unsafe)
            .cloned()
            .collect::<HashSet<_>>();

        let mut struct_opts: BTreeMap<String, StructOpts> = BTreeMap::new();
        let all_structs: HashSet<&String> = zero_copy.union(&repr).collect::<HashSet<_>>();
        all_structs.into_iter().for_each(|name| {
            let is_c_repr = c_repr.contains(name);
            let is_transparent_repr = transparent_repr.contains(name);
            let is_packed_repr = packed_repr.contains(name);

            let representation = match (is_c_repr, is_transparent_repr, is_packed_repr) {
                (true, false, false) => Some(Representation::C),
                (false, true, false) => Some(Representation::Transparent),
                (false, false, true) => Some(Representation::Packed),
                (false, false, false) => None,
                _ => panic!("a type cannot have many representation"),
            };

            let is_zero_copy_safe = zero_copy_safe.contains(name);
            let is_zero_copy_unsafe = zero_copy_unsafe.contains(name);

            let zero_copy = match (is_zero_copy_safe, is_zero_copy_unsafe) {
                (true, true) => panic!("cant be safe and unsafe zero copy at the same time"),
                (true, false) => Some(ZeroCopy::Safe),
                (false, true) => Some(ZeroCopy::Unsafe),
                (false, false) => None,
            };

            struct_opts.insert(
                name.to_string(),
                StructOpts {
                    representation,
                    zero_copy,
                },
            );
        });

        Generator { idl, struct_opts }
    }
}

#[derive(Clone, Copy, Default)]
pub struct StructOpts {
    pub representation: Option<Representation>,
    pub zero_copy: Option<ZeroCopy>,
}

#[derive(Clone, Copy)]
pub enum ZeroCopy {
    Unsafe,
    Safe,
}

#[derive(Clone, Copy)]
pub enum Representation {
    C,
    Transparent,
    Packed,
}
pub struct Generator {
    pub idl: anchor_syn::idl::Idl,
    pub struct_opts: BTreeMap<String, StructOpts>,
}

impl Generator {
    pub fn generate_cpi_interface(&self) -> TokenStream {
        let idl = &self.idl;
        let program_name: Ident = format_ident!("{}", idl.name);

        let accounts = generate_accounts(&idl.types, &idl.accounts, &self.struct_opts);
        let typedefs = generate_typedefs(&idl.types, &self.struct_opts);
        let ix_handlers = generate_ix_handlers(&idl.instructions);
        let ix_structs = generate_ix_structs(&idl.instructions);

        let docs = format!(
        " Anchor CPI crate generated from {} v{} using [anchor-gen](https://crates.io/crates/anchor-gen) v{}.",
        &idl.name,
        &idl.version,
        &GEN_VERSION.unwrap_or("unknown")
    );

        quote! {
            use anchor_lang::prelude::*;

            pub mod typedefs {
                //! User-defined types.
                use super::*;
                #typedefs
            }

            pub mod state {
                //! Structs of accounts which hold state.
                use super::*;
                #accounts
            }

            pub mod ix_accounts {
                //! Accounts used in instructions.
                use super::*;
                #ix_structs
            }

            use ix_accounts::*;
            pub use state::*;
            pub use typedefs::*;

            #[program]
            pub mod #program_name {
                #![doc = #docs]

                use super::*;
                #ix_handlers
            }
        }
    }
}
