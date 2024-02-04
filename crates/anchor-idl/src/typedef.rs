use std::collections::BTreeMap;

use anchor_syn::idl::{EnumFields, IdlEnumVariant, IdlField, IdlType, IdlTypeDefinition};
use heck::ToSnakeCase;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::StructOpts;

#[derive(Copy, Clone, Debug, Default)]
pub struct FieldListProperties {
    pub can_copy: bool,
    pub can_derive_default: bool,
}

pub fn get_field_list_properties(
    defs: &[IdlTypeDefinition],
    fields: &[IdlField],
) -> FieldListProperties {
    get_type_list_properties(
        defs,
        &fields.iter().map(|f| f.ty.clone()).collect::<Vec<_>>(),
    )
}

pub fn get_type_list_properties(
    defs: &[IdlTypeDefinition],
    fields: &[IdlType],
) -> FieldListProperties {
    fields.iter().fold(
        FieldListProperties {
            can_copy: true,
            can_derive_default: true,
        },
        |acc, el| {
            let inner_props = get_type_properties(defs, el);
            let can_copy = acc.can_copy && inner_props.can_copy;
            let can_derive_default = acc.can_derive_default && inner_props.can_derive_default;
            FieldListProperties {
                can_copy,
                can_derive_default,
            }
        },
    )
}

pub fn get_variant_list_properties(
    defs: &[IdlTypeDefinition],
    variants: &[IdlEnumVariant],
) -> FieldListProperties {
    variants.iter().fold(
        FieldListProperties {
            can_copy: true,
            can_derive_default: true,
        },
        |acc, el| {
            let props = match &el.fields {
                Some(EnumFields::Named(fields)) => get_field_list_properties(defs, fields),
                Some(EnumFields::Tuple(fields)) => get_type_list_properties(defs, fields),
                None => acc,
            };
            FieldListProperties {
                can_copy: acc.can_copy && props.can_copy,
                can_derive_default: acc.can_derive_default && props.can_derive_default,
            }
        },
    )
}

pub fn get_type_properties(defs: &[IdlTypeDefinition], ty: &IdlType) -> FieldListProperties {
    match ty {
        IdlType::Bool
        | IdlType::U8
        | IdlType::I8
        | IdlType::U16
        | IdlType::I16
        | IdlType::U32
        | IdlType::I32
        | IdlType::F32
        | IdlType::U64
        | IdlType::I64
        | IdlType::F64
        | IdlType::U128
        | IdlType::I128
        | IdlType::PublicKey => FieldListProperties {
            can_copy: true,
            can_derive_default: true,
        },
        IdlType::Bytes => FieldListProperties {
            can_copy: false,
            can_derive_default: false,
        },
        IdlType::String | IdlType::Vec(_) => FieldListProperties {
            can_copy: false,
            can_derive_default: true,
        },
        IdlType::Defined(inner) => {
            let def = defs.iter().find(|def| def.name == *inner).unwrap();
            match &def.ty {
                anchor_syn::idl::IdlTypeDefinitionTy::Struct { fields } => {
                    get_field_list_properties(defs, fields)
                }
                anchor_syn::idl::IdlTypeDefinitionTy::Enum { variants } => {
                    get_variant_list_properties(defs, variants)
                }
            }
        }
        IdlType::Option(inner) => get_type_properties(defs, inner),
        IdlType::Array(inner, len) => {
            let inner = get_type_properties(defs, inner);
            let can_derive_array_len = *len <= 32;
            FieldListProperties {
                can_copy: inner.can_copy,
                can_derive_default: can_derive_array_len && inner.can_derive_default,
            }
        }
    }
}

/// Generates struct fields from a list of [IdlField]s.
pub fn generate_fields(fields: &[IdlField]) -> TokenStream {
    let fields_rendered = fields.iter().map(|arg| {
        let name = format_ident!("{}", arg.name.to_snake_case());
        let type_name = crate::ty_to_rust_type(&arg.ty);
        let stream: proc_macro2::TokenStream = type_name.parse().unwrap();
        quote! {
            pub #name: #stream
        }
    });
    quote! {
        #(#fields_rendered),*
    }
}

