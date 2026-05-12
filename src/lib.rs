//! Fast image and GIF captioning library.
//!
//! Caption adds a White bar above the image with auto-sized black text. Supports multiline
//! word-wrap, color emoji, Japanese text faces, and uses super epic fast quantization technology.
//!
//! # Example
//!
//! ```no_run
//! use caption::{Captioner, OutputFormat};
//!
//! let mut captioner = Captioner::new();
//! let bytes: &[u8] = include_bytes!("../tests/fixtures/sources/static_500x500.png");
//! let out = captioner.caption_bytes(bytes, "hello world", OutputFormat::Same).unwrap();
//! assert!(!out.is_empty());
//! ```

mod captioner;
mod error;
mod font;
mod gif;
mod layout;

pub use captioner::{Captioner, OutputFormat};
pub use error::CaptionError;
