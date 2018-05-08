//! Drone procedural macros shared lib.
//!
//! See `drone-core` documentation for details.

#![feature(proc_macro)]
#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/drone-macros-core/0.8.1")]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(feature = "clippy", allow(precedence))]

#[macro_use]
extern crate lazy_static;
extern crate proc_macro;
extern crate proc_macro2;
extern crate regex;
#[macro_use]
extern crate syn;

mod extern_static;
mod extern_struct;
mod macros;
mod new_mod;
mod new_static;
mod new_struct;
mod unkeywordize;

pub use self::extern_static::ExternStatic;
pub use self::extern_struct::ExternStruct;
pub use self::new_mod::NewMod;
pub use self::new_static::NewStatic;
pub use self::new_struct::NewStruct;
pub use self::unkeywordize::unkeywordize;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::synom::ParseError;

/// Emits a parse error on the given span.
pub fn emit_parse_err(span: Span, err: ParseError) -> TokenStream {
  emit_err(span, &format!("{}", err))
}

/// Emits an error on the given span.
pub fn emit_err(span: Span, err: &str) -> TokenStream {
  span.unstable().error(err).emit();
  TokenStream::empty()
}
