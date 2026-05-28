# effie

A Rust crate for writing UEFI applications. effie maps directly to the UEFI
specification rather than abstracting over it, so if you know the spec, the API
will feel familiar. Each protocol, table, status code, and GUID has a direct
counterpart, and doc comments cite the relevant spec section so you can
cross-reference easily.

## Usage

```toml
[dependencies]
effie = { git = "https://github.com/tailwags/effie" }
```

effie owns the `efi_main` entry point and calls your `fn main() -> Result`. On
error it prints the status description to the console before returning. The `w!`
macro converts string literals to `&'static WStr` at compile time.

Unlike uefi-rs, there is no entry-point macro to apply. effie handles
initialization internally, so your `main` is a plain function:

```rust
#[unsafe(no_mangle)]
fn main() -> effie::Result {
    Ok(())
}
```

## Examples

### Hello world

```rust
#![no_main]
#![no_std]
extern crate alloc;

use effie::w;

#[unsafe(no_mangle)]
fn main() -> effie::Result {
    let mut con_out = effie::system_table().con_out()?;
    con_out.output_line(w!("Hello from effie!"))
}
```

### Opening a protocol

`open_protocol` takes a handle and your image handle as the agent. The returned
`Protocol<P>` calls `CloseProtocol` on drop. A typical pattern is opening
`LoadedImage` to get the boot device, then opening `SimpleFilesystem` on it:

```rust
#![no_main]
#![no_std]
extern crate alloc;

use effie::protocols::{LoadedImage, SimpleFilesystem};
use effie::w;

#[unsafe(no_mangle)]
fn main() -> effie::Result {
    let bs = effie::system_table().boot_services();
    let image_handle = effie::image_handle();

    let image = bs.open_protocol::<LoadedImage>(&image_handle, &image_handle)?;
    let mut fs = bs.open_protocol::<SimpleFilesystem>(image.device(), &image_handle)?;
    let mut root = fs.open_volume()?;
    // root is a FileHandle for the root directory of the boot volume; use it to open files

    let mut con_out = effie::system_table().con_out()?;
    con_out.output_line(w!("filesystem opened"))
}
```

### Locating a protocol

`locate_protocol` finds the first handle with the protocol without registering a
claim on it, so there's nothing to close:

```rust
#![no_main]
#![no_std]
extern crate alloc;

use effie::protocols::GraphicsOutput;

#[unsafe(no_mangle)]
fn main() -> effie::Result {
    let gop = effie::system_table().boot_services().locate_protocol::<GraphicsOutput>()?;
    let info = gop.current_mode_info();
    let fb_base = gop.current_mode().frame_buffer_base;
    // info.horizontal_resolution × info.vertical_resolution pixels at fb_base
    Ok(())
}
```

## What's implemented

Tables: System Table and Boot Services.

Protocols: `SimpleTextOutput`, `SimpleTextInput`, `GraphicsOutput`,
`LoadedImage`, `SimpleFilesystem`, `File`, `DevicePath`.

A built-in global allocator backed by UEFI boot services means heap types like
`Vec` and `Box` just work.

## Building

UEFI targets require `build-std`:

```sh
cargo build -Z build-std=core,alloc --target x86_64-unknown-uefi
cargo build -Z build-std=core,alloc --target aarch64-unknown-uefi
```

## Scope

effie is developed alongside [bread](https://github.com/bread-bootloader/bread),
an experimental UEFI bootloader for Linux. Coverage is driven by what bread
needs. Contributions adding protocols or services are welcome.

## Third-party code

The `guid` module (`src/guid/`) is vendored from the
[`uguid`](https://github.com/google/gpt-disk-rs/tree/main/uguid) crate, part of
[gpt-disk-rs](https://github.com/google/gpt-disk-rs) by Google LLC. It is used
here under the Apache-2.0 license. Copyright 2022 Google LLC. The original
copyright notices are preserved in the source files.

## License

This project is licensed under the
[APACHE-2.0 LICENSE](http://www.apache.org/licenses/LICENSE-2.0). You can find
more info in the [LICENSE](LICENSE) file.
