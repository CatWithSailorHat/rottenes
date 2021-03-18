use crate::{bitmisc::U8BitTest, ppu::Interface, rom::Rom};
use crate::cpu;
use crate::ppu;

use crate::mapper;
use crate::error::LoadError;
use crate::nes;

use std::{io::{Cursor}, num::Wrapping, path::Path};
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;

use serde::{Serialize, Deserialize};
use bincode;

pub struct Emulator {
    mapper: Option<Box<dyn mapper::Mapper>>,
    nes: nes::State,
}

impl nes::Context for Emulator {
    fn state_mut(&mut self) -> &mut nes::State {
        &mut self.nes
    }

    fn state(&self) -> &nes::State {
        &self.nes
    }

    fn mapper(&mut self) -> &mut Box<dyn mapper::Mapper> {
        self.mapper.as_mut().unwrap()
    }
}

impl Emulator {
    pub fn new() -> Self {
        Emulator {
            mapper: None,
            nes: nes::State::new(),
        }
    }

    pub fn load_rom_from_file(&mut self, path: &Path) -> Result<(), LoadError>  {
        let mut file = File::open(path).unwrap();
        self.load_from_stream(&mut file)
    }

    pub fn load_rom_from_bytes(&mut self, data: &[u8]) -> Result<(), LoadError>  {
        let mut stream = Cursor::new(data);
        self.load_from_stream(&mut stream)
    }

    pub fn get_framebuffer(&self) -> &Vec<ppu::RgbColor> {
        nes::Interface::get_framebuffer(self)
    }

    pub fn set_input_1(&mut self, input_1: nes::StandardInput, value: bool) {
        nes::Interface::set_input_1(self, input_1, value)
    }

    pub fn run_for_one_frame(&mut self) {
        nes::Interface::run_for_one_frame(self);
    }

    pub fn load_state(&mut self, state: &Vec<u8>) {
        let (serialized_nes, serialized_mapper): (Vec<u8>, Vec<u8>) = bincode::deserialize(&state[..]).unwrap();
        self.nes = bincode::deserialize(&serialized_nes[..]).unwrap();
        self.mapper.as_mut().unwrap().load_state(serialized_mapper);
    }

    pub fn reset(&mut self) {
        nes::Interface::reset(self)
    }

    pub fn save_state(&mut self) -> Vec<u8> {
        let serialized_nes = bincode::serialize(&self.nes).unwrap();
        let serialized_mapper = self.mapper.as_mut().unwrap().save_state();
        bincode::serialize(&(serialized_nes, serialized_mapper)).unwrap()
    }

    fn load_from_stream<R: Read + Seek>(&mut self, stream: &mut R) -> Result<(), LoadError> {
        let rom = Rom::parse(stream)?;
        let mapper = mapper::create_mapper(rom)?;
        self.mapper = Some(mapper);
        Ok(())
    }
}
