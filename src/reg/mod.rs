//! Memory-mapped registers.
//!
//! # Mappings and Tokens
//!
//! Most register should be already mapped by platform crates.
//!
//! ```
//! # #![feature(proc_macro)]
//! # use std as core;
//! # #[macro_use] extern crate drone_core;
//! use core::mem::size_of_val;
//! use drone_core::reg::prelude::*;
//! use drone_core::reg::{tokens, mappings};
//!
//! mappings! {
//!   /// SysTick timer.
//!   STK; // block name
//!
//!   /// SysTick control and status register.
//!   CTRL { // register name
//!     0xE000_E010 // memory address
//!     0x20 // bit size
//!     0x0000_0000 // reset value
//!     RReg WReg; // list of marker traits for the register
//!
//!     /// Counter enable.
//!     ENABLE { // field name
//!       0 // offset
//!       1 // width
//!       RRRegField WWRegField // list of marker traits for the field
//!     }
//!   }
//! }
//!
//! tokens! {
//!   /// Register tokens.
//!   RegIndex;
//!
//!   STK {
//!     /// SysTick control and status register.
//!     CTRL;
//!   }
//! }
//!
//! fn main() {
//!   let regs = unsafe { RegIndex::new() };
//!   assert_eq!(size_of_val(&regs.stk_ctrl.enable), 0);
//!   assert_eq!(size_of_val(&regs.stk_ctrl), 0);
//!   assert_eq!(size_of_val(&regs), 0);
//! }
//! ```

pub mod marker;
pub mod prelude;

mod field;
mod hold;
#[cfg_attr(feature = "clippy", allow(module_inception))]
mod reg;
mod tag;

pub use self::field::*;
pub use self::hold::*;
pub use self::reg::*;
pub use self::tag::*;
pub use drone_core_macros::{reg_mappings as mappings, reg_tokens as tokens};

/// Forkable token.
pub trait RegFork {
  /// Returns a duplicate of the token.
  fn fork(&mut self) -> Self;
}

/// A set of register tokens.
pub trait RegTokens {
  /// Creates a new set of register tokens.
  ///
  /// # Safety
  ///
  /// Must be called no more than once.
  unsafe fn new() -> Self;
}
