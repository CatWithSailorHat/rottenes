use serde::{Deserialize, Serialize};

const CPU_ADDRESS_SPACE_MAPPED_BEGIN: u16 = 0x6000;
const CPU_ADDRESS_SPACE_MAPPED_END: u16 = 0xFFFF;
const PPU_ADDRESS_SPACE_MAPPED_BEGIN: u16 = 0x0000;
const PPU_ADDRESS_SPACE_MAPPED_END: u16 = 0x2FFF;

const CPU_MINIMUM_MAP_SIZE: usize = 0x2000; // 8K
const PPU_MINIMUM_MAP_SIZE: usize = 0x0400; // 1K
const CPU_MAP_TABLE_SIZE: usize =
    (CPU_ADDRESS_SPACE_MAPPED_END - CPU_ADDRESS_SPACE_MAPPED_BEGIN) as usize / CPU_MINIMUM_MAP_SIZE
        + 1;
const PPU_MAP_TABLE_SIZE: usize =
    (PPU_ADDRESS_SPACE_MAPPED_END - PPU_ADDRESS_SPACE_MAPPED_BEGIN) as usize / PPU_MINIMUM_MAP_SIZE
        + 1;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct MapTableItem {
    offset: usize,
    attribute: MemAttr,
    bank_type: Option<BankType>,
}

