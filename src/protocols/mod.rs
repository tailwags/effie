//! UEFI protocol implementations.
//!
//! Each module in this directory wraps a UEFI protocol interface, providing safe Rust
//! methods over the raw `extern "efiapi"` function pointers.

mod device_path;
mod file;
mod graphics_output;
mod loaded_image;
mod simple_filesystem;
mod simple_text_input;
mod simple_text_output;

pub use device_path::*;
pub use file::*;
pub use graphics_output::*;
pub use loaded_image::*;
pub use simple_filesystem::*;
pub use simple_text_input::*;
pub use simple_text_output::*;
