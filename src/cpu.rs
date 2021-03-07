// #![allow(dead_code)]

use std::{num::Wrapping};
use crate::bitmisc::U16Address;

bitflags! {
    pub struct Flags: u8 {
        /// carry flag
        const C = 1 << 0;
        /// zero flag
        const Z = 1 << 1;
        /// interrupt disable
        const I = 1 << 2;
        /// decimal mode
        const D = 1 << 3;
        /// break command
        const B = 1 << 4;
        /// -UNUSED-
        const U = 1 << 5;
        /// overflow flag
        const V = 1 << 6;
        /// negative flag
        const N = 1 << 7;
    }
}

const INT_NMI_ADDRESS: u16 = 0xFFFA;
const INT_IRQ_BRK_ADDRESS: u16 = 0xFFFE;
const INT_RESET_ADDRESS: u16 = 0xFFFC;

#[allow(non_snake_case)]
pub struct Registers {
    pub A: u8, pub X: u8, pub Y: u8, pub SP: u8, pub PC: u16, pub P: Flags,
}

impl Registers {
    fn new() -> Self {
        Registers { A: 0, X: 0, Y: 0, SP: 0xFD, PC: 0, P: (Flags::I | Flags::U) }
    }

    fn set_nz(&mut self, x: u8) -> u8 {
        self.P.set(Flags::N, x & 0x80 != 0);
        self.P.set(Flags::Z, x == 0);
        x
    }

    fn set_cv(&mut self, x: u8, y: u8, r: u16) {
        self.P.set(Flags::C, r > 0xff);
        self.P.set(Flags::V, (x ^ y) & 0x80 == 0 && (x as u16 ^ r) & 0x80 == 0x80);
    }

    fn get_c_as_u8(&self) -> u8 {
        if self.P.contains(Flags::C) { 1 } else { 0 }
    }
}

pub struct State {
    pub regs: Registers,
    pub nmi: bool,
    pub irq: bool,
}

impl State {
    pub fn new() -> Self {
        State {
            regs: Registers::new(),
            nmi: false,
            irq: false,
        }
    }
}

pub trait Context: Sized {
    fn peek(&mut self, addr: u16) -> u8;
    fn poke(&mut self, addr: u16, val: u8);
    fn state(&self) -> &State;
    fn state_mut(&mut self) -> &mut State;
}

pub trait Interface: Sized + Context {
    fn reset(&mut self) {
        Private::reset(self);
    }

    fn step(&mut self) {
        if self.state().nmi {
            self.hardware_interrupt();
            self.state_mut().nmi = false;
        } else if self.state().irq && self.state().regs.P.contains(Flags::I) {
            self.hardware_interrupt();
            self.state_mut().irq = false;
        }
        else {
            Private::execute_one_instruction(self);
        }
    }
}

