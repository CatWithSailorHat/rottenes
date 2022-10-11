use crate::cartridge::{BankType, BankWindow, BaseMapper, Mapper, MemAttr};
use crate::cartridge::{ChrRom, NesHeader, PrgRom};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct State {
    inner: BaseMapper,
}

impl State {
    pub fn new(header: &NesHeader, prg_rom: &PrgRom, chr_rom: &ChrRom) -> Self {
        let mut inner = BaseMapper::new();
         
        inner.initialize(prg_rom, chr_rom, 0, 0x2000);

        inner.map_ppu_address(0x0000, BankType::CHR_MEM, 0, BankWindow::Size8k);
        
        let last_bank = inner.bank_num(BankType::PRG_ROM, BankWindow::Size16k) - 1;
        inner.map_cpu_address(0x8000, BankType::PRG_ROM, 0, BankWindow::Size16k);
        inner.map_cpu_address(0xC000, BankType::PRG_ROM, last_bank as u8, BankWindow::Size16k);

        match header.mirroring {
            super::MirrorMode::Vertical => {
                inner.initialize_and_map_nametable_vertical();
            }
            super::MirrorMode::Horizontal => {
                inner.initialize_and_map_nametable_horizontal();
            }
        };
        State { inner }
    }
}

impl Mapper for State {
    fn peek(&mut self, addr: u16) -> u8 {
        self.inner.peek_cpu_memory(addr)
    }

    fn poke(&mut self, addr: u16, value: u8) {
        match addr {
            0x6000..=0x7FFF => {
                self.inner.poke_cpu_memory(addr, value)
            }
            0x8000..=0xFFFF => {
                let selector = value & 0b0000_0111;
                self.inner.map_cpu_address(0x8000, BankType::PRG_ROM, selector, BankWindow::Size16k);
            }
            _ => unreachable!("CPU ADDRESS: 0x{:X}", addr)
        }
    }

    fn vpeek(&mut self, addr: u16) -> u8 {
        self.inner.peek_ppu_memory(addr)
    }

    fn vpoke(&mut self, addr: u16, value: u8) {
        self.inner.poke_ppu_memory(addr, value)
    }

    fn load_state(&mut self, state: Vec<u8>) {
        let state: Self = bincode::deserialize(&state[..]).unwrap();
        *self = state;
    }

    fn save_state(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}
