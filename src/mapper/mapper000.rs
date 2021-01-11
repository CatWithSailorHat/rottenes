use super::Mapper;
use crate::rom::Rom;

pub struct State {
    rom: Rom,
}

impl State {
    pub fn new(rom: Rom) -> State {
        State {
            rom
        }
    }
}

impl Mapper for State {
    fn peek(&mut self, addr: u16) -> u8 {
        todo!()
    }

    fn poke(&mut self, addr: u16, val: u8) {
        todo!()
    }

    fn vpeek(&mut self, addr: u16) -> u8 {
        todo!()
    }

    fn vpoke(&mut self, addr: u16, val: u8) {
        todo!()
    }
}