impl<T: Context> Interface for T {}
impl<T: Context> Private for T {}
trait Private: Sized + Context {
    #[inline]
    fn execute_one_instruction(&mut self) {
        type A = AddressingMode;
        type I = Instruction;
        let opcode: u8 = self.fetch_and_inc_pc();
        let (insturction, mode): (Instruction, AddressingMode) = match opcode {
            0x00=>(I::BRK, A::IMP),   0x01=>(I::ORA, A::IZX),   0x02=>(I::KIL, A::IMP),   0x03=>(I::SLO, A::IZX),
            0x04=>(I::NOP, A::ZPG),   0x05=>(I::ORA, A::ZPG),   0x06=>(I::ASL, A::ZPG),   0x07=>(I::SLO, A::ZPG),
            0x08=>(I::PHP, A::IMP),   0x09=>(I::ORA, A::IMM),   0x0A=>(I::ASL, A::ACC),   0x0B=>(I::ANC, A::IMM),
            0x0C=>(I::NOP, A::ABS),   0x0D=>(I::ORA, A::ABS),   0x0E=>(I::ASL, A::ABS),   0x0F=>(I::SLO, A::ABS),
            0x10=>(I::BPL, A::REL),   0x11=>(I::ORA, A::IZY),   0x12=>(I::KIL, A::IMP),   0x13=>(I::SLO, A::IZY),
            0x14=>(I::NOP, A::ZPX),   0x15=>(I::ORA, A::ZPX),   0x16=>(I::ASL, A::ZPX),   0x17=>(I::SLO, A::ZPX),
            0x18=>(I::CLC, A::IMP),   0x19=>(I::ORA, A::ABY),   0x1A=>(I::NOP, A::IMP),   0x1B=>(I::SLO, A::ABY),
            0x1C=>(I::NOP, A::ABX),   0x1D=>(I::ORA, A::ABX),   0x1E=>(I::ASL, A::ABX),   0x1F=>(I::SLO, A::ABX),

            0x20=>(I::JSR, A::ABS),   0x21=>(I::AND, A::IZX),   0x22=>(I::KIL, A::IMP),   0x23=>(I::RLA, A::IZX),
            0x24=>(I::BIT, A::ZPG),   0x25=>(I::AND, A::ZPG),   0x26=>(I::ROL, A::ZPG),   0x27=>(I::RLA, A::ZPG),
            0x28=>(I::PLP, A::IMP),   0x29=>(I::AND, A::IMM),   0x2A=>(I::ROL, A::ACC),   0x2B=>(I::ANC, A::IMM),
            0x2C=>(I::BIT, A::ABS),   0x2D=>(I::AND, A::ABS),   0x2E=>(I::ROL, A::ABS),   0x2F=>(I::RLA, A::ABS),
            0x30=>(I::BMI, A::REL),   0x31=>(I::AND, A::IZY),   0x32=>(I::KIL, A::IMP),   0x33=>(I::RLA, A::IZY),
            0x34=>(I::NOP, A::ZPX),   0x35=>(I::AND, A::ZPX),   0x36=>(I::ROL, A::ZPX),   0x37=>(I::RLA, A::ZPX),
            0x38=>(I::SEC, A::IMP),   0x39=>(I::AND, A::ABY),   0x3A=>(I::NOP, A::IMP),   0x3B=>(I::RLA, A::ABY),
            0x3C=>(I::NOP, A::ABX),   0x3D=>(I::AND, A::ABX),   0x3E=>(I::ROL, A::ABX),   0x3F=>(I::RLA, A::ABX),

            0x40=>(I::RTI, A::IMP),   0x41=>(I::EOR, A::IZX),   0x42=>(I::KIL, A::IMP),   0x43=>(I::SRE, A::IZX),
            0x44=>(I::NOP, A::ZPG),   0x45=>(I::EOR, A::ZPG),   0x46=>(I::LSR, A::ZPG),   0x47=>(I::SRE, A::ZPG),
            0x48=>(I::PHA, A::IMP),   0x49=>(I::EOR, A::IMM),   0x4A=>(I::LSR, A::ACC),   0x4B=>(I::ALR, A::IMM),
            0x4C=>(I::JMP, A::ABS),   0x4D=>(I::EOR, A::ABS),   0x4E=>(I::LSR, A::ABS),   0x4F=>(I::SRE, A::ABS),
            0x50=>(I::BVC, A::REL),   0x51=>(I::EOR, A::IZY),   0x52=>(I::KIL, A::IMP),   0x53=>(I::SRE, A::IZY),
            0x54=>(I::NOP, A::ZPX),   0x55=>(I::EOR, A::ZPX),   0x56=>(I::LSR, A::ZPX),   0x57=>(I::SRE, A::ZPX),
            0x58=>(I::CLI, A::IMP),   0x59=>(I::EOR, A::ABY),   0x5A=>(I::NOP, A::IMP),   0x5B=>(I::SRE, A::ABY),
            0x5C=>(I::NOP, A::ABX),   0x5D=>(I::EOR, A::ABX),   0x5E=>(I::LSR, A::ABX),   0x5F=>(I::SRE, A::ABX),

            0x60=>(I::RTS, A::IMP),   0x61=>(I::ADC, A::IZX),   0x62=>(I::KIL, A::IMP),   0x63=>(I::RRA, A::IZX),
            0x64=>(I::NOP, A::ZPG),   0x65=>(I::ADC, A::ZPG),   0x66=>(I::ROR, A::ZPG),   0x67=>(I::RRA, A::ZPG),
            0x68=>(I::PLA, A::IMP),   0x69=>(I::ADC, A::IMM),   0x6A=>(I::ROR, A::ACC),   0x6B=>(I::ARR, A::IMM),
            0x6C=>(I::JMP, A::IND),   0x6D=>(I::ADC, A::ABS),   0x6E=>(I::ROR, A::ABS),   0x6F=>(I::RRA, A::ABS),
            0x70=>(I::BVS, A::REL),   0x71=>(I::ADC, A::IZY),   0x72=>(I::KIL, A::IMP),   0x73=>(I::RRA, A::IZY),
            0x74=>(I::NOP, A::ZPX),   0x75=>(I::ADC, A::ZPX),   0x76=>(I::ROR, A::ZPX),   0x77=>(I::RRA, A::ZPX),
            0x78=>(I::SEI, A::IMP),   0x79=>(I::ADC, A::ABY),   0x7A=>(I::NOP, A::IMP),   0x7B=>(I::RRA, A::ABY),
            0x7C=>(I::NOP, A::ABX),   0x7D=>(I::ADC, A::ABX),   0x7E=>(I::ROR, A::ABX),   0x7F=>(I::RRA, A::ABX),

            0x80=>(I::NOP, A::IMM),   0x81=>(I::STA, A::IZX),   0x82=>(I::NOP, A::IMM),   0x83=>(I::SAX, A::IZX),
            0x84=>(I::STY, A::ZPG),   0x85=>(I::STA, A::ZPG),   0x86=>(I::STX, A::ZPG),   0x87=>(I::SAX, A::ZPG),
            0x88=>(I::DEY, A::IMP),   0x89=>(I::NOP, A::IMM),   0x8A=>(I::TXA, A::IMP),   0x8B=>(I::XAA, A::IMM),
            0x8C=>(I::STY, A::ABS),   0x8D=>(I::STA, A::ABS),   0x8E=>(I::STX, A::ABS),   0x8F=>(I::SAX, A::ABS),
            0x90=>(I::BCC, A::REL),   0x91=>(I::STA, A::IZY),   0x92=>(I::KIL, A::IMP),   0x93=>(I::AHX, A::IZY),
            0x94=>(I::STY, A::ZPX),   0x95=>(I::STA, A::ZPX),   0x96=>(I::STX, A::ZPY),   0x97=>(I::SAX, A::ZPY),
            0x98=>(I::TYA, A::IMP),   0x99=>(I::STA, A::ABY),   0x9A=>(I::TXS, A::IMP),   0x9B=>(I::TAS, A::ABY),
            0x9C=>(I::SHY, A::ABX),   0x9D=>(I::STA, A::ABX),   0x9E=>(I::SHX, A::ABY),   0x9F=>(I::AHX, A::ABY),

            0xA0=>(I::LDY, A::IMM),   0xA1=>(I::LDA, A::IZX),   0xA2=>(I::LDX, A::IMM),   0xA3=>(I::LAX, A::IZX),
            0xA4=>(I::LDY, A::ZPG),   0xA5=>(I::LDA, A::ZPG),   0xA6=>(I::LDX, A::ZPG),   0xA7=>(I::LAX, A::ZPG),
            0xA8=>(I::TAY, A::IMP),   0xA9=>(I::LDA, A::IMM),   0xAA=>(I::TAX, A::IMP),   0xAB=>(I::LAX, A::IMM),
            0xAC=>(I::LDY, A::ABS),   0xAD=>(I::LDA, A::ABS),   0xAE=>(I::LDX, A::ABS),   0xAF=>(I::LAX, A::ABS),
            0xB0=>(I::BCS, A::REL),   0xB1=>(I::LDA, A::IZY),   0xB2=>(I::KIL, A::IMP),   0xB3=>(I::LAX, A::IZY),
            0xB4=>(I::LDY, A::ZPX),   0xB5=>(I::LDA, A::ZPX),   0xB6=>(I::LDX, A::ZPY),   0xB7=>(I::LAX, A::ZPY),
            0xB8=>(I::CLV, A::IMP),   0xB9=>(I::LDA, A::ABY),   0xBA=>(I::TSX, A::IMP),   0xBB=>(I::LAS, A::ABY),
            0xBC=>(I::LDY, A::ABX),   0xBD=>(I::LDA, A::ABX),   0xBE=>(I::LDX, A::ABY),   0xBF=>(I::LAX, A::ABY),

            0xC0=>(I::CPY, A::IMM),   0xC1=>(I::CMP, A::IZX),   0xC2=>(I::NOP, A::IMM),   0xC3=>(I::DCP, A::IZX),
            0xC4=>(I::CPY, A::ZPG),   0xC5=>(I::CMP, A::ZPG),   0xC6=>(I::DEC, A::ZPG),   0xC7=>(I::DCP, A::ZPG),
            0xC8=>(I::INY, A::IMP),   0xC9=>(I::CMP, A::IMM),   0xCA=>(I::DEX, A::IMP),   0xCB=>(I::AXS, A::IMM),
            0xCC=>(I::CPY, A::ABS),   0xCD=>(I::CMP, A::ABS),   0xCE=>(I::DEC, A::ABS),   0xCF=>(I::DCP, A::ABS),
            0xD0=>(I::BNE, A::REL),   0xD1=>(I::CMP, A::IZY),   0xD2=>(I::KIL, A::IMP),   0xD3=>(I::DCP, A::IZY),
            0xD4=>(I::NOP, A::ZPX),   0xD5=>(I::CMP, A::ZPX),   0xD6=>(I::DEC, A::ZPX),   0xD7=>(I::DCP, A::ZPX),
            0xD8=>(I::CLD, A::IMP),   0xD9=>(I::CMP, A::ABY),   0xDA=>(I::NOP, A::IMP),   0xDB=>(I::DCP, A::ABY),
            0xDC=>(I::NOP, A::ABX),   0xDD=>(I::CMP, A::ABX),   0xDE=>(I::DEC, A::ABX),   0xDF=>(I::DCP, A::ABX),
            
            0xE0=>(I::CPX, A::IMM),   0xE1=>(I::SBC, A::IZX),   0xE2=>(I::NOP, A::IMM),   0xE3=>(I::ISC, A::IZX),
            0xE4=>(I::CPX, A::ZPG),   0xE5=>(I::SBC, A::ZPG),   0xE6=>(I::INC, A::ZPG),   0xE7=>(I::ISC, A::ZPG),
            0xE8=>(I::INX, A::IMP),   0xE9=>(I::SBC, A::IMM),   0xEA=>(I::NOP, A::IMP),   0xEB=>(I::SBC, A::IMM),
            0xEC=>(I::CPX, A::ABS),   0xED=>(I::SBC, A::ABS),   0xEE=>(I::INC, A::ABS),   0xEF=>(I::ISC, A::ABS),
            0xF0=>(I::BEQ, A::REL),   0xF1=>(I::SBC, A::IZY),   0xF2=>(I::KIL, A::IMP),   0xF3=>(I::ISC, A::IZY),
            0xF4=>(I::NOP, A::ZPX),   0xF5=>(I::SBC, A::ZPX),   0xF6=>(I::INC, A::ZPX),   0xF7=>(I::ISC, A::ZPX),
            0xF8=>(I::SED, A::IMP),   0xF9=>(I::SBC, A::ABY),   0xFA=>(I::NOP, A::IMP),   0xFB=>(I::ISC, A::ABY),
            0xFC=>(I::NOP, A::ABX),   0xFD=>(I::SBC, A::ABX),   0xFE=>(I::INC, A::ABX),   0xFF=>(I::ISC, A::ABX),
        };
        // println!("{:X}, {:X}, {:?}, {:?}", self.regs().PC - 1, opcode, insturction, mode);
        // if self.regs().PC - 1 < 0x2000 {
        //     panic!()
        // }
        mode.execute_instruction(self, insturction)
    }

