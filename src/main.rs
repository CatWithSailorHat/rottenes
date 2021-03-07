use std::{path::Path};

#[macro_use]
extern crate bitflags;

extern crate sdl2; 


mod cpu;
mod bitmisc;
mod ppu;
mod nes;
mod orphan;
mod error;
mod mapper;
mod rom;
mod gui;

fn main() {
    let mut gui = gui::GuiObject::new();
    gui.load_rom_from_file(Path::new("./test-roms/nestest.nes")).unwrap();
    gui.run();
    println!("Hello, rottenes!");
}