/// Generates struct fields from a list of [IdlField]s.
pub fn generate_enum_fields(fields: &[IdlField]) -> TokenStream {
    let fields_rendered = fields.iter().map(|arg| {
        let name = format_ident!("{}", arg.name.to_snake_case());
        let type_name = crate::ty_to_rust_type(&arg.ty);
        let stream: proc_macro2::TokenStream = type_name.parse().unwrap();
        quote! {
            #name: #stream
        }
    });
    quote! {
        #(#fields_rendered),*
    }
}

/// Generates a struct.
pub fn generate_struct(
    defs: &[IdlTypeDefinition],
    struct_name: &Ident,
    fields: &[IdlField],
    opts: StructOpts,
) -> TokenStream {
    let fields_rendered = generate_fields(fields);
    let props = get_field_list_properties(defs, fields);

    let derive_default = if props.can_derive_default {
        quote! {
            #[derive(Default)]
        }
    } else {
        quote! {}
    };
    let derive_serializers = if opts.zero_copy {
        let repr = if opts.packed {
            quote! {
                #[repr(packed)]
            }
        } else {
            quote! {
                #[repr(C)]
            }
        };
        quote! {
            #[zero_copy]
            #repr
        }
    } else {
        let derive_copy = if props.can_copy {
            quote! {
                #[derive(Copy)]
            }
        } else {
            quote! {}
        };
        quote! {
            #[derive(AnchorSerialize, AnchorDeserialize, Clone)]
            #derive_copy
        }
    };

    quote! {
        #derive_serializers
        #[derive(Debug)]
        #derive_default
        pub struct #struct_name {
            #fields_rendered
        }
    }
}

/// Generates an enum.
pub fn generate_enum(
    defs: &[IdlTypeDefinition],
    enum_name: &Ident,
    variants: &[IdlEnumVariant],
) -> TokenStream {
    let variant_idents = variants.iter().map(|v| {
        let name = format_ident!("{}", v.name);
        match &v.fields {
            Some(EnumFields::Named(idl_fields)) => {
                let fields = generate_enum_fields(idl_fields);
                quote! {
                  #name {
                    #fields
                  }
                }
            }
            _ => {
                quote! {
                  #name
                }
            }
        }
    });
    let props = get_variant_list_properties(defs, variants);

    let derive_copy = if props.can_copy {
        quote! {
            #[derive(Copy)]
        }
    } else {
        quote! {}
    };

    let default_impl = match variants.first() {
        Some(IdlEnumVariant {
            fields: Some(EnumFields::Named(fields)),
            ..
        }) if fields.len() > 0 => {
            quote! {}
        }
        _ => {
            let default_variant = format_ident!("{}", variants.first().unwrap().name);
            quote! {
              impl Default for #enum_name {
                  fn default() -> Self {
                      Self::#default_variant
                  }
              }
            }
        }
    };

    quote! {
        #[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
        #derive_copy
        pub enum #enum_name {
            #(#variant_idents),*
        }

        #default_impl
    }
}

/// Generates structs and enums.
pub fn generate_typedefs(
    typedefs: &[IdlTypeDefinition],
    struct_opts: &BTreeMap<String, StructOpts>,
) -> TokenStream {
    let defined = typedefs.iter().map(|def| {
        let struct_name = format_ident!("{}", def.name);
        match &def.ty {
            anchor_syn::idl::IdlTypeDefinitionTy::Struct { fields } => {
                let opts = struct_opts.get(&def.name).copied().unwrap_or_default();
                generate_struct(typedefs, &struct_name, fields, opts)
            }
            anchor_syn::idl::IdlTypeDefinitionTy::Enum { variants } => {
                generate_enum(typedefs, &struct_name, variants)
            }
        }
    });
    quote! {
        #(#defined)*
    }
}