    #[inline]
    fn hardware_interrupt(&mut self) {
        let interrupt_addr = if self.state().nmi {
            INT_NMI_ADDRESS
        } else {
            INT_IRQ_BRK_ADDRESS
        };
        self.dummy_load(self.regs().PC);
        self.dummy_load(self.regs().PC);
        self.push(self.regs().PC.fetch_hi());
        self.push(self.regs().PC.fetch_lo());
        self.regs_mut().P.set(Flags::B, false);
        self.push(self.regs().P.bits);
        self.regs_mut().P.set(Flags::I, true);
        self.regs_mut().PC = self.load16(interrupt_addr);
    }

    #[inline]
    fn reset(&mut self) {
        // FIXME
        self.regs_mut().SP = 0x00FD;
        self.regs_mut().PC = self.load16(INT_RESET_ADDRESS);
    }

    #[inline]
    fn regs(&self) -> &Registers {
        &self.state().regs
    }

    #[inline]
    fn regs_mut(&mut self) -> &mut Registers {
        &mut self.state_mut().regs
    }

    #[inline]
    fn load(&mut self, addr: u16) -> u8 {
        self.peek(addr)
    }

    #[inline]
    fn store(&mut self, addr: u16, val: u8) {
        self.poke(addr, val)
    }

    #[inline]
    fn load16(&mut self, addr: u16) -> u16 {
        let low = self.load(addr) as u16;
        let addr1 = Wrapping(addr) + Wrapping(1);
        let high = (self.load(addr1.0) as u16) << 8;
        low | high
    }

