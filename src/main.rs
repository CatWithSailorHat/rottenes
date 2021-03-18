use std::{path::Path};

#[macro_use]
extern crate bitflags;
extern crate sdl2; 
extern crate serde;
extern crate bincode;

mod cpu;
mod bitmisc;
mod ppu;
mod error;
mod mapper;
mod rom;
mod gui;
mod emulator;
mod nes;

fn main() {
    let mut gui = gui::GuiObject::new();
    gui.load_rom_from_file(Path::new("/Users/hoshizora/Desktop/test-roms/smb.nes")).unwrap();
    // gui.load_rom_from_file(Path::new("test-roms/spritecans.nes")).unwrap();
    gui.run();
    println!("Hello, rottenes!");
    
}


