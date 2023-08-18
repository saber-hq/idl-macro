//! Generates Rust code from an Anchor IDL.

pub use anchor_syn::idl::*;

mod account;
mod event;
mod instruction;
mod program;
mod state;
mod typedef;

pub use account::*;
pub use event::*;
pub use instruction::*;
pub use program::*;
pub use state::*;
pub use typedef::*;

/// Version of anchor-idl.
pub const GEN_VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

/// Converts an [IdlType] to a [String] of the Rust representation.
pub fn ty_to_rust_type(ty: &IdlType) -> String {
    match ty {
        IdlType::Bool => "bool".to_string(),
        IdlType::U8 => "u8".to_string(),
        IdlType::I8 => "i8".to_string(),
        IdlType::U16 => "u16".to_string(),
        IdlType::I16 => "i16".to_string(),
        IdlType::U32 => "u32".to_string(),
        IdlType::I32 => "i32".to_string(),
        IdlType::F32 => "f32".to_string(),
        IdlType::U64 => "u64".to_string(),
        IdlType::I64 => "i64".to_string(),
        IdlType::F64 => "f64".to_string(),
        IdlType::U128 => "u128".to_string(),
        IdlType::I128 => "i128".to_string(),
        IdlType::Bytes => "Vec<u8>".to_string(),
        IdlType::String => "String".to_string(),
        IdlType::PublicKey => "Pubkey".to_string(),
        IdlType::Option(inner) => format!("Option<{}>", ty_to_rust_type(inner)),
        IdlType::Vec(inner) => format!("Vec<{}>", ty_to_rust_type(inner)),
        IdlType::Array(ty, size) => format!("[{}; {}]", ty_to_rust_type(ty), size),
        IdlType::Defined(name) => name.to_string(),
    }
}
