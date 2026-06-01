//! Logging macros that forward to the [`log`] crate when the `log` feature is
//! enabled, and silently discard all arguments otherwise.
//!
//! Use these instead of gating every call site with `#[cfg(feature = "log")]`:
//!
//! ```ignore
//! use crate::log;
//!
//! log::trace!("open_protocol: guid={}", P::GUID);
//! log::warn!("firmware returned null interface");
//! ```

#[cfg(feature = "log")]
pub use ::log::{debug, error, info, trace, warn};

// When the `log` feature is off, define five no-op macros at the crate root
// (required so they can be re-exported with full `pub` visibility) and then
// alias them into this module under the standard names.
#[cfg(not(feature = "log"))]
pub use crate::__log_noop as debug;
#[cfg(not(feature = "log"))]
pub use crate::__log_noop as error;
#[cfg(not(feature = "log"))]
pub use crate::__log_noop as info;
#[cfg(not(feature = "log"))]
pub use crate::__log_noop as trace;
#[cfg(not(feature = "log"))]
pub use crate::__log_noop as warn;

#[cfg(not(feature = "log"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __log_noop {
    ($($t:tt)*) => { if false { let _ = ::core::format_args!($($t)*); } };
}
