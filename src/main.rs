use std::{fs::File, path::Path};

use rom::Rom;

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
    let mut test_file = File::open(Path::new("./tests/rom/nestest.nes")).unwrap();
        let rom = Rom::parse(&mut test_file).unwrap();
        println!("{}", rom.mapper_id);
        println!("{:X}", rom.prg_rom.len());
        println!("{:X}", rom.chr_rom.len());
    println!("Hello, rottenes!");
}
