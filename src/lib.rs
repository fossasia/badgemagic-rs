#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

pub mod protocol;

#[cfg(feature = "usb-hid")]
pub mod usb_hid;

#[cfg(feature = "embedded-graphics")]
pub mod util;

#[cfg(feature = "embedded-graphics")]
pub use embedded_graphics;

#[cfg(feature = "cli")]
#[doc(hidden)]
pub mod cli {
    include!(concat!(env!("OUT_DIR"), "/cli.rs"));
}
