use crate::error::LoadError;
use std::{io::{Read, prelude::*}};
use crate::bitmisc::U8BitTest;

#[derive(PartialEq, Eq)]
pub enum NesVersion {
    V1,
    V2,
}

#[derive(PartialEq, Eq)]
pub enum MirrorMode {
    H,
    V,
}

pub struct Rom {
    pub prg_rom: Vec<u8>,
    pub prg_banks: usize,
    pub chr_rom: Vec<u8>,
    pub chr_banks: usize,
    pub trainner: Vec<u8>,
    pub mirroring: MirrorMode,
    pub four_screen_mode: bool,
    pub has_battery: bool,
    pub nes_version: NesVersion,
    pub mapper_id: u16,
}

impl Rom {
    pub fn parse<R: Read + Seek>(stream: &mut R) -> Result<Self, LoadError> {
        let mut header = [0u8; 16];
        stream.read_exact(&mut header)?;
        for (b1, b2) in header.iter().zip("NES\x1A".bytes()) {
            if *b1 != b2 {
                return Err(LoadError::NotNesRom);
            }
        }
    
        let prg_banks = header[4] as usize;
        let chr_banks = header[5] as usize;

        let mirroring = if header[6].is_b0_set() { MirrorMode::V } else { MirrorMode::H };
        let has_battery = header[6].is_b1_set();
        let has_trainner = header[6].is_b2_set();
        let four_screen_mode = header[6].is_b3_set();

        let mapper_id_lo = (header[6] >> 4) & 0b1111;
        let mapper_id_hi = (header[7] >> 4) & 0b1111;
        let mapper_id = ((mapper_id_hi << 4) | (mapper_id_lo)) as u16;
        let nes_version = if (header[7] >> 2) | 0b11 == 0b10 {
            NesVersion::V2
        } else {
            NesVersion::V1
        };

        if nes_version == NesVersion::V2 {
            todo!("Nes 2.0 format support")
        }
        
        let mut trainner: Vec<u8> = Vec::new();
        if has_trainner {
            let mut trainner_buf = [0u8; 0x200];
            stream.read_exact(&mut trainner_buf)?;
            let mut buf = trainner_buf.to_vec();
            trainner.append(&mut buf);
        }
        
        let mut i: usize = 0;
        let mut prg_buf = [0u8; 0x4000];
        let mut prg_rom: Vec<u8> = Vec::new();
        while i < prg_banks {
            stream.read_exact(&mut prg_buf)?;
            let mut buf = prg_buf.to_vec();
            prg_rom.append(&mut buf);
            i += 1;
        }
        
        let mut i: usize = 0;
        let mut chr_rom: Vec<u8> = Vec::new();
        let mut chr_buf = [0u8; 0x2000];
        while i < chr_banks {
            stream.read_exact(&mut chr_buf)?;
            let mut buf = chr_buf.to_vec();
            chr_rom.append(&mut buf);
            i += 1;
        }
        
        Ok(Rom{
            prg_rom,
            chr_rom,
            trainner,
            mirroring,
            four_screen_mode,
            has_battery,
            nes_version,
            mapper_id,
            prg_banks,
            chr_banks,
        })
    }
}
