use super::Mapper;
use crate::rom::{MirrorMode, Rom};

pub struct State {
    rom: Rom,
    mirror_prg: bool,
    prg_ram: [u8; 0x2000],
    vram: [u8; 0x800],
}

impl State {
    pub fn new(rom: Rom) -> State {
        assert!(rom.prg_banks <= 2);
        let mirror_rom = rom.prg_banks == 1;
        State {
            rom,
            mirror_prg: mirror_rom,
            prg_ram: [0; 0x2000],
            vram: [0; 0x800],
        }
    }
}

impl Mapper for State {
    fn peek_expansion_rom(&mut self, addr: u16) -> u8 {
        unimplemented!()
    }

    fn poke_expansion_rom(&mut self, addr: u16, val: u8) {
        unimplemented!()
    }

    fn peek_sram(&mut self, addr: u16) -> u8 {
        self.prg_ram[(addr & 0x1FFF) as usize]
    }

    fn poke_sram(&mut self, addr: u16, val: u8) {
        self.prg_ram[(addr & 0x1FFF) as usize] = val;
    }

    fn peek_prg_rom(&mut self, addr: u16) -> u8 {
        let addr = if self.mirror_prg { addr & 0xBFFF } else { addr };
        self.rom.prg_rom[(addr & 0x7FFF) as usize]
    }

    fn poke_prg_rom(&mut self, addr: u16, val: u8) {
        let addr = if self.mirror_prg { addr & 0xBFFF } else { addr };
        self.rom.prg_rom[(addr & 0x7FFF) as usize] = val;
    }

    fn vpeek_nametable(&mut self, addr: u16) -> u8 {
        let index = if self.rom.mirroring == MirrorMode::H {
            let t = addr & 0xBFF;
            if t > 0x7FF { (t & 0x7FF) + 0x400 } else { t }
        } else {
            addr & 0x7FF
        } as usize;
        self.vram[index]
    }

    fn vpoke_nametable(&mut self, addr: u16, val: u8) {
        let index = if self.rom.mirroring == MirrorMode::H {
            let t = addr & 0xBFF;
            if t > 0x7FF { (t & 0x7FF) + 0x400 } else { t }
        } else {
            addr & 0x7FF
        } as usize;
        self.vram[index] = val;
    }

    fn vpeek_pattern(&mut self, addr: u16) -> u8 {
        self.rom.chr_rom[(addr & 0x1FFF) as usize]
    }

    fn vpoke_pattern(&mut self, addr: u16, val: u8) {
        self.rom.chr_rom[(addr & 0x1FFF) as usize] = val;
    }
}