mod mapper;
mod mapper_000;
mod mapper_001;
mod mapper_002;
mod mapper_003;
mod mapper_004;
mod nesrom;

use std::io::{Read, Seek};

use crate::error::LoadError;
// use crate::rom::Rom;
pub use mapper::*;
pub use nesrom::{NesHeader, NesVersion, MirrorMode, PrgRom, ChrRom, Trainner};

pub fn parse_stream<R: Read + Seek>(stream: &mut R) -> Result<(NesHeader, Box<dyn Mapper>), LoadError> {
    let (header, prg_rom, chr_rom, trainner) = nesrom::parse(stream)?;

    println!("MAPPER ID: {}", header.mapper_id);
    match header.mapper_id {
        000 => Ok((header, Box::new(mapper_000::State::new(&header, &prg_rom, &chr_rom)))),
        001 => Ok((header, Box::new(mapper_001::State::new(&header, &prg_rom, &chr_rom)))),
        002 => Ok((header, Box::new(mapper_002::State::new(&header, &prg_rom, &chr_rom)))),
        003 => Ok((header, Box::new(mapper_003::State::new(&header, &prg_rom, &chr_rom)))),
        004 => Ok((header, Box::new(mapper_004::State::new(&header, &prg_rom, &chr_rom)))),
        _ => Err(LoadError::UnsupportedMapper(header.mapper_id)),
    }
}