impl Default for MapTableItem {
    fn default() -> Self {
        MapTableItem {
            offset: 0,
            attribute: MemAttr::ReadOnly,
            bank_type: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum MemAttr {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum BankType {
    PRG_ROM,
    PRG_RAM,
    CHR_MEM,
    NAMETABLE,
}

#[derive(Clone, Copy)]
pub enum BankWindow {
    Size32k = 0x8000,
    Size16k = 0x4000,
    Size8k = 0x2000,
    Size4k = 0x1000,
    Size2k = 0x0800,
    Size1k = 0x0400,
}

#[derive(Serialize, Deserialize)]
pub struct BaseMapper {
    cpu_map_table: [MapTableItem; CPU_MAP_TABLE_SIZE],
    ppu_map_table: [MapTableItem; PPU_MAP_TABLE_SIZE],
    is_chr_rom_provided: bool,

    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_mem: Vec<u8>,
    nametable: Vec<u8>,
}

impl BaseMapper {
    fn cpu_map_table_idx(addr: u16) -> usize {
        (addr - CPU_ADDRESS_SPACE_MAPPED_BEGIN) as usize / CPU_MINIMUM_MAP_SIZE
    }

    fn ppu_map_table_idx(addr: u16) -> usize {
        (addr - PPU_ADDRESS_SPACE_MAPPED_BEGIN) as usize / PPU_MINIMUM_MAP_SIZE
    }

    fn default_mem_attr(&self, bank_type: BankType) -> MemAttr {
        match bank_type {
            BankType::PRG_ROM => MemAttr::ReadOnly,
            BankType::PRG_RAM => MemAttr::ReadWrite,
            BankType::CHR_MEM => {
                if self.is_chr_rom_provided {
                    MemAttr::ReadOnly
                } else {
                    MemAttr::ReadWrite
                }
            }
            BankType::NAMETABLE => MemAttr::ReadWrite,
        }
    }

    pub fn new() -> Self {
        BaseMapper {
            cpu_map_table: [MapTableItem::default(); CPU_MAP_TABLE_SIZE],
            ppu_map_table: [MapTableItem::default(); PPU_MAP_TABLE_SIZE],
            is_chr_rom_provided: false,
            prg_rom: Vec::new(),
            prg_ram: Vec::new(),
            chr_mem: Vec::new(),
            nametable: Vec::new(),
        }
    }

    pub fn initialize(
        &mut self,
        prg_rom: &Vec<u8>,
        chr_rom: &Vec<u8>,
        prg_ram_capacity: usize,
        chr_capacity: usize,
    ) {
        self.prg_rom = prg_rom.clone();
        self.prg_ram.resize(prg_ram_capacity, 0);
        if chr_rom.len() == 0 {
            self.chr_mem.resize(chr_capacity, 0);
        } else {
            self.chr_mem = chr_rom.clone();
        }
    }

    // pub fn initialize_prg_rom(&mut self, prg_rom: &Vec<u8>) {
    //     self.prg_rom = prg_rom.clone()
    // }

    // pub fn initialize_prg_ram(&mut self, size: usize) {
    //     self.prg_ram.resize(size, 0);
    // }

    // pub fn initialize_and_map_chr(&mut self, chr_rom: &Vec<u8>, chr_capacity: usize) {
    //     if chr_rom.len() == 0 {
    //         self.chr_mem.resize(chr_capacity, 0);
    //     } else {
    //         self.chr_mem = chr_rom.clone();
    //     }
    //     self.map_ppu_address(0x0000, BankType::CHR_MEM, 0, BankWindow::Size8k);
    // }

    // pub fn initialize_nametable(&mut self, size: usize) {
    //     self.nametable.resize(size, 0);
    // }

    pub fn map_cpu_address(
        &mut self,
        addr: u16,
        bank_type: BankType,
        bank_selector: u8,
        bank_window: BankWindow,
    ) {
        self.map_cpu_address_with_attr(
            addr,
            bank_type,
            bank_selector,
            bank_window,
            self.default_mem_attr(bank_type),
        )
    }

    pub fn map_ppu_address(
        &mut self,
        addr: u16,
        bank_type: BankType,
        bank_selector: u8,
        bank_window: BankWindow,
    ) {
        self.map_ppu_address_with_attr(
            addr,
            bank_type,
            bank_selector,
            bank_window,
            self.default_mem_attr(bank_type),
        )
    }

    pub fn map_cpu_address_with_attr(
        &mut self,
        addr: u16,
        bank_type: BankType,
        bank_selector: u8,
        bank_window: BankWindow,
        mem_attr: MemAttr,
    ) {
        let bank_selector = bank_selector % ((self.prg_rom.len() / (bank_window as usize)) as u8);
        let addr = addr & (bank_window as u16 - 1).reverse_bits();
        let bank_window = bank_window as usize;
        let offset = bank_window * bank_selector as usize;
        let idx_base = Self::cpu_map_table_idx(addr);
        for i in 0..bank_window / CPU_MINIMUM_MAP_SIZE {
            let idx = idx_base + i;
            match bank_type {
                BankType::PRG_ROM | BankType::PRG_RAM => {
                    self.cpu_map_table[idx].offset = offset + i * CPU_MINIMUM_MAP_SIZE;
                    self.cpu_map_table[idx].bank_type = Some(bank_type);
                    self.cpu_map_table[idx].attribute = mem_attr;
                }
                _ => {
                    panic!("Cannot map CHR memory to cpu addr space")
                }
            }
        }
    }

    pub fn unmap_cpu_address(&mut self, addr: u16, bank_window: BankWindow) {
        let addr = addr & (bank_window as u16 - 1).reverse_bits();
        let bank_window = bank_window as usize;
        let idx_base = Self::cpu_map_table_idx(addr);
        for i in 0..bank_window / CPU_MINIMUM_MAP_SIZE {
            let idx = idx_base + i;
            self.cpu_map_table[idx].offset = 0;
            self.cpu_map_table[idx].bank_type = None;
            self.cpu_map_table[idx].attribute = MemAttr::ReadOnly;
        }
    }

    pub fn map_ppu_address_with_attr(
        &mut self,
        addr: u16,
        bank_type: BankType,
        bank_selector: u8,
        bank_window: BankWindow,
        mem_attr: MemAttr,
    ) {
        let round = self.chr_mem.len() / (bank_window as usize);
        let bank_selector = if round == 0 {
            bank_selector
        }
        else {
            (bank_selector as usize % round) as u8
        };
        let addr = addr & (bank_window as u16 - 1).reverse_bits();
        let bank_window = bank_window as usize;
        let offset = bank_window * bank_selector as usize;
        let idx_base = Self::ppu_map_table_idx(addr);
        for i in 0..bank_window / PPU_MINIMUM_MAP_SIZE {
            let idx = idx_base + i;
            match bank_type {
                BankType::CHR_MEM | BankType::CHR_MEM | BankType::NAMETABLE => {
                    self.ppu_map_table[idx].offset = offset + i * PPU_MINIMUM_MAP_SIZE;
                    self.ppu_map_table[idx].bank_type = Some(bank_type);
                    self.ppu_map_table[idx].attribute = mem_attr;
                }
                _ => {
                    panic!("Cannot map PRG memory to ppu addr space")
                }
            }
        }
    }

    pub fn unmap_ppu_address(&mut self, addr: u16, bank_window: BankWindow) {
        let addr = addr & (bank_window as u16 - 1).reverse_bits();
        let bank_window = bank_window as usize;
        let idx_base = Self::ppu_map_table_idx(addr);
        for i in 0..bank_window / PPU_MINIMUM_MAP_SIZE {
            let idx = idx_base + i;
            self.ppu_map_table[idx].offset = 0;
            self.ppu_map_table[idx].bank_type = None;
            self.ppu_map_table[idx].attribute = MemAttr::ReadOnly;
        }
    }

    pub fn peek_cpu_memory(&self, addr: u16) -> u8 {
        let item = self.cpu_map_table[Self::cpu_map_table_idx(addr)];
        let offset = (addr as usize & (CPU_MINIMUM_MAP_SIZE - 1)) + item.offset;
        if let Some(bank_type) = item.bank_type {
            self.internal_peek(bank_type, item.attribute, offset)
        } else {
            panic!("Peek unmapped cpu memory: 0x{:x}", addr)
        }
    }

    pub fn poke_cpu_memory(&mut self, addr: u16, value: u8) {
        let item = self.cpu_map_table[Self::cpu_map_table_idx(addr)];
        let offset = (addr as usize & (CPU_MINIMUM_MAP_SIZE - 1)) + item.offset;
        if let Some(bank_type) = item.bank_type {
            self.internal_poke(bank_type, item.attribute, offset, value);
        } else {
            panic!("Poke unmapped cpu memory: 0x{:x}", addr)
        }
    }

    pub fn peek_ppu_memory(&self, addr: u16) -> u8 {
        let item = self.ppu_map_table[Self::ppu_map_table_idx(addr)];
        let offset = (addr as usize & (PPU_MINIMUM_MAP_SIZE - 1)) + item.offset;
        if let Some(bank_type) = item.bank_type {
            self.internal_peek(bank_type, item.attribute, offset)
        } else {
            panic!("Peek unmapped ppu memory: 0x{:x}", addr)
        }
    }

    pub fn poke_ppu_memory(&mut self, addr: u16, value: u8) {
        let item = self.ppu_map_table[Self::ppu_map_table_idx(addr)];
        let offset = (addr as usize & (PPU_MINIMUM_MAP_SIZE - 1)) + item.offset;
        if let Some(bank_type) = item.bank_type {
            self.internal_poke(bank_type, item.attribute, offset, value);
        } else {
            panic!("Poke unmapped ppu memory: 0x{:x}", addr)
        }
    }

    pub fn initialize_and_map_nametable_vertical(&mut self) {
        self.nametable.resize(0x800, 0);
        self.map_ppu_address(0x2000, BankType::NAMETABLE, 0, BankWindow::Size1k);
        self.map_ppu_address(0x2400, BankType::NAMETABLE, 1, BankWindow::Size1k);
        self.map_ppu_address(0x2800, BankType::NAMETABLE, 0, BankWindow::Size1k);
        self.map_ppu_address(0x2C00, BankType::NAMETABLE, 1, BankWindow::Size1k);
    }

    pub fn initialize_and_map_nametable_horizontal(&mut self) {
        self.nametable.resize(0x800, 0);
        self.map_ppu_address(0x2000, BankType::NAMETABLE, 0, BankWindow::Size1k);
        self.map_ppu_address(0x2400, BankType::NAMETABLE, 0, BankWindow::Size1k);
        self.map_ppu_address(0x2800, BankType::NAMETABLE, 1, BankWindow::Size1k);
        self.map_ppu_address(0x2C00, BankType::NAMETABLE, 1, BankWindow::Size1k);
    }

    pub fn initialize_and_map_nametable_fourscreen(&mut self) {
        self.nametable.resize(0x2000, 0);
        self.map_ppu_address(0x2000, BankType::NAMETABLE, 0, BankWindow::Size1k);
        self.map_ppu_address(0x2400, BankType::NAMETABLE, 1, BankWindow::Size1k);
        self.map_ppu_address(0x2800, BankType::NAMETABLE, 2, BankWindow::Size1k);
        self.map_ppu_address(0x2C00, BankType::NAMETABLE, 3, BankWindow::Size1k);
    }

    pub fn initialize_and_map_nametable_onescreen_lower_bank(&mut self) {
        self.nametable.resize(0x800, 0);
        self.map_ppu_address(0x2000, BankType::NAMETABLE, 0, BankWindow::Size1k);
        self.map_ppu_address(0x2400, BankType::NAMETABLE, 0, BankWindow::Size1k);
        self.map_ppu_address(0x2800, BankType::NAMETABLE, 0, BankWindow::Size1k);
        self.map_ppu_address(0x2C00, BankType::NAMETABLE, 0, BankWindow::Size1k);
    }

    pub fn initialize_and_map_nametable_onescreen_upper_bank(&mut self) {
        self.nametable.resize(0x800, 0);
        self.map_ppu_address(0x2000, BankType::NAMETABLE, 1, BankWindow::Size1k);
        self.map_ppu_address(0x2400, BankType::NAMETABLE, 1, BankWindow::Size1k);
        self.map_ppu_address(0x2800, BankType::NAMETABLE, 1, BankWindow::Size1k);
        self.map_ppu_address(0x2C00, BankType::NAMETABLE, 1, BankWindow::Size1k);
    }

    pub fn bank_num(&self, bank_type: BankType, bank_window: BankWindow) -> usize {
        match bank_type {
            BankType::PRG_ROM => self.prg_rom.len() / bank_window as usize,
            BankType::PRG_RAM => self.prg_ram.len() / bank_window as usize,
            BankType::CHR_MEM => self.chr_mem.len() / bank_window as usize,
            BankType::NAMETABLE => self.nametable.len() / bank_window as usize,
        }
    }

    #[inline]
    fn internal_peek(&self, bank_type: BankType, attribute: MemAttr, offset: usize) -> u8 {
        match (bank_type, attribute) {
            (_, MemAttr::WriteOnly) => 0, // TODO: implement openbus
            (BankType::PRG_ROM, _) => self.prg_rom[offset],
            (BankType::PRG_RAM, _) => self.prg_ram[offset],
            (BankType::CHR_MEM, _) => self.chr_mem[offset],
            (BankType::NAMETABLE, _) => self.nametable[offset],
        }
    }

    #[inline]
    fn internal_poke(&mut self, bank_type: BankType, attribute: MemAttr, offset: usize, value: u8) {
        match (bank_type, attribute) {
            (_, MemAttr::ReadOnly) => (), // TODO: implement openbus
            (BankType::PRG_ROM, _) => self.prg_rom[offset] = value,
            (BankType::PRG_RAM, _) => self.prg_ram[offset] = value,
            (BankType::CHR_MEM, _) => self.chr_mem[offset] = value,
            (BankType::NAMETABLE, _) => self.nametable[offset] = value,
        }
    }
}

pub trait Mapper {
    fn peek_expansion_rom(&mut self, addr: u16) -> u8 {
        println!("PEEK EXPANSION ROM: 0x{:x}", addr);
        0
    }
    fn poke_expansion_rom(&mut self, addr: u16, val: u8) {
        println!("POKE EXPANSION ROM: 0x{:x}, VALUE: 0x{:x}", addr, val);
    }

    fn peek(&mut self, addr: u16) -> u8;
    fn poke(&mut self, addr: u16, val: u8);

    fn vpeek(&mut self, addr: u16) -> u8;
    fn vpoke(&mut self, addr: u16, val: u8);

    fn irq(&mut self) -> bool { false }
    fn irq_acknowledge(&mut self) -> bool { false }

    fn load_state(&mut self, state: Vec<u8>);
    fn save_state(&self) -> Vec<u8>;
}
