use std::path::Path;

#[macro_use]
extern crate sdl2; 
extern crate nes;

mod gui;

fn main() {
    let mut gui = gui::GuiObject::new();
    gui.load_rom_from_file(Path::new("../test-roms/spritecans.nes")).unwrap();
    gui.run();
    println!("Hello, rottenes!");
    
}


