use std::io::Read;
use std::path::Path;

use std::fs::File;

#[macro_use]
extern crate sdl2; 
extern crate nes;

mod gui;

fn main() {
    let path_str = String::from("../test-roms/spritecans.nes");
    println!("{}", path_str);

    let mut gui = gui::GuiObject::new();
    gui.load_rom_from_file(Path::new(&path_str)).unwrap();
    gui.run();
    println!("Hello, rottenes!");
}


