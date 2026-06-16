//! Feature-gated zero-copy parsers for common network protocols.
//!
//! Each protocol lives behind its own Cargo feature and is generated at build
//! time from a bytesmith DSL spec in `specs/`. Enabling a feature exposes a
//! module of the same name holding the generated parser types. No protocol is
//! enabled by default; turn on the ones you need or enable `all`.

extern crate self as bytesmith_protocols;

pub mod hooks;

include!(concat!(env!("OUT_DIR"), "/protocols.rs"));
