#[macro_use]
extern crate bitflags;
extern crate serde;
extern crate bincode;

mod cpu;
mod bitmisc;
mod ppu;
mod error;
mod mapper;
mod rom;
mod emulator;

pub use emulator::{StandardInput, Emulator};
pub use error::LoadError;
