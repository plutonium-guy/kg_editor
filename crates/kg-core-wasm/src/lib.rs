//! wasm-bindgen JS bindings for kg-core.

#![cfg(target_arch = "wasm32")]
#![deny(unsafe_code)]

mod schema;
mod uow;
mod value;

pub use schema::*;
pub use uow::*;
pub use value::*;