    #[inline]
    fn fetch_and_inc_pc(&mut self) -> u8 {
        let addr = self.regs().PC;
        let next_pc = Wrapping(self.regs().PC) + Wrapping(1);
        self.regs_mut().PC = next_pc.0;
        self.load(addr)
    }

    #[inline]
    fn fetch16_and_inc_pc(&mut self) -> u16 {
        let addr = self.regs().PC;
        let next_pc = Wrapping(self.regs().PC) + Wrapping(2);
        self.regs_mut().PC = next_pc.0;
        self.load16(addr)
    }

    #[inline]
    fn stack_address(&self) -> u16 {
        self.regs().SP as u16 + 0x100
    }

    #[inline]
    fn push(&mut self, val: u8) {
        self.store(self.stack_address(), val);
        self.regs_mut().SP = (Wrapping(self.regs().SP) - Wrapping(1)).0;
    }

    #[inline]
    fn pull(&mut self) -> u8 {
        self.regs_mut().SP = (Wrapping(self.regs().SP) + Wrapping(1)).0; 
        let val = self.load(self.stack_address());
        val
    }

    #[inline]
    fn dummy_load(&mut self, addr: u16) {
        self.load(addr);
    }

    #[inline]
    fn dummy_store(&mut self, addr: u16, value: u8) {
        self.store(addr, value);
    }
}

#[inline]
fn is_cross_page(addr: u16, offset: u8) -> bool {
    (Wrapping(addr) + Wrapping(offset as u16)).0 & 0xFF00 != (addr & 0xFF00)
}

#[inline]
fn on_same_page(addr1: u16, addr2: u16) -> bool {
    addr1 & 0xff00 == addr2 & 0xff00
}

enum Operation{
    Read            (fn(&mut Registers, u8)),
    ReadModifyWrite (fn(&mut Registers, u8) -> u8),
    Write           (fn(&mut Registers) -> u8),
    BranchOn        (Flags, bool),
    SetRegister     (fn(&mut Registers)),
    PLP, PLA, PHP, PHA, RTI, RTS, BRK, JSR, JMP,  // These items are all control instructions
    Unimplemented,
}

#[derive(Debug)]
pub enum AddressingMode {
    IMM, ACC, ABS, ABX, ABY, ZPG, ZPX, ZPY, IZX, IZY, IMP, IND, REL
}

impl AddressingMode {
    fn execute_instruction<CPU: Private>(&self, cpu: &mut CPU, instruction: Instruction) {
        match self {
            AddressingMode::IMM => imm_inner(cpu, instruction),
            AddressingMode::ACC => acc_inner(cpu, instruction),
            AddressingMode::ABS => abs_inner(cpu, instruction),
            AddressingMode::ABX => abx_inner(cpu, instruction),
            AddressingMode::ABY => aby_inner(cpu, instruction),
            AddressingMode::ZPG => zpg_inner(cpu, instruction),
            AddressingMode::ZPX => zpx_inner(cpu, instruction),
            AddressingMode::ZPY => zpy_inner(cpu, instruction),
            AddressingMode::IZX => izx_inner(cpu, instruction),
            AddressingMode::IZY => izy_inner(cpu, instruction),
            AddressingMode::IMP => imp_inner(cpu, instruction),
            AddressingMode::IND => ind_inner(cpu, instruction),
            AddressingMode::REL => rel_inner(cpu, instruction)
        }
    }
}

