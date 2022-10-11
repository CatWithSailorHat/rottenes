use crate::cartridge::{BankType, BankWindow, BaseMapper, Mapper, MemAttr};
use crate::cartridge::{ChrRom, NesHeader, PrgRom};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum PrgRomBankSwitchMode {
    Switch32k,
    FixFirstBank,
    FixLastBank,
}

#[derive(Debug, Serialize, Deserialize)]
enum ChrRomBankSwitchMode {
    Switch8k,
    Switch4k,
}

#[derive(Serialize, Deserialize)]
pub struct State {
    inner: BaseMapper,
    shifter: u8,
    prg_rom_bank_mode: PrgRomBankSwitchMode,
    chr_rom_bank_mode: ChrRomBankSwitchMode,
    prg_rom_16k_selector: u8,
    chr_4k_lower_selector: u8,
    chr_4k_upper_selector: u8,
}

impl State {
    pub fn new(header: &NesHeader, prg_rom: &PrgRom, chr_rom: &ChrRom) -> Self {
        let mut inner = BaseMapper::new();
        inner.initialize(prg_rom, chr_rom, 0x8000, 0x20000);

        inner.map_cpu_address(0x6000, BankType::PRG_RAM, 0, BankWindow::Size8k);
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

        let shifter =  0b0001_0000u8;
        let prg_rom_bank_mode = PrgRomBankSwitchMode::FixLastBank;
        let chr_rom_bank_mode = ChrRomBankSwitchMode::Switch4k;

        State { 
            inner, 
            shifter, 
            prg_rom_bank_mode, 
            chr_rom_bank_mode, 
            prg_rom_16k_selector: 0, 
            chr_4k_lower_selector: 0, 
            chr_4k_upper_selector: 1, 
        }
    }

    fn update_map_state(&mut self) {
        match self.prg_rom_bank_mode {
            PrgRomBankSwitchMode::Switch32k => {
                self.inner.map_cpu_address(0x8000, BankType::PRG_ROM, self.prg_rom_16k_selector + 0, BankWindow::Size16k);
                self.inner.map_cpu_address(0xC000, BankType::PRG_ROM, self.prg_rom_16k_selector + 1, BankWindow::Size16k);
            },
            PrgRomBankSwitchMode::FixFirstBank => {
                self.inner.map_cpu_address(0x8000, BankType::PRG_ROM, 0, BankWindow::Size16k);
                self.inner.map_cpu_address(0xC000, BankType::PRG_ROM, self.prg_rom_16k_selector, BankWindow::Size16k)
            },
            PrgRomBankSwitchMode::FixLastBank => {
                let last_bank = self.inner.bank_num(BankType::PRG_ROM, BankWindow::Size16k) - 1;
                self.inner.map_cpu_address(0x8000, BankType::PRG_ROM, self.prg_rom_16k_selector, BankWindow::Size16k);
                self.inner.map_cpu_address(0xC000, BankType::PRG_ROM, last_bank as u8, BankWindow::Size16k);
            },
        }

        match self.chr_rom_bank_mode {
            ChrRomBankSwitchMode::Switch8k => {
                self.inner.map_ppu_address(0x0000, BankType::CHR_MEM, self.chr_4k_lower_selector + 0, BankWindow::Size4k);
                self.inner.map_ppu_address(0x1000, BankType::CHR_MEM, self.chr_4k_lower_selector + 1, BankWindow::Size4k);
            },
            ChrRomBankSwitchMode::Switch4k => {
                self.inner.map_ppu_address(0x0000, BankType::CHR_MEM, self.chr_4k_lower_selector, BankWindow::Size4k);
                self.inner.map_ppu_address(0x1000, BankType::CHR_MEM, self.chr_4k_upper_selector, BankWindow::Size4k);
            },
        }
    }
}

impl Mapper for State {
    fn peek(&mut self, addr: u16) -> u8 {
        self.inner.peek_cpu_memory(addr)
    }

    fn poke(&mut self, addr: u16, value: u8) {
        match addr {
            0x6000..=0x7FFF => self.inner.poke_cpu_memory(addr, value),
            0x8000..=0xFFFF => {
                let shifter_full = self.shifter & 1 != 0;
                let reset = value & 0b1000_0000 != 0;

                self.shifter |= (value & 1) << 5;
                self.shifter >>= 1;
                if shifter_full {
                    let value = self.shifter;
                    match addr {
                        0x8000..=0x9FFF => {
                            match value & 0b11 {
                                0 => self.inner.initialize_and_map_nametable_onescreen_lower_bank(),
                                1 => self.inner.initialize_and_map_nametable_onescreen_upper_bank(),
                                2 => self.inner.initialize_and_map_nametable_vertical(),
                                3 => self.inner.initialize_and_map_nametable_horizontal(),
                                _ => unreachable!(),
                            }
                            self.prg_rom_bank_mode = match (value >> 2) & 0b11 {
                                0 | 1 => PrgRomBankSwitchMode::Switch32k,
                                2 => PrgRomBankSwitchMode::FixFirstBank,
                                3 => PrgRomBankSwitchMode::FixLastBank,
                                _ => unreachable!(),
                            };
                            self.chr_rom_bank_mode = match (value >> 4) & 0b1 {
                                0 => ChrRomBankSwitchMode::Switch8k,
                                1 => ChrRomBankSwitchMode::Switch4k,
                                _ => unreachable!(),
                            };
                            self.update_map_state();
                        }
                        0xA000..=0xBFFF => {
                            match self.chr_rom_bank_mode {
                                ChrRomBankSwitchMode::Switch8k => self.chr_4k_lower_selector = value & 0b11110,
                                ChrRomBankSwitchMode::Switch4k => self.chr_4k_lower_selector = value & 0b11111,
                            }
                        }
                        0xC000..=0xDFFF => {
                            match self.chr_rom_bank_mode {
                                ChrRomBankSwitchMode::Switch8k => {},
                                ChrRomBankSwitchMode::Switch4k => self.chr_4k_upper_selector = value & 0b11111,
                            }
                        }
                        0xE000..=0xFFFF => {
                            match self.prg_rom_bank_mode {
                                PrgRomBankSwitchMode::Switch32k => {
                                    self.prg_rom_16k_selector = value & 0b1110;
                                },
                                PrgRomBankSwitchMode::FixFirstBank => {
                                    self.prg_rom_16k_selector = value & 0b1111;
                                },
                                PrgRomBankSwitchMode::FixLastBank => {
                                    self.prg_rom_16k_selector = value & 0b1111;
                                },
                            }
                            
                        }
                        _ => unreachable!("CPU ADDRESS: 0x{:X}", addr)
                    }
                    self.shifter = 0b0001_0000;
                } else if reset {
                    self.shifter = 0b0001_0000;
                    self.prg_rom_bank_mode = PrgRomBankSwitchMode::FixLastBank;
                }
                self.update_map_state();
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
