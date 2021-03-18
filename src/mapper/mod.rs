mod mapper000;


use crate::error::LoadError;
use crate::rom::Rom;

pub trait Mapper {
    fn peek_expansion_rom(&mut self, addr: u16) -> u8;
    fn poke_expansion_rom(&mut self, addr: u16, val: u8);
    
    fn peek_sram(&mut self, addr: u16) -> u8;
    fn poke_sram(&mut self, addr: u16, val: u8);

    fn peek_prg_rom(&mut self, addr: u16) -> u8;
    fn poke_prg_rom(&mut self, addr: u16, val: u8);

    fn vpeek_nametable(&mut self, addr: u16) -> u8;
    fn vpoke_nametable(&mut self, addr: u16, val: u8);

    fn vpeek_pattern(&mut self, addr: u16) -> u8;
    fn vpoke_pattern(&mut self, addr: u16, val: u8);

    fn load_state(&mut self, state: Vec<u8>);
    fn save_state(&self) -> Vec<u8>;
}

pub fn create_mapper(rom: Rom) -> Result<Box<dyn Mapper>, LoadError> {
    match rom.mapper_id {
        000 => { Ok(Box::new(mapper000::Mappper000::new(rom))) }
        _ => Err(LoadError::UnsupportedMapper(rom.mapper_id))
    }
}