#[derive(Debug)]
pub enum Instruction {
    NOP, LDA, LDX, LDY, CMP, CPX, CPY, ADC, SBC, BIT, AND, EOR, ORA, ASL, LSR, 
    ROL, ROR, INC, DEC, STA, STX, STY, BCC, BCS, BNE, BEQ, BPL, BMI, BVC, BVS, 
    INX, INY, DEX, DEY, TXA, TYA, TAX, TAY, TXS, TSX, CLC, CLI, CLV, CLD, SEC, 
    SEI, SED, JMP, PLP, PLA, PHP, PHA, RTI, RTS, BRK, JSR, KIL, ISC, DCP, AXS, 
    LAS, LAX, AHX, SAX, XAA, SHX, RRA, TAS, SHY, ARR, SRE, ALR, RLA, ANC, SLO,
}

impl Instruction {
    fn get_operation(&self) -> Operation {
        match self {
            Instruction::NOP => Operation::Read(|_, _| {
                // nothing to do
            }),
        
            Instruction::LDA => Operation::Read(|regs, val| {
                regs.A = val;
                regs.set_nz(val);
            }),
        
            Instruction::LDX => Operation::Read(|regs, val| {
                regs.X = val;
                regs.set_nz(val);
            }),
        
            Instruction::LDY => Operation::Read(|regs, val| {
                regs.Y = val;
                regs.set_nz(val);
            }),
        
            Instruction::CMP => Operation::Read(|regs, val| {
                let diff = Wrapping(regs.A) - Wrapping(val);
                regs.set_nz(diff.0);
                regs.P.set(Flags::C, regs.A >= val);
            }),
        
            Instruction::CPX => Operation::Read(|regs, val| {
                let diff = Wrapping(regs.X) - Wrapping(val);
                regs.set_nz(diff.0);
                regs.P.set(Flags::C, regs.X >= val);
            }),
        
            Instruction::CPY => Operation::Read(|regs, val| {
                let diff = Wrapping(regs.Y) - Wrapping(val);
                regs.set_nz(diff.0);
                regs.P.set(Flags::C, regs.Y >= val);
            }),
        
            Instruction::ADC => Operation::Read(|regs, val| {
                let r = Wrapping(regs.A as u16) + Wrapping(val as u16) + Wrapping(regs.get_c_as_u8() as u16);
                let a = regs.A;
                regs.set_cv(a, val, r.0);
                regs.A = regs.set_nz(r.0 as u8);
            }),
        
            Instruction::SBC => Operation::Read(|regs, val| {
                let val = val ^ 0xff;
                let r = Wrapping(regs.A as u16) + Wrapping(val as u16) + Wrapping(regs.get_c_as_u8() as u16);
                let a = regs.A;
                regs.set_cv(a, val, r.0);
                regs.A = regs.set_nz(r.0 as u8);
            }),
        
            Instruction::BIT => Operation::Read(|regs, val| {
                regs.P.set(Flags::Z, (val & regs.A) == 0);
                regs.P.set(Flags::N, val & 0x80 != 0);
                regs.P.set(Flags::V, val & 0x40 != 0);
            }),
        
            Instruction::AND => Operation::Read(|regs, val| {
                regs.A &= val;
                let a = regs.A;
                regs.set_nz(a);
            }),
        
            Instruction::EOR => Operation::Read(|regs, val| {
                regs.A ^= val;
                let a = regs.A;
                regs.set_nz(a);
            }),
        
            Instruction::ORA => Operation::Read(|regs, val| {
                regs.A |= val;
                let a = regs.A;
                regs.set_nz(a);
            }),
        
            Instruction::ASL => Operation::ReadModifyWrite(|regs, val| {
                regs.P.set(Flags::C, val & 0x80 != 0);
                regs.set_nz(val << 1)
            }),
        
            Instruction::LSR => Operation::ReadModifyWrite(|regs, val| {
                regs.P.set(Flags::C, val & 0x01 != 0);
                regs.set_nz(val >> 1)
            }),
        
            Instruction::ROL => Operation::ReadModifyWrite(|regs, val| {
                let c = regs.get_c_as_u8();
                regs.P.set(Flags::C, val & 0x80 != 0);
                regs.set_nz((val << 1) | c)
            }),
        
            Instruction::ROR => Operation::ReadModifyWrite(|regs, val| {
                let c = regs.get_c_as_u8();
                regs.P.set(Flags::C, val & 0x01 != 0);
                regs.set_nz((val >> 1) | c << 7)
            }),
        
            Instruction::INC => Operation::ReadModifyWrite(|regs, val| {
                let r = Wrapping(val) + Wrapping(1);
                regs.set_nz(r.0)
            }),
        
            Instruction::DEC => Operation::ReadModifyWrite(|regs, val| {
                let r = Wrapping(val) - Wrapping(1);
                regs.set_nz(r.0)
            }),
        
            Instruction::STA => Operation::Write(|regs| {
                regs.A
            }),
        
            Instruction::STX => Operation::Write(|regs| {
                regs.X
            }),
        
            Instruction::STY => Operation::Write(|regs| {
                regs.Y
            }),
        
            Instruction::BCC => Operation::BranchOn(Flags::C, false),
        
            Instruction::BCS => Operation::BranchOn(Flags::C, true),
        
            Instruction::BNE => Operation::BranchOn(Flags::Z, false),
        
            Instruction::BEQ => Operation::BranchOn(Flags::Z, true),
        
            Instruction::BPL => Operation::BranchOn(Flags::N, false),
        
            Instruction::BMI => Operation::BranchOn(Flags::N, true),
        
            Instruction::BVC => Operation::BranchOn(Flags::V, false),
        
            Instruction::BVS => Operation::BranchOn(Flags::V, true),
        
            Instruction::INX => Operation::SetRegister(|regs| {
                let r = Wrapping(regs.X) + Wrapping(1);
                regs.X = regs.set_nz(r.0)
            }),
        
            Instruction::INY => Operation::SetRegister(|regs| {
                let r = Wrapping(regs.Y) + Wrapping(1);
                regs.Y = regs.set_nz(r.0)
            }),
        
            Instruction::DEX => Operation::SetRegister(|regs| {
                let r = Wrapping(regs.X) - Wrapping(1);
                regs.X = regs.set_nz(r.0)
            }),
        
            Instruction::DEY => Operation::SetRegister(|regs| {
                let r = Wrapping(regs.Y) - Wrapping(1);
                regs.Y = regs.set_nz(r.0)
            }),
        
            Instruction::TXA => Operation::SetRegister(|regs| {
                regs.A = regs.set_nz(regs.X)
            }),
        
            Instruction::TYA => Operation::SetRegister(|regs| {
                regs.A = regs.set_nz(regs.Y)
            }),
        
            Instruction::TAX => Operation::SetRegister(|regs| {
                regs.X = regs.set_nz(regs.A)
            }),
        
            Instruction::TAY => Operation::SetRegister(|regs| {
                regs.Y = regs.set_nz(regs.A)
            }),
        
            Instruction::TXS => Operation::SetRegister(|regs| {
                regs.SP = regs.X  // no need to set N and Z
            }),
        
            Instruction::TSX => Operation::SetRegister(|regs| {
                regs.X = regs.set_nz(regs.SP)
            }),
        
            Instruction::CLC => Operation::SetRegister(|regs| {
                regs.P.set(Flags::C, false);
            }),
        
            Instruction::CLI => Operation::SetRegister(|regs| {
                regs.P.set(Flags::I, false);
            }),
        
            Instruction::CLV => Operation::SetRegister(|regs| {
                regs.P.set(Flags::V, false);
            }),
        
            Instruction::CLD => Operation::SetRegister(|regs| {
                regs.P.set(Flags::D, false);
            }),
        
            Instruction::SEC => Operation::SetRegister(|regs| {
                regs.P.set(Flags::C, true);
            }),
        
            Instruction::SEI => Operation::SetRegister(|regs| {
                regs.P.set(Flags::I, true);
            }),
        
            Instruction::SED => Operation::SetRegister(|regs| {
                regs.P.set(Flags::D, true);
            }),
        
            Instruction::JMP => Operation::JMP,
        
            Instruction::PLP => Operation::PLP,
        
            Instruction::PLA => Operation::PLA,
        
            Instruction::PHP => Operation::PHP,
        
            Instruction::PHA => Operation::PHA,
        
            Instruction::RTI => Operation::RTI,
        
            Instruction::RTS => Operation::RTS,
        
            Instruction::BRK => Operation::BRK,
        
            Instruction::JSR => Operation::JSR,
        
            Instruction::KIL => panic!("KIL instruction executed!"),
        
            Instruction::ISC => Operation::Unimplemented,
        
            Instruction::DCP => Operation::Unimplemented,
        
            Instruction::AXS => Operation::Unimplemented,
        
            Instruction::LAS => Operation::Unimplemented,
        
            Instruction::LAX => Operation::Unimplemented,
        
            Instruction::AHX => Operation::Unimplemented,
        
            Instruction::SAX => Operation::Unimplemented,
        
            Instruction::XAA => Operation::Unimplemented,
        
            Instruction::SHX => Operation::Unimplemented,
        
            Instruction::RRA => Operation::Unimplemented,
        
            Instruction::TAS => Operation::Unimplemented,
        
            Instruction::SHY => Operation::Unimplemented,
        
            Instruction::ARR => Operation::Unimplemented,
        
            Instruction::SRE => Operation::Unimplemented,
        
            Instruction::ALR => Operation::Unimplemented,
        
            Instruction::RLA => Operation::Unimplemented,
        
            Instruction::ANC => Operation::Unimplemented,
        
            Instruction::SLO => Operation::Unimplemented,
        }
    }
}

