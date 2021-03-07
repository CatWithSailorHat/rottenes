use crate::{bitmisc::U8BitTest, rom::Rom};
use crate::cpu;
use crate::ppu;

use crate::mapper;
use crate::error::LoadError;

use std::{io::{Cursor}, num::Wrapping, path::Path};
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;

pub enum DmaState {
    NoDma,
    OmaDma(u8),
}

bitflags! {
    pub struct StandardInput: u8 {
        const RIGHT =  1 << 0;
        const LEFT =   1 << 1;
        const DOWN =   1 << 2;
        const UP =     1 << 3;
        const START =  1 << 4;
        const SELECT = 1 << 5;
        const B =      1 << 6;
        const A =      1 << 7;
    }
}

pub struct State {
    ppu: ppu::State,
    mos6502: cpu::State,
    mapper: Option<Box<dyn mapper::Mapper>>,
    ram: [u8; 0x800],
    cpu_cycle: Wrapping<usize>,
    dma_state: DmaState,
    frame_generated: bool,

    input_1_offset: usize,
    input_2_offset: usize,
    input_1_mask: StandardInput,
    input_2_mask: StandardInput,
    input_strobe: bool,
}

impl State {
    pub fn new() -> Self {
        State {
            ppu: ppu::State::new(),
            mos6502: cpu::State::new(),
            mapper: None,
            ram: [0; 0x800],
            cpu_cycle: Wrapping(0),
            dma_state: DmaState::NoDma,
            frame_generated: false,
            input_1_offset: 0,
            input_2_offset: 0,
            input_1_mask: StandardInput::empty(),
            input_2_mask: StandardInput::empty(),
            input_strobe: false,
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
            cpu::Interface::step(self);
        }
        self.state_mut().frame_generated = false;
        // self.clear_input_mask();
    }

    fn reset(&mut self) {
        cpu::Interface::reset(self);
    }

    fn get_cycle(&self) -> usize {
        self.state().cpu_cycle.0
    }

    fn get_framebuffer(&self) -> &ppu::FrameBuffer {
        ppu::Interface::get_framebuffer(self)
    }

    fn dbg_list_palette_ram(&self) -> [ppu::RgbColor; 32] {
        let mut result = [ppu::RgbColor::default(); 32];
        for i in 0x00..=0x1fusize {
            let rgb = self.state().ppu.palette.get_rgb(self.state().ppu.palette_ram[i] as usize);
            result[i] = rgb;
        }
        result
    }

    fn set_input_1(&mut self, input_1: StandardInput, value: bool) {
        self.state_mut().input_1_mask.set(input_1, value);
    }
}

impl<C: Context> cpu::Context for C {
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
            for i in 0usize..=255 {
                self.on_cpu_cycle();
                let value = Private::access(self, base_read_addr + i as u16, AccessMode::Read);
                self.on_cpu_cycle();
                let index = (i + self.state().ppu.oamaddr) & 0xFF;
                self.state_mut().ppu.oamdata[index] = value;
            }
            self.state_mut().dma_state = DmaState::NoDma;
        }

        self.on_cpu_cycle();
        Private::access(self, addr, AccessMode::Read)
    }

    fn poke(&mut self, addr: u16, val: u8) {
        self.on_cpu_cycle();
        Private::access(self, addr, AccessMode::Write(val));
    }

    fn state(&self) -> &cpu::State {
        &self.state().mos6502
    }

    fn state_mut(&mut self) -> &mut cpu::State {
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
    fn clear_input_mask(&mut self) {
        self.state_mut().input_1_mask = StandardInput::empty();
        self.state_mut().input_2_mask = StandardInput::empty();
    }

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
                    (_, _) => panic!("Invalid register access {:x}", addr),
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
            0x4016 => {
                match mode {
                    AccessMode::Read => {
                        if !self.state_mut().input_strobe {
                            let d0 = if ((self.state().input_1_mask.bits << self.state().input_1_offset) & 0b1000_0000) == 0 { 
                                0u8 
                            } else { 
                                1u8 
                            } << 0;
                            self.state_mut().input_1_offset += 1;
                            d0
                        }
                        else {
                            0u8
                        }
                    },
                    AccessMode::Write(value) => {
                        self.state_mut().input_strobe = value.is_b0_set();
                        if self.state().input_strobe {
                            self.state_mut().input_1_offset = 0;
                            self.state_mut().input_2_offset = 0;
                        }
                        value
                    }
                }
            },
            0x4017 => {
                0
            }
            0x4000..=0x4013 | 0x4018..=0x401F | 0x4015 => {
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
