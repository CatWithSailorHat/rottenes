use crate::{orphan::Orphan, rom::Rom};
use crate::mos6502;
use crate::ppu;

use crate::bitmisc::U8BitTest;
use crate::mapper;
use crate::error::LoadError;

use std::{io::{Cursor, ErrorKind}, path::Path};
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;

pub struct State {
    ppu: ppu::State,
    mos6502: mos6502::State,
    mapper: Option<Box<dyn mapper::Mapper>>,
}

pub trait Context: Sized {
    fn state_mut( &mut self ) -> &mut State;
    fn state( &self ) -> &State;

    fn on_cycle(&mut self) {}
}

pub trait Interface: Sized + Context {
    fn load_rom_from_file(&mut self, path: &Path) -> Result<(), LoadError>  {
        let mut file = File::open(path).unwrap();
        Private::load_from_stream(self, &mut file)
    }

    fn load_rom_from_bytes(&mut self, data: &[u8]) -> Result<(), LoadError>  {
        let mut stream = Cursor::new(data);
        Private::load_from_stream(self, &mut stream)
    }
}


impl<T: Context> Interface for T {}
impl<T: Context> Private for T {}
trait Private: Sized + Context {
    fn load_from_stream<R: Read + Seek>(&mut self, stream: &mut R) -> Result<(), LoadError> {
        let rom = Rom::parse(stream)?;
        let mapper = mapper::create_mapper(rom)?;
        self.state_mut().mapper = Some(mapper);
        Ok(())
    }
}

impl<C: Context> mos6502::Context for Orphan<C> {
    fn peek(&mut self, addr: u16) -> u8 {
        todo!()
    }

    fn poke(&mut self, addr: u16, val: u8) {
        todo!()
    }

    fn state(&self) -> &mos6502::State {
        todo!()
    }

    fn state_mut(&mut self) -> &mut mos6502::State {
        todo!()
    }

    fn skip_one_cycle(&mut self) {
        todo!()
    }
}

impl<C: Context> ppu::Context for Orphan<C> {
    fn peek_vram(&mut self, addr: u16) -> u8 {
        todo!()
    }

    fn poke_vram(&mut self, addr: u16, val: u8) {
        todo!()
    }

    fn state(&self) -> &ppu::State {
        todo!()
    }

    fn state_mut(&mut self) -> &mut ppu::State {
        todo!()
    }

    fn set_nmi(&mut self, value: bool) {
        todo!()
    }

    fn generate_frame(&mut self) {
        todo!()
    }
}
