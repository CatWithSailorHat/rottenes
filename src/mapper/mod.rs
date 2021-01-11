mod mapper000;

use crate::error::LoadError;
use crate::rom::Rom;

pub trait Mapper {
    fn peek(&mut self, addr: u16) -> u8;
    fn poke(&mut self, addr: u16, val: u8);
    fn vpeek(&mut self, addr: u16) -> u8;
    fn vpoke(&mut self, addr: u16, val: u8);
}

pub fn create_mapper(rom: Rom) -> Result<Box<dyn Mapper>, LoadError> {
    match rom.mapper_id {
        000 => { Ok(Box::new(mapper000::State::new(rom))) }
        _ => Err(LoadError::UnsupportedMapper(rom.mapper_id))
    }
}