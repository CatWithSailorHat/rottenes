use crate::{bitmisc::U8BitTest, error::LoadError};
use crate::cpu;
use crate::ppu;

use crate::mapper;

use serde::{Serialize, Deserialize};
use std::num::Wrapping;

use crate::{rom::Rom};

use std::{io::{Cursor}, path::Path};
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;

use bincode;



enum AccessMode {
    Read,
    Write(u8),
}

#[derive(Serialize, Deserialize)]
pub enum DmaState {
    NoDma,
    OmaDma(u8),
}

bitflags! {
    #[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
struct NesState {
    ppu: ppu::State,
    mos6502: cpu::State,
    ram: Vec<u8>,
    cpu_cycle: Wrapping<usize>,
    dma_state: DmaState,
    frame_generated: bool,

    input_1_offset: usize,
    input_2_offset: usize,
    input_1_mask: StandardInput,
    input_2_mask: StandardInput,
    input_strobe: bool,
}

impl NesState {
    pub fn new() -> Self {
        NesState {
            ppu: ppu::State::new(),
            mos6502: cpu::State::new(),
            ram: [0; 0x800].to_vec(),
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

pub struct Emulator {
    mapper: Option<Box<dyn mapper::Mapper>>,
    nes: NesState,
}

impl Emulator {
    pub fn new() -> Self {
        Emulator {
            mapper: None,
            nes: NesState::new(),
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

    pub fn load_state(&mut self, state: &Vec<u8>) {
        let (serialized_nes, serialized_mapper): (Vec<u8>, Vec<u8>) = bincode::deserialize(&state[..]).unwrap();
        self.nes = bincode::deserialize(&serialized_nes[..]).unwrap();
        self.mapper.as_mut().unwrap().load_state(serialized_mapper);
    }

    pub fn save_state(&mut self) -> Vec<u8> {
        let serialized_nes = bincode::serialize(&self.nes).unwrap();
        let serialized_mapper = self.mapper.as_mut().unwrap().save_state();
        bincode::serialize(&(serialized_nes, serialized_mapper)).unwrap()
    }

    pub fn run_for_one_frame(&mut self) {
        while !self.nes.frame_generated {
            cpu::Interface::step(self);
        }
        self.nes.frame_generated = false;
        self.clear_input_mask();
    }

    pub fn reset(&mut self) {
        cpu::Interface::reset(self);
    }

    pub fn get_cycle(&self) -> usize {
        self.nes.cpu_cycle.0
    }

    pub fn get_framebuffer(&self) -> &Vec<ppu::RgbColor> {
        ppu::Interface::get_framebuffer(self)
    }

    pub fn dbg_list_palette_ram(&self) -> [ppu::RgbColor; 32] {
        let mut result = [ppu::RgbColor::default(); 32];
        for i in 0x00..=0x1fusize {
            let rgb = self.nes.ppu.palette.get_rgb(self.nes.ppu.palette_ram[i] as usize);
            result[i] = rgb;
        }
        result
    }

    pub fn set_input_1(&mut self, input_1: StandardInput, value: bool) {
        self.nes.input_1_mask.set(input_1, value);
    }

    fn clear_input_mask(&mut self) {
        self.nes.input_1_mask = StandardInput::empty();
        self.nes.input_2_mask = StandardInput::empty();
    }

    fn load_from_stream<R: Read + Seek>(&mut self, stream: &mut R) -> Result<(), LoadError> {
        let rom = Rom::parse(stream)?;
        let mapper = mapper::create_mapper(rom)?;
        self.mapper = Some(mapper);
        Ok(())
    }
}

impl Emulator {
    fn access(&mut self, addr: u16, mode: AccessMode) -> u8 {
        match addr {
            0x0000..=0x1FFF => {
                match mode {
                    AccessMode::Read => {
                        self.nes.ram[(addr & 0x7FF) as usize]
                    },
                    AccessMode::Write(value) => {
                        self.nes.ram[(addr & 0x7FF) as usize] = value; value
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
                        self.nes.dma_state = DmaState::OmaDma(value);
                        value
                    }
                }
                
            },
            0x4016 => {
                match mode {
                    AccessMode::Read => {
                        if !self.nes.input_strobe {
                            let d0 = if ((self.nes.input_1_mask.bits << self.nes.input_1_offset) & 0b1000_0000) == 0 { 
                                0u8 
                            } else { 
                                1u8 
                            } << 0;
                            self.nes.input_1_offset += 1;
                            d0
                        }
                        else {
                            0u8
                        }
                    },
                    AccessMode::Write(value) => {
                        self.nes.input_strobe = value.is_b0_set();
                        if self.nes.input_strobe {
                            self.nes.input_1_offset = 0;
                            self.nes.input_2_offset = 0;
                        }
                        value
                    }
                }
            },
            0x4017 => {
                match mode {
                    AccessMode::Read => {
                        if !self.nes.input_strobe {
                            let d0 = if ((self.nes.input_2_mask.bits << self.nes.input_2_offset) & 0b1000_0000) == 0 { 
                                0u8 
                            } else { 
                                1u8 
                            } << 0;
                            self.nes.input_2_offset += 1;
                            d0
                        }
                        else {
                            0u8
                        }
                    },
                    AccessMode::Write(value) => {
                        0  // FIXME
                    }
                }
            }
            0x4000..=0x4013 | 0x4018..=0x401F | 0x4015 => {
                0  // FIXME
            },
            0x4020..=0x5FFF => {
                let mapper = self.mapper.as_mut().unwrap();
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
                let mapper = self.mapper.as_mut().unwrap();
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
                let mapper =  self.mapper.as_mut().unwrap();
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
        let mapper =  self.mapper.as_mut().unwrap();
        match addr {
            0x0000..= 0x1FFF => {
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
        self.nes.cpu_cycle += Wrapping(1);
        ppu::Interface::tick(self);
        ppu::Interface::tick(self);
        ppu::Interface::tick(self);
    }
}

impl cpu::Context for Emulator {
    fn peek(&mut self, addr: u16) -> u8 {
        // dma hijack
        if let DmaState::OmaDma(v) = self.nes.dma_state {
            self.on_cpu_cycle();
            self.access(addr, AccessMode::Read);

            if self.get_cycle() & 1 == 1 {  // not on `dma get cycle`
                self.on_cpu_cycle();
                self.access(addr, AccessMode::Read);
            }
            
            let base_read_addr = (v as u16) << 8;
            for i in 0usize..=255 {
                self.on_cpu_cycle();
                let value = self.access(base_read_addr + i as u16, AccessMode::Read);
                self.on_cpu_cycle();
                let index = (i + self.nes.ppu.oamaddr) & 0xFF;
                self.nes.ppu.oamdata[index] = value;
            }
            self.nes.dma_state = DmaState::NoDma;
        }

        self.on_cpu_cycle();
        self.access(addr, AccessMode::Read)
    }

    fn poke(&mut self, addr: u16, val: u8) {
        self.on_cpu_cycle();
        self.access(addr, AccessMode::Write(val));
    }

    fn state(&self) -> &cpu::State {
        &self.nes.mos6502
    }

    fn state_mut(&mut self) -> &mut cpu::State {
        &mut self.nes.mos6502
    }
}

impl ppu::Context for Emulator {
    fn peek_vram(&mut self, addr: u16) -> u8 {
        self.vaccess(addr, AccessMode::Read)
    }

    fn poke_vram(&mut self, addr: u16, val: u8) {
        self.vaccess(addr, AccessMode::Write(val));
    }

    fn state(&self) -> &ppu::State {
        &self.nes.ppu
    }

    fn state_mut(&mut self) -> &mut ppu::State {
        &mut self.nes.ppu
    }

    fn generate_frame(&mut self) {
        self.nes.frame_generated = true;
    }

    fn trigger_nmi(&mut self) {
        self.nes.mos6502.nmi = true;
    }
}