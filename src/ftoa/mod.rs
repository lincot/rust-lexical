//! Fast lexical float-to-string conversion routines.

// Re-export the modules
mod util;
mod basen;

cfg_if! {
    if #[cfg(feature = "grisu3")] {
        mod grisu3;
    } else if #[cfg(feature = "ryu")] {
        mod ryu;
    }
    else {
        mod float;
        mod grisu2;
    }
}

mod api;

// Re-exports
pub(crate) use self::util::exponent_notation_char;
pub use self::api::*;
