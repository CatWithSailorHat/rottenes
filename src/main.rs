#[macro_use]
extern crate bitflags;

mod mos6502;
mod bitmisc;
mod ppu;
mod nes;
mod orphan;
mod error;
mod mapper;
mod rom;

fn main() {
    println!("Hello, rottenes!");
}