// addressing mode

fn imm_inner<CPU: Private>(cpu: &mut CPU, instruction: Instruction) {
    let val = cpu.fetch_and_inc_pc();
    match instruction.get_operation() {
        Operation::Read(f) => {
            f(cpu.regs_mut(), val)
        }
        Operation::Unimplemented => {panic!()},
        _ => panic!("Invalid instruction `{:?}` for `IMM`", instruction),
    };
}

fn acc_inner<CPU: Private>(cpu: &mut CPU, instruction: Instruction) {
    cpu.dummy_load(cpu.regs().PC);
    match instruction.get_operation() {
        Operation::ReadModifyWrite(f) => {
            let val = cpu.regs().A;
            cpu.regs_mut().A = f(cpu.regs_mut(), val);
        }
        Operation::Unimplemented => {panic!()},
        _ => panic!("Invalid instruction `{:?}` for `ACC`", instruction),
    };
}

fn abs_inner<CPU: Private>(cpu: &mut CPU, instruction: Instruction) {
    let low = cpu.fetch_and_inc_pc();
    match instruction.get_operation() {
        Operation::Read(f) => {
            let addr = ((cpu.fetch_and_inc_pc() as u16) << 8) | low as u16;
            let val = cpu.load(addr);
            f(cpu.regs_mut(), val);
        },
        Operation::ReadModifyWrite(f) => {
            let addr = ((cpu.fetch_and_inc_pc() as u16) << 8) | low as u16;
            let val = cpu.load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        Operation::Write(f) => {
            let addr = ((cpu.fetch_and_inc_pc() as u16) << 8) | low as u16;
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        },
        Operation::JMP => {
            let addr = ((cpu.fetch_and_inc_pc() as u16) << 8) | low as u16;
            cpu.regs_mut().PC = addr;
        },
        Operation::JSR => {
            let pc = cpu.regs().PC;
            let pch = (pc >> 8) as u8;
            let pcl = pc as u8;
            cpu.dummy_load(cpu.stack_address()); 
            cpu.push(pch); cpu.push(pcl);
            let high = cpu.load(pc);
            cpu.regs_mut().PC = (low as u16) | ((high as u16) << 8);
        },
        Operation::Unimplemented => {panic!()},
        _ => panic!("Invalid instruction `{:?}` for `ABS`", instruction),
    };
}

fn abx_inner<CPU: Private>(cpu: &mut CPU, instruction: Instruction) {
    let base = cpu.fetch16_and_inc_pc();
    let offset = cpu.regs().X;
    let addr = (Wrapping(base) + Wrapping(offset as u16)).0;
    match instruction.get_operation() {
        Operation::Read(f) => {
            let val = cpu.load(addr);
            if is_cross_page(base, offset) { cpu.dummy_load((base & 0xFF00) | (addr & 0x00FF)) };
            f(cpu.regs_mut(), val);
        },
        Operation::ReadModifyWrite(f) => {
            let val = cpu.load(addr);
            cpu.dummy_load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        Operation::Write(f) => {
            cpu.dummy_load(addr);
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        },
        Operation::Unimplemented => {panic!()},
        _ => panic!("Invalid instruction `{:?}` for `ABX`", instruction),
    }
}

fn aby_inner<CPU: Private>(cpu: &mut CPU, instruction: Instruction) {
    let base = cpu.fetch16_and_inc_pc();
    let offset = cpu.regs().Y;
    let addr = (Wrapping(base) + Wrapping(offset as u16)).0;
    match instruction.get_operation() {
        Operation::Read(f) => {
            let val = cpu.load(addr);
            if is_cross_page(base, offset) { cpu.dummy_load((base & 0xFF00) | (addr & 0x00FF)) };
            f(cpu.regs_mut(), val);
        },
        Operation::ReadModifyWrite(f) => {
            let val = cpu.load(addr);
            cpu.dummy_load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        Operation::Write(f) => {
            cpu.dummy_load(addr);
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        },
        Operation::Unimplemented => {panic!()},
        _ => panic!("Invalid instruction `{:?}` for `ABY`", instruction),
    }
}

fn zpg_inner<CPU: Private>(cpu: &mut CPU, instruction: Instruction) {
    let addr = cpu.fetch_and_inc_pc() as u16;
    match instruction.get_operation() {
        Operation::Read(f) => {
            let val = cpu.load(addr);
            f(cpu.regs_mut(), val);
        },
        Operation::ReadModifyWrite(f) => {
            let val = cpu.load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        Operation::Write(f) => {
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        }
        Operation::Unimplemented => {panic!()},
        _ => panic!("Invalid instruction `{:?}` for `ZPG`", instruction),
    }
}

fn zpx_inner<CPU: Private>(cpu: &mut CPU, instruction: Instruction) {
    let base = cpu.fetch_and_inc_pc() as u16;
    let offset = cpu.regs().X;
    let addr = (Wrapping(base) + Wrapping(offset as u16)).0 & 0xff;  // always less than 0x100
    cpu.dummy_load(base);
    match instruction.get_operation() {
        Operation::Read(f) => {
            let val = cpu.load(addr);
            f(cpu.regs_mut(), val);
        },
        Operation::ReadModifyWrite(f) => {
            let val = cpu.load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        Operation::Write(f) => {
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        }
        Operation::Unimplemented => {panic!()},
        _ => panic!("Invalid instruction `{:?}` for `ZPX`", instruction),
    }
}

fn zpy_inner<CPU: Private>(cpu: &mut CPU, instruction: Instruction) {
    let base = cpu.fetch_and_inc_pc() as u16;
    let offset = cpu.regs().Y;
    let addr = (Wrapping(base) + Wrapping(offset as u16)).0 & 0xff;  // always less than 0x100
    cpu.dummy_load(base);
    match instruction.get_operation() {
        Operation::Read(f) => {
            let val = cpu.load(addr);
            f(cpu.regs_mut(), val);
        },
        Operation::ReadModifyWrite(f) => {
            let val = cpu.load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        Operation::Write(f) => {
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        }
        Operation::Unimplemented => {panic!()},
        _ => panic!("Invalid instruction `{:?}` for `ZPY`", instruction),
    }
}

fn izx_inner<CPU: Private>(cpu: &mut CPU, instruction: Instruction) {
    let pointer = cpu.fetch_and_inc_pc();
    cpu.dummy_load(pointer as u16);
    let pointer = Wrapping(pointer) + Wrapping(cpu.regs().X);
    let low = cpu.load(pointer.0 as u16) as u16;
    let high = cpu.load((pointer + Wrapping(1)).0 as u16) as u16;
    let addr = low | (high << 8);
    match instruction.get_operation() {
        Operation::Read(f) => {
            let val = cpu.load(addr);
            f(cpu.regs_mut(), val);
        },
        Operation::ReadModifyWrite(f) => {
            let val = cpu.load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        Operation::Write(f) => {
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        }
        Operation::Unimplemented => {panic!()},
        _ => panic!("Invalid instruction `{:?}` for `IZX`", instruction),
    }
}

fn izy_inner<CPU: Private>(cpu: &mut CPU, instruction: Instruction) {
    let pointer = cpu.fetch_and_inc_pc();
    let low = cpu.load(pointer as u16) as u16;
    let high = cpu.load((Wrapping(pointer) + Wrapping(1)).0 as u16) as u16;
    let base = low | (high << 8);
    let offset = cpu.regs().Y;
    let addr = (Wrapping(base) + Wrapping(offset as u16)).0;
    match instruction.get_operation() {
        Operation::Read(f) => {
            if is_cross_page(base, offset) { cpu.dummy_load((base & 0xFF00) | (addr & 0x00FF)); };
            let val = cpu.load(addr);
            f(cpu.regs_mut(), val);
        },
        Operation::ReadModifyWrite(f) => {
            cpu.dummy_load(addr);
            let val = cpu.load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        Operation::Write(f) => {
            cpu.dummy_load(addr);
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        },
        Operation::Unimplemented => {panic!()},
        _ => panic!("Invalid instruction `{:?}` for `IZY`", instruction),
    }
}

fn imp_inner<CPU: Private>(cpu: &mut CPU, instruction: Instruction) {
    cpu.dummy_load(cpu.regs().PC);
    match instruction.get_operation() {
        Operation::Read(_) => {
            // nothing to do
        }
        Operation::SetRegister(f) => {
            f(cpu.regs_mut());
        },
        Operation::PLP => {
            cpu.dummy_load(cpu.stack_address());  // increment S
            let new_p = cpu.pull();
            cpu.regs_mut().P.bits = new_p;
        },
        Operation::PLA => {
            cpu.dummy_load(cpu.stack_address());  // increment S
            let new_a = cpu.pull();
            cpu.regs_mut().A = cpu.regs_mut().set_nz(new_a);
        }
        Operation::PHP => {
            let p = cpu.regs().P.bits;
            cpu.push(p);
        },
        Operation::PHA => {
            let a = cpu.regs().A;
            cpu.push(a);
        },
        Operation::RTI => {
            cpu.dummy_load(cpu.stack_address());  // increment S
            cpu.regs_mut().P.bits = cpu.pull();
            let pcl = cpu.pull() as u16;
            let pch = (cpu.pull() as u16) << 8;
            cpu.regs_mut().PC = pcl | pch;
        },
        Operation::RTS => {
            cpu.dummy_load(cpu.stack_address());  // increment S
            let pcl = cpu.pull() as u16;
            let pch = (cpu.pull() as u16) << 8;
            cpu.regs_mut().PC = pcl | pch;
            cpu.fetch_and_inc_pc();
        },
        Operation::BRK => {
            let pc = cpu.regs().PC + 1;
            let pch = (pc >> 8) as u8;
            let pcl = pc as u8;
            cpu.push(pch); cpu.push(pcl);
            let interrupt_addr = if cpu.state().nmi {
                INT_NMI_ADDRESS
            } else {
                INT_IRQ_BRK_ADDRESS
            };
            cpu.regs_mut().P.set(Flags::B, true);
            cpu.push(cpu.regs().P.bits);
            cpu.regs_mut().P.set(Flags::I, true);
            cpu.regs_mut().PC = cpu.load16(interrupt_addr)
        },
        Operation::Unimplemented => {panic!()},
        _ => panic!("Invalid instruction `{:?}` for `IMP`", instruction),
    }
}

fn ind_inner<CPU: Private>(cpu: &mut CPU, instruction: Instruction) {
    let pointer = cpu.fetch16_and_inc_pc();
    let low = cpu.load(pointer) as u16;
    let high = (
        cpu.load(
            (pointer & 0xff00) | ((Wrapping(pointer) + Wrapping(1)).0 & 0xff)
        )
    ) as u16;
    let addr = low | (high << 8);
    match instruction.get_operation() {        
        Operation::JMP => {
            cpu.regs_mut().PC = addr;
        },
        Operation::Unimplemented => {panic!()},
        _ => panic!("Invalid instruction `{:?}` for `IND`", instruction),
    }
}

fn rel_inner<CPU: Private>(cpu: &mut CPU, instruction: Instruction) {
    let operand = cpu.fetch_and_inc_pc() as i8 as i16 as u16;
    match instruction.get_operation() {
        Operation::BranchOn(flag, is_set) => {
            if cpu.regs().P.contains(flag) == is_set {
                let old_pc = cpu.regs().PC;
                let new_pc = (Wrapping(cpu.regs().PC) + Wrapping(operand)).0;
                cpu.dummy_load(cpu.regs().PC);
                cpu.regs_mut().PC = new_pc;
                if !on_same_page(old_pc, new_pc) { cpu.dummy_load((old_pc & 0xFF00) | (new_pc & 0x00FF)); };
            }
        },
        Operation::Unimplemented => {panic!()},
        _ => panic!("Invalid instruction `{:?}` for `REL`", instruction),
    }
}