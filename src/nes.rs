use crate::{orphan::Orphan, rom::Rom};
use crate::mos6502;
use crate::ppu;

use crate::mapper;
use crate::error::LoadError;

use std::{io::{Cursor}, num::Wrapping, path::Path, unimplemented};
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;

pub enum DmaState {
    NoDma,
    OmaDma(u8),
}

pub struct State {
    ppu: ppu::State,
    mos6502: mos6502::State,
    mapper: Option<Box<dyn mapper::Mapper>>,
    ram: [u8; 0x800],
    cpu_cycle: Wrapping<usize>,
    dma_state: DmaState,
    frame_generated: bool,
}

impl State {
    pub fn new() -> Self {
        State {
            ppu: ppu::State::new(),
            mos6502: mos6502::State::new(),
            mapper: None,
            ram: [0; 0x800],
            cpu_cycle: Wrapping(0),
            dma_state: DmaState::NoDma,
            frame_generated: false,
        }
    }
}

pub trait Context: Sized {
    fn state_mut( &mut self ) -> &mut State;
    fn state( &self ) -> &State;

    fn on_cycle(&mut self) {}
    fn on_frame(&mut self) {}
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

    fn run_for_one_frame(&mut self) {
        while !self.state().frame_generated {
            mos6502::Interface::step(self);
        }
        self.state_mut().frame_generated = false;
    }

    fn reset(&mut self) {
        mos6502::Interface::reset(self);
    }

    fn get_cycle(&self) -> usize {
        self.state().cpu_cycle.0
    }

    fn get_framebuffer(&self) -> &ppu::FrameBuffer {
        ppu::Interface::get_framebuffer(self)
    }
}

impl<C: Context> mos6502::Context for C {
    fn peek(&mut self, addr: u16) -> u8 {
        // dma hijack
        if let DmaState::OmaDma(v) = self.state().dma_state {
            self.on_cpu_cycle();
            Private::access(self, addr, AccessMode::Read);

            if self.get_cycle() & 1 == 1 {  // not on `dma get cycle`
                self.on_cpu_cycle();
                Private::access(self, addr, AccessMode::Read);
            }
            
            let base_read_addr = (v as u16) << 8;
            for i in 0usize..=511 {
                self.on_cpu_cycle();
                let value = Private::access(self, base_read_addr + i as u16, AccessMode::Read);
                self.on_cpu_cycle();
                self.state_mut().ppu.oamdata[i] = value;
            }
        }

        self.on_cpu_cycle();
        Private::access(self, addr, AccessMode::Read)
    }

    fn poke(&mut self, addr: u16, val: u8) {
        self.on_cpu_cycle();
        Private::access(self, addr, AccessMode::Write(val));
    }

    fn state(&self) -> &mos6502::State {
        &self.state().mos6502
    }

    fn state_mut(&mut self) -> &mut mos6502::State {
        &mut self.state_mut().mos6502
    }
}

impl<C: Context> ppu::Context for C {
    fn peek_vram(&mut self, addr: u16) -> u8 {
        Private::vaccess(self, addr, AccessMode::Read)
    }

    fn poke_vram(&mut self, addr: u16, val: u8) {
        Private::vaccess(self, addr, AccessMode::Write(val));
    }

    fn state(&self) -> &ppu::State {
        &self.state().ppu
    }

    fn state_mut(&mut self) -> &mut ppu::State {
        &mut self.state_mut().ppu
    }

    fn generate_frame(&mut self) {
        self.state_mut().frame_generated = true;
    }

    fn trigger_nmi(&mut self) {
        self.state_mut().mos6502.nmi = true;
    }
}

impl<T: Context> Interface for T {}
impl<T: Context> Private for T {}

enum AccessMode {
    Read,
    Write(u8),
}
trait Private: Sized + Context {
    fn load_from_stream<R: Read + Seek>(&mut self, stream: &mut R) -> Result<(), LoadError> {
        let rom = Rom::parse(stream)?;
        let mapper = mapper::create_mapper(rom)?;
        self.state_mut().mapper = Some(mapper);
        Ok(())
    }

