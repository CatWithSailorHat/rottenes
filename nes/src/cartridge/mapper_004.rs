use crate::cartridge::{BankType, BankWindow, BaseMapper, Mapper};
use crate::cartridge::{ChrRom, NesHeader, PrgRom};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct State {
    inner: BaseMapper,
    bank_register: u8,
    prg_rom_bank_mode: bool,
    chr_a12_inversion: bool,
    second_last_prg_rom_bank: usize,
    irq_enable: bool,
    irq_counter: u8,
    irq_latch: u8,
    four_screen: bool,
}

impl State {
    pub fn new(header: &NesHeader, prg_rom: &PrgRom, chr_rom: &ChrRom) -> Self {
        let mut inner = BaseMapper::new();
         
        inner.initialize(prg_rom, chr_rom, 0x2000, 0x2000);
        
        inner.map_cpu_address(0x6000, BankType::PRG_RAM, 0, BankWindow::Size8k);

        inner.map_ppu_address(0x0000, BankType::CHR_MEM, 0, BankWindow::Size8k);
        
        let second_last_prg_rom_bank = if inner.bank_num(BankType::PRG_ROM, BankWindow::Size8k) > 2 {
            inner.bank_num(BankType::PRG_ROM, BankWindow::Size8k) - 2
        } else {
            0
        };
        let last_bank = inner.bank_num(BankType::PRG_ROM, BankWindow::Size8k) - 1;
        inner.map_cpu_address(0x8000, BankType::PRG_ROM, 0, BankWindow::Size8k);
        inner.map_cpu_address(0xA000, BankType::PRG_ROM, 0, BankWindow::Size8k);
        inner.map_cpu_address(0xC000, BankType::PRG_ROM, second_last_prg_rom_bank as u8, BankWindow::Size8k);
        inner.map_cpu_address(0xE000, BankType::PRG_ROM, last_bank as u8, BankWindow::Size8k);

        match (header.mirroring, header.four_screen_mode) {
            (_, true) => {
                inner.initialize_and_map_nametable_fourscreen();
            }
            (super::MirrorMode::Vertical, false) => {
                inner.initialize_and_map_nametable_vertical();
            }
            (super::MirrorMode::Horizontal, false) => {
                inner.initialize_and_map_nametable_horizontal();
            }
        };
        State { 
            inner, 
            bank_register: 0, 
            prg_rom_bank_mode: false, 
            chr_a12_inversion: false, 
            second_last_prg_rom_bank, 
            irq_enable: false,
            irq_counter: 0,
            irq_latch: 0, 
            four_screen: header.four_screen_mode,
        }
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
            0x8000..=0x9FFF => {
                if addr & 1 == 0 {
                    self.chr_a12_inversion = value & 0b1000_0000 != 0;
                    self.prg_rom_bank_mode = value & 0b0100_0000 != 0;
                    self.bank_register = value & 0b0000_0111;
                }
                else {
                    match (self.bank_register, self.chr_a12_inversion, self.prg_rom_bank_mode) {
                        (0, false, _) => {
                            self.inner.map_ppu_address(0x0000, BankType::CHR_MEM, value >> 1, BankWindow::Size2k)
                        }
                        (0, true, _) => {
                            self.inner.map_ppu_address(0x1000, BankType::CHR_MEM, value >> 1, BankWindow::Size2k)
                        }
                        (1, false, _) => {
                            self.inner.map_ppu_address(0x0800, BankType::CHR_MEM, value >> 1, BankWindow::Size2k)
                        }
                        (1, true, _) => {
                            self.inner.map_ppu_address(0x1800, BankType::CHR_MEM, value >> 1, BankWindow::Size2k)
                        }
                        (2, false, _) => {
                            self.inner.map_ppu_address(0x1000, BankType::CHR_MEM, value, BankWindow::Size1k)
                        }
                        (2, true, _) => {
                            self.inner.map_ppu_address(0x0000, BankType::CHR_MEM, value, BankWindow::Size1k)
                        }
                        (3, false, _) => {
                            self.inner.map_ppu_address(0x1400, BankType::CHR_MEM, value, BankWindow::Size1k)
                        }
                        (3, true, _) => {
                            self.inner.map_ppu_address(0x0400, BankType::CHR_MEM, value, BankWindow::Size1k)
                        }
                        (4, false, _) => {
                            self.inner.map_ppu_address(0x1800, BankType::CHR_MEM, value, BankWindow::Size1k)
                        }
                        (4, true, _) => {
                            self.inner.map_ppu_address(0x0800, BankType::CHR_MEM, value, BankWindow::Size1k)
                        }
                        (5, false, _) => {
                            self.inner.map_ppu_address(0x1C00, BankType::CHR_MEM, value, BankWindow::Size1k)
                        }
                        (5, true, _) => {
                            self.inner.map_ppu_address(0x0C00, BankType::CHR_MEM, value, BankWindow::Size1k)
                        }
                        (6, _, false) => {
                            self.inner.map_cpu_address(0x8000, BankType::PRG_ROM, value & 0b0011_1111, BankWindow::Size8k);
                            self.inner.map_cpu_address(0xC000, BankType::PRG_ROM, self.second_last_prg_rom_bank as u8, BankWindow::Size8k);
                        }
                        (6, _, true) => {
                            self.inner.map_cpu_address(0xC000, BankType::PRG_ROM, value & 0b0011_1111, BankWindow::Size8k);
                            self.inner.map_cpu_address(0x8000, BankType::PRG_ROM, self.second_last_prg_rom_bank as u8, BankWindow::Size8k);
                        }
                        (7, _, _) => {
                            self.inner.map_cpu_address(0xA000, BankType::PRG_ROM, value & 0b0011_1111, BankWindow::Size8k);
                        }
                        _ => { unreachable!() }
                    }
                }
            }
            0xA000..=0xBFFF => {
                if addr & 1 == 0 {
                    match (value & 1 != 0, self.four_screen) {
                        (_, true) => {
                            self.inner.initialize_and_map_nametable_fourscreen();
                        }
                        (false, false) => {
                            self.inner.initialize_and_map_nametable_vertical();
                        }
                        (true, false) => {
                            self.inner.initialize_and_map_nametable_horizontal();
                        }
                    };
                }
                else {
                    // not to implement `PRG RAM protect`
                }
            }
            0xC000..=0xDFFF => {
                if addr & 1 == 0 {
                    self.irq_latch = value;
                }
                else {
                    self.irq_counter = 0;
                }
            }
            0xE000..=0xFFFF => {
                if addr & 1 == 0 {
                    self.irq_enable = false;
                }
                else {
                    self.irq_enable = true;
                }
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

    fn irq(&mut self) -> bool { 
        if self.irq_counter == 0 {
            self.irq_counter = self.irq_latch;
            self.irq_enable
        }
        else {
            self.irq_counter -= 1;
            false
        }
    }
}
