#[macro_use]
extern crate bitflags;

mod mos6502;
mod bitmisc;
mod ppu;

fn main() {
    println!("Hello, rottenes!");
    let q = 1;
    match q {
        1 => 1,
        1 => 2,
        _ => 0,
    };
}