    fn access(&mut self, addr: u16, mode: AccessMode) -> u8 {
        match addr {
            0x0000..=0x1FFF => {
                match mode {
                    AccessMode::Read => {
                        self.state_mut().ram[(addr & 0x7FF) as usize]
                    },
                    AccessMode::Write(value) => {
                        self.state_mut().ram[(addr & 0x7FF) as usize] = value; value
                    }
                }
            },
            0x2000..=0x3FFF => {
                match (addr & 7, mode) {
                    (0, AccessMode::Write(value)) => {
                        ppu::Interface::write_ppuctrl(self, value); value
                    },
                    (1, AccessMode::Write(value)) => {
                        ppu::Interface::write_ppumask(self, value); value
                    },
                    (2, AccessMode::Read) => {
                        ppu::Interface::read_ppustatus(self)
                    },
                    (3, AccessMode::Write(value)) => {
                        ppu::Interface::write_oamaddr(self, value); value
                    },
                    (4, AccessMode::Read) => {
                        ppu::Interface::read_oamdata(self)
                    },
                    (4, AccessMode::Write(value)) => {
                        ppu::Interface::write_oamdata(self, value); value
                    }
                    (5, AccessMode::Write(value)) => {
                        ppu::Interface::write_ppuscroll(self, value); value
                    },
                    (6, AccessMode::Write(value)) => {
                        ppu::Interface::write_ppuaddr(self, value); value
                    }
                    (7, AccessMode::Read) => {
                        ppu::Interface::read_ppudata(self)
                    },
                    (7, AccessMode::Write(value)) => {
                        ppu::Interface::write_ppudata(self, value); value
                    }
                    (_, _) => panic!("Invalid register access"),
                }
            },
            0x4014 => {
                match mode {
                    AccessMode::Read => panic!("Invalid dma port access"),
                    AccessMode::Write(value) => {
                        self.state_mut().dma_state = DmaState::OmaDma(value);
                        value
                    }
                }
                
            },
            0x4000..=0x4013 | 0x4015..=0x401F => {
                0  // FIXME
            },
            0x4020..=0x5FFF => {
                let mapper = self.state_mut().mapper.as_mut().unwrap();
                match mode {
                    AccessMode::Read => {
                        mapper.peek_expansion_rom(addr)
                    },
                    AccessMode::Write(value) => {
                        mapper.poke_expansion_rom(addr, value); value
                    }
                }
            }
            0x6000..=0x7FFF => {
                let mapper = self.state_mut().mapper.as_mut().unwrap();
                match mode {
                    AccessMode::Read => {
                        mapper.peek_sram(addr)
                    },
                    AccessMode::Write(value) => {
                        mapper.poke_sram(addr, value); value
                    }
                }
            }
            0x8000..=0xFFFF => {
                let mapper = self.state_mut().mapper.as_mut().unwrap();
                match mode {
                    AccessMode::Read => {
                        mapper.peek_prg_rom(addr)
                    },
                    AccessMode::Write(value) => {
                        mapper.poke_prg_rom(addr, value); value
                    }
                }
            }
        }
    }

    fn vaccess(&mut self, addr: u16, mode: AccessMode) -> u8 {
        match addr {
            0x0000..= 0x1FFF => {
                let mapper = self.state_mut().mapper.as_mut().unwrap();
                match mode {
                    AccessMode::Read => {
                        mapper.vpeek_pattern(addr)
                    },
                    AccessMode::Write(value) => {
                        mapper.vpoke_pattern(addr, value); value
                    }
                }
            },
            0x2000..=0x3EFF => {
                let addr = addr & 0x2FFF;
                let mapper = self.state_mut().mapper.as_mut().unwrap();
                match mode {
                    AccessMode::Read => {
                        mapper.vpeek_nametable(addr)
                    },
                    AccessMode::Write(value) => {
                        mapper.vpoke_nametable(addr, value); value
                    }
                }
            },
            _ => unreachable!()
        }
    }

    fn on_cpu_cycle(&mut self) {
        self.on_cycle();
        self.state_mut().cpu_cycle += Wrapping(1);
        ppu::Interface::tick(self);
        ppu::Interface::tick(self);
        ppu::Interface::tick(self);
    }
}
