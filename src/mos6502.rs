#![allow(dead_code)]

use std::num::Wrapping;

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
    fn skip_one_cycle(&mut self);
}

pub trait Interface: Sized + Context {
    fn step(&mut self) {
        let opcode: u8 = Private::fetch_and_inc_pc(self);
        let (insturction, addressing): (fn()->AccessMode, fn(&mut Self, AccessMode)) = match opcode {
            0x00=>(brk, imp),   0x01=>(ora, izx),   0x02=>(kil, imp),   0x03=>(slo, izx),
            0x04=>(nop, zpg),   0x05=>(ora, zpg),   0x06=>(asl, zpg),   0x07=>(slo, zpg),
            0x08=>(php, imp),   0x09=>(ora, imm),   0x0a=>(asl, acc),   0x0b=>(anc, imm),
            0x0c=>(nop, abs),   0x0d=>(ora, abs),   0x0e=>(asl, abs),   0x0f=>(slo, abs),
            0x10=>(bpl, rel),   0x11=>(ora, izy),   0x12=>(kil, imp),   0x13=>(slo, izy),
            0x14=>(nop, zpx),   0x15=>(ora, zpx),   0x16=>(asl, zpx),   0x17=>(slo, zpx),
            0x18=>(clc, imp),   0x19=>(ora, aby),   0x1a=>(nop, imp),   0x1b=>(slo, aby),
            0x1c=>(nop, abx),   0x1d=>(ora, abx),   0x1e=>(asl, abx),   0x1f=>(slo, abx),

            0x20=>(jsr, abs),   0x21=>(and, izx),   0x22=>(kil, imp),   0x23=>(rla, izx),
            0x24=>(bit, zpg),   0x25=>(and, zpg),   0x26=>(rol, zpg),   0x27=>(rla, zpg),
            0x28=>(plp, imp),   0x29=>(and, imm),   0x2a=>(rol, acc),   0x2b=>(anc, imm),
            0x2c=>(bit, abs),   0x2d=>(and, abs),   0x2e=>(rol, abs),   0x2f=>(rla, abs),
            0x30=>(bmi, rel),   0x31=>(and, izy),   0x32=>(kil, imp),   0x33=>(rla, izy),
            0x34=>(nop, zpx),   0x35=>(and, zpx),   0x36=>(rol, zpx),   0x37=>(rla, zpx),
            0x38=>(sec, imp),   0x39=>(and, aby),   0x3a=>(nop, imp),   0x3b=>(rla, aby),
            0x3c=>(nop, abx),   0x3d=>(and, abx),   0x3e=>(rol, abx),   0x3f=>(rla, abx),

            0x40=>(rti, imp),   0x41=>(eor, izx),   0x42=>(kil, imp),   0x43=>(sre, izx),
            0x44=>(nop, zpg),   0x45=>(eor, zpg),   0x46=>(lsr, zpg),   0x47=>(sre, zpg),
            0x48=>(pha, imp),   0x49=>(eor, imm),   0x4a=>(lsr, acc),   0x4b=>(alr, imm),
            0x4c=>(jmp, abs),   0x4d=>(eor, abs),   0x4e=>(lsr, abs),   0x4f=>(sre, abs),
            0x50=>(bvc, rel),   0x51=>(eor, izy),   0x52=>(kil, imp),   0x53=>(sre, izy),
            0x54=>(nop, zpx),   0x55=>(eor, zpx),   0x56=>(lsr, zpx),   0x57=>(sre, zpx),
            0x58=>(cli, imp),   0x59=>(eor, aby),   0x5a=>(nop, imp),   0x5b=>(sre, aby),
            0x5c=>(nop, abx),   0x5d=>(eor, abx),   0x5e=>(lsr, abx),   0x5f=>(sre, abx),

            0x60=>(rts, imp),   0x61=>(adc, izx),   0x62=>(kil, imp),   0x63=>(rra, izx),
            0x64=>(nop, zpg),   0x65=>(adc, zpg),   0x66=>(ror, zpg),   0x67=>(rra, zpg),
            0x68=>(pla, imp),   0x69=>(adc, imm),   0x6a=>(ror, acc),   0x6b=>(arr, imm),
            0x6c=>(jmp, ind),   0x6d=>(adc, abs),   0x6e=>(ror, abs),   0x6f=>(rra, abs),
            0x70=>(bvs, rel),   0x71=>(adc, izy),   0x72=>(kil, imp),   0x73=>(rra, izy),
            0x74=>(nop, zpx),   0x75=>(adc, zpx),   0x76=>(ror, zpx),   0x77=>(rra, zpx),
            0x78=>(sei, imp),   0x79=>(adc, aby),   0x7a=>(nop, imp),   0x7b=>(rra, aby),
            0x7c=>(nop, abx),   0x7d=>(adc, abx),   0x7e=>(ror, abx),   0x7f=>(rra, abx),

            0x80=>(nop, imm),   0x81=>(sta, izx),   0x82=>(nop, imm),   0x83=>(sax, izx),
            0x84=>(sty, zpg),   0x85=>(sta, zpg),   0x86=>(stx, zpg),   0x87=>(sax, zpg),
            0x88=>(dey, imp),   0x89=>(nop, imm),   0x8a=>(txa, imp),   0x8b=>(xaa, imm),
            0x8c=>(sty, abs),   0x8d=>(sta, abs),   0x8e=>(stx, abs),   0x8f=>(sax, abs),
            0x90=>(bcc, rel),   0x91=>(sta, izy),   0x92=>(kil, imp),   0x93=>(ahx, izy),
            0x94=>(sty, zpx),   0x95=>(sta, zpx),   0x96=>(stx, zpy),   0x97=>(sax, zpy),
            0x98=>(tya, imp),   0x99=>(sta, aby),   0x9a=>(txs, imp),   0x9b=>(tas, aby),
            0x9c=>(shy, abx),   0x9d=>(sta, abx),   0x9e=>(shx, aby),   0x9f=>(ahx, aby),

            0xa0=>(ldy, imm),   0xa1=>(lda, izx),   0xa2=>(ldx, imm),   0xa3=>(lax, izx),
            0xa4=>(ldy, zpg),   0xa5=>(lda, zpg),   0xa6=>(ldx, zpg),   0xa7=>(lax, zpg),
            0xa8=>(tay, imp),   0xa9=>(lda, imm),   0xaa=>(tax, imp),   0xab=>(lax, imm),
            0xac=>(ldy, abs),   0xad=>(lda, abs),   0xae=>(ldx, abs),   0xaf=>(lax, abs),
            0xb0=>(bcs, rel),   0xb1=>(lda, izy),   0xb2=>(kil, imp),   0xb3=>(lax, izy),
            0xb4=>(ldy, zpx),   0xb5=>(lda, zpx),   0xb6=>(ldx, zpy),   0xb7=>(lax, zpy),
            0xb8=>(clv, imp),   0xb9=>(lda, aby),   0xba=>(tsx, imp),   0xbb=>(las, aby),
            0xbc=>(ldy, abx),   0xbd=>(lda, abx),   0xbe=>(ldx, aby),   0xbf=>(lax, aby),

            0xc0=>(cpy, imm),   0xc1=>(cmp, izx),   0xc2=>(nop, imm),   0xc3=>(dcp, izx),
            0xc4=>(cpy, zpg),   0xc5=>(cmp, zpg),   0xc6=>(dec, zpg),   0xc7=>(dcp, zpg),
            0xc8=>(iny, imp),   0xc9=>(cmp, imm),   0xca=>(dex, imp),   0xcb=>(axs, imm),
            0xcc=>(cpy, abs),   0xcd=>(cmp, abs),   0xce=>(dec, abs),   0xcf=>(dcp, abs),
            0xd0=>(bne, rel),   0xd1=>(cmp, izy),   0xd2=>(kil, imp),   0xd3=>(dcp, izy),
            0xd4=>(nop, zpx),   0xd5=>(cmp, zpx),   0xd6=>(dec, zpx),   0xd7=>(dcp, zpx),
            0xd8=>(cld, imp),   0xd9=>(cmp, aby),   0xda=>(nop, imp),   0xdb=>(dcp, aby),
            0xdc=>(nop, abx),   0xdd=>(cmp, abx),   0xde=>(dec, abx),   0xdf=>(dcp, abx),
            
            0xe0=>(cpx, imm),   0xe1=>(sbc, izx),   0xe2=>(nop, imm),   0xe3=>(isc, izx),
            0xe4=>(cpx, zpg),   0xe5=>(sbc, zpg),   0xe6=>(inc, zpg),   0xe7=>(isc, zpg),
            0xe8=>(inx, imp),   0xe9=>(sbc, imm),   0xea=>(nop, imp),   0xeb=>(sbc, imm),
            0xec=>(cpx, abs),   0xed=>(sbc, abs),   0xee=>(inc, abs),   0xef=>(isc, abs),
            0xf0=>(beq, rel),   0xf1=>(sbc, izy),   0xf2=>(kil, imp),   0xf3=>(isc, izy),
            0xf4=>(nop, zpx),   0xf5=>(sbc, zpx),   0xf6=>(inc, zpx),   0xf7=>(isc, zpx),
            0xf8=>(sed, imp),   0xf9=>(sbc, aby),   0xfa=>(nop, imp),   0xfb=>(isc, aby),
            0xfc=>(nop, abx),   0xfd=>(sbc, abx),   0xfe=>(inc, abx),   0xff=>(isc, abx),
        };
        addressing(self, insturction());
    }
}

impl<T: Context> Interface for T {}
impl<T: Context> Private for T {}
trait Private: Sized + Context {
    #[inline]
    fn tick(&mut self) {
        self.skip_one_cycle();
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
        self.tick();
        self.peek(addr)
    }

    #[inline]
    fn store(&mut self, addr: u16, val: u8) {
        self.tick();
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

enum AccessMode{
    Read            (fn(&mut Registers, u8)),
    ReadModifyWrite (fn(&mut Registers, u8) -> u8),
    Write           (fn(&mut Registers) -> u8),
    BranchOn        (Flags, bool),
    SetRegister     (fn(&mut Registers)),
    PLP, PLA, PHP, PHA, RTI, RTS, BRK, JSR, JMP  // These items are all control instructions
}

#[inline]
fn is_cross_page(addr: u16, offset: u8) -> bool {
    (Wrapping(addr) + Wrapping(offset as u16)).0 & 0xFF00 != (addr & 0xFF00)
}

#[inline]
fn on_same_page(addr1: u16, addr2: u16) -> bool {
    addr1 & 0xff00 == addr2 & 0xff00
}

// addressing mode

fn imm<CPU: Private>(cpu: &mut CPU, access: AccessMode) {
    let val = cpu.fetch_and_inc_pc();
    match access {
        AccessMode::Read(f) => {
            f(cpu.regs_mut(), val)
        }
        _ => panic!("Invalid access mode of `imm`"),
    };
}

fn acc<CPU: Private>(cpu: &mut CPU, access: AccessMode) {
    let _ = cpu.fetch_and_inc_pc();
    match access {
        AccessMode::ReadModifyWrite(f) => {
            let val = cpu.regs().A;
            cpu.regs_mut().A = f(cpu.regs_mut(), val);
        }
        _ => panic!("Invalid access mode of `acc`"),
    };
}

fn abs<CPU: Private>(cpu: &mut CPU, access: AccessMode) {
    let addr = cpu.fetch16_and_inc_pc();
    match access {
        AccessMode::Read(f) => {
            let val = cpu.load(addr);
            f(cpu.regs_mut(), val);
        },
        AccessMode::ReadModifyWrite(f) => {
            let val = cpu.load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        AccessMode::Write(f) => {
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        },
        AccessMode::JMP => {
            cpu.regs_mut().PC = addr;
        },
        _ => panic!("Invalid access mode of `abs`"),
    };
}

fn abx<CPU: Private>(cpu: &mut CPU, access: AccessMode) {
    let base = cpu.fetch16_and_inc_pc();
    let offset = cpu.regs().X;
    let addr = (Wrapping(base) + Wrapping(offset as u16)).0;
    match access {
        AccessMode::Read(f) => {
            let val = cpu.load(addr);
            if is_cross_page(base, offset) { cpu.dummy_load(addr) };
            f(cpu.regs_mut(), val);
        },
        AccessMode::ReadModifyWrite(f) => {
            let val = cpu.load(addr);
            cpu.dummy_load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        AccessMode::Write(f) => {
            cpu.dummy_load(addr);
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        },
        _ => panic!("Invalid access mode of `abx`"),
    }
}

fn aby<CPU: Private>(cpu: &mut CPU, access: AccessMode) {
    let base = cpu.fetch16_and_inc_pc();
    let offset = cpu.regs().Y;
    let addr = (Wrapping(base) + Wrapping(offset as u16)).0;
    match access {
        AccessMode::Read(f) => {
            let val = cpu.load(addr);
            if is_cross_page(base, offset) { cpu.dummy_load(addr) };
            f(cpu.regs_mut(), val);
        },
        AccessMode::ReadModifyWrite(f) => {
            let val = cpu.load(addr);
            cpu.dummy_load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        AccessMode::Write(f) => {
            cpu.dummy_load(addr);
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        },
        _ => panic!("Invalid access mode of `aby`"),
    }
}

fn zpg<CPU: Private>(cpu: &mut CPU, access: AccessMode) {
    let addr = cpu.fetch_and_inc_pc() as u16;
    match access {
        AccessMode::Read(f) => {
            let val = cpu.load(addr);
            f(cpu.regs_mut(), val);
        },
        AccessMode::ReadModifyWrite(f) => {
            let val = cpu.load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        AccessMode::Write(f) => {
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        }
        _ => panic!("Invalid access mode of `zpg`"),
    }
}

fn zpx<CPU: Private>(cpu: &mut CPU, access: AccessMode) {
    let base = cpu.fetch_and_inc_pc() as u16;
    let offset = cpu.regs().X;
    let addr = (Wrapping(base) + Wrapping(offset as u16)).0 & 0xff;  // always less than 0x100
    cpu.dummy_load(base);
    match access {
        AccessMode::Read(f) => {
            let val = cpu.load(addr);
            f(cpu.regs_mut(), val);
        },
        AccessMode::ReadModifyWrite(f) => {
            let val = cpu.load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        AccessMode::Write(f) => {
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        }
        _ => panic!("Invalid access mode of `zpx`"),
    }
}

fn zpy<CPU: Private>(cpu: &mut CPU, access: AccessMode) {
    let base = cpu.fetch_and_inc_pc() as u16;
    let offset = cpu.regs().Y;
    let addr = (Wrapping(base) + Wrapping(offset as u16)).0 & 0xff;  // always less than 0x100
    cpu.dummy_load(base);
    match access {
        AccessMode::Read(f) => {
            let val = cpu.load(addr);
            f(cpu.regs_mut(), val);
        },
        AccessMode::ReadModifyWrite(f) => {
            let val = cpu.load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        AccessMode::Write(f) => {
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        }
        _ => panic!("Invalid access mode of `zpy`"),
    }
}

fn izx<CPU: Private>(cpu: &mut CPU, access: AccessMode) {
    let pointer = cpu.fetch_and_inc_pc();
    cpu.dummy_load(pointer as u16);
    let pointer = Wrapping(pointer) + Wrapping(cpu.regs().X);
    let low = cpu.load(pointer.0 as u16) as u16;
    let high = cpu.load((pointer + Wrapping(1)).0 as u16) as u16;
    let addr = low | (high << 8);
    match access {
        AccessMode::Read(f) => {
            let val = cpu.load(addr);
            f(cpu.regs_mut(), val);
        },
        AccessMode::ReadModifyWrite(f) => {
            let val = cpu.load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        AccessMode::Write(f) => {
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        }
        _ => panic!("Invalid access mode of `izx`"),
    }
}

fn izy<CPU: Private>(cpu: &mut CPU, access: AccessMode) {
    let pointer = cpu.fetch_and_inc_pc();
    let low = cpu.load(pointer as u16) as u16;
    let high = cpu.load((Wrapping(pointer) + Wrapping(1)).0 as u16) as u16;
    let base_addr = low | (high << 8);
    let offset = cpu.regs().Y;
    let addr = (Wrapping(base_addr) + Wrapping(offset as u16)).0;
    match access {
        AccessMode::Read(f) => {
            if is_cross_page(base_addr, offset) { cpu.dummy_load(addr); };
            let val = cpu.load(addr);
            f(cpu.regs_mut(), val);
        },
        AccessMode::ReadModifyWrite(f) => {
            cpu.dummy_load(addr);
            let val = cpu.load(addr);
            let res = f(cpu.regs_mut(), val);
            cpu.dummy_store(addr, res);
            cpu.store(addr, res);
        },
        AccessMode::Write(f) => {
            cpu.dummy_load(addr);
            let res = f(cpu.regs_mut());
            cpu.store(addr, res);
        },
        _ => panic!("Invalid access mode of `izy`"),
    }
}

fn imp<CPU: Private>(cpu: &mut CPU, access: AccessMode) {
    let low = cpu.fetch_and_inc_pc();
    match access {
        AccessMode::SetRegister(f) => {
            f(cpu.regs_mut());
        },
        AccessMode::PLP => {
            cpu.dummy_load(cpu.stack_address());  // increment S
            let new_p = cpu.pull();
            cpu.regs_mut().P.bits = new_p;
        },
        AccessMode::PLA => {
            cpu.dummy_load(cpu.stack_address());  // increment S
            let new_a = cpu.pull();
            cpu.regs_mut().A = cpu.regs_mut().set_nz(new_a);
        }
        AccessMode::PHP => {
            let p = cpu.regs().P.bits;
            cpu.push(p);
        },
        AccessMode::PHA => {
            let a = cpu.regs().A;
            cpu.push(a);
        },
        AccessMode::RTI => {
            cpu.dummy_load(cpu.stack_address());  // increment S
            cpu.regs_mut().P.bits = cpu.pull();
            let pcl = cpu.pull() as u16;
            let pch = (cpu.pull() as u16) << 8;
            cpu.regs_mut().PC = pcl | pch;
        },
        AccessMode::RTS => {
            cpu.dummy_load(cpu.stack_address());  // increment S
            let pcl = cpu.pull() as u16;
            let pch = (cpu.pull() as u16) << 8;
            cpu.regs_mut().PC = pcl | pch;
            cpu.tick();  // increment PC
            let next_pc = Wrapping(cpu.regs().PC) + Wrapping(1);
            cpu.regs_mut().PC = next_pc.0;
        },
        AccessMode::BRK => {
            let pc = cpu.regs().PC;
            let pch = (pc >> 8) as u8;
            let pcl = pc as u8;
            cpu.push(pch); cpu.push(pcl);
            let mut p = cpu.regs().P;
            p.set(Flags::B, true);
            cpu.push(p.bits);
            cpu.regs_mut().PC = cpu.load16(0xfffe)
        },
        AccessMode::JSR => {
            let pc = cpu.regs().PC;
            let pch = (pc >> 8) as u8;
            let pcl = pc as u8;
            cpu.tick();
            cpu.push(pch); cpu.push(pcl);
            let high = cpu.load(pc);
            cpu.regs_mut().PC = (low as u16) | ((high as u16) << 8);
        },
        _ => panic!("Invalid access mode of `imp`"),
    }
}

fn ind<CPU: Private>(cpu: &mut CPU, access: AccessMode) {
    let pointer = cpu.fetch16_and_inc_pc();
    let low = cpu.load(pointer) as u16;
    let high = (
        cpu.load(
            (pointer & 0xff00) | ((Wrapping(pointer) + Wrapping(1)).0 & 0xff)
        )
    ) as u16;
    let addr = low | (high << 8);
    match access {        
        AccessMode::JMP => {
            cpu.regs_mut().PC = addr;
        },
        _ => panic!("Invalid access mode of `ind`"),
    }
}

fn rel<CPU: Private>(cpu: &mut CPU, access: AccessMode) {
    let operand = cpu.fetch_and_inc_pc() as u16;
    let new_pc = cpu.regs().PC + operand;
    match access {
        AccessMode::BranchOn(flag, is_set) => {
            if cpu.regs().P.contains(flag) == is_set {
                cpu.dummy_load(cpu.regs().PC);
                let old_pc = cpu.regs().PC;
                cpu.regs_mut().PC = new_pc;
                if !on_same_page(old_pc, new_pc) { cpu.dummy_load(cpu.regs().PC); };
            }
        },
        _ => panic!("Invalid access mode of `rel`"),
    }
}


// instructions

fn nop() -> AccessMode {
    AccessMode::Read(|_, _| {
        // nothing
    })
}

fn lda() -> AccessMode {
    AccessMode::Read(|regs, val| {
        regs.A = val
    })
}

fn ldx() -> AccessMode {
    AccessMode::Read(|regs, val| {
        regs.X = val
    })
}

fn ldy() -> AccessMode {
    AccessMode::Read(|regs, val| {
        regs.Y = val
    })
}

fn cmp() -> AccessMode {
    AccessMode::Read(|regs, val| {
        let diff = Wrapping(regs.A) - Wrapping(val);
        regs.set_nz(diff.0);
        regs.P.set(Flags::C, regs.A >= val);
    })
}

fn cpx() -> AccessMode {
    AccessMode::Read(|regs, val| {
        let diff = Wrapping(regs.X) - Wrapping(val);
        regs.set_nz(diff.0);
        regs.P.set(Flags::C, regs.X >= val);
    })
}

fn cpy() -> AccessMode {
    AccessMode::Read(|regs, val| {
        let diff = Wrapping(regs.Y) - Wrapping(val);
        regs.set_nz(diff.0);
        regs.P.set(Flags::C, regs.Y >= val);
    })
}

fn adc() -> AccessMode {
    AccessMode::Read(|regs, val| {
        let r = Wrapping(regs.A as u16) + Wrapping(val as u16) + Wrapping(regs.get_c_as_u8() as u16);
        let a = regs.A;
        regs.set_cv(a, val, r.0);
        regs.A = regs.set_nz(r.0 as u8);
    })
}

fn sbc() -> AccessMode {
    AccessMode::Read(|regs, val| {
        let val = val & 0xff;
        let r = Wrapping(regs.A as u16) + Wrapping(val as u16) + Wrapping(regs.get_c_as_u8() as u16);
        let a = regs.A;
        regs.set_cv(a, val, r.0);
        regs.A = regs.set_nz(r.0 as u8);
    })
}

fn bit() -> AccessMode {
    AccessMode::Read(|regs, val| {
        regs.P.set(Flags::Z, !(val & regs.A) != 0);
        regs.P.set(Flags::N, val & 0x80 != 0);
        regs.P.set(Flags::V, val & 0x40 != 0);
    })
}

fn and() -> AccessMode {
    AccessMode::Read(|regs, val| {
        regs.A &= val;
        let a = regs.A;
        regs.set_nz(a);
    })
}

fn eor() -> AccessMode {
    AccessMode::Read(|regs, val| {
        regs.A ^= val;
        let a = regs.A;
        regs.set_nz(a);
    })
}

fn ora() -> AccessMode {
    AccessMode::Read(|regs, val| {
        regs.A |= val;
        let a = regs.A;
        regs.set_nz(a);
    })
}

fn asl() -> AccessMode {
    AccessMode::ReadModifyWrite(|regs, val| {
        regs.P.set(Flags::C, val & 0x80 != 0);
        regs.set_nz(val << 1)
    })
}

fn lsr() -> AccessMode {
    AccessMode::ReadModifyWrite(|regs, val| {
        regs.P.set(Flags::C, val & 0x01 != 0);
        regs.set_nz(val >> 1)
    })
}

fn rol() -> AccessMode {
    AccessMode::ReadModifyWrite(|regs, val| {
        let c = regs.get_c_as_u8();
        regs.P.set(Flags::C, val & 0x80 != 0);
        regs.set_nz((val << 1) | c)
    })
}

fn ror() -> AccessMode {
    AccessMode::ReadModifyWrite(|regs, val| {
        let c = regs.get_c_as_u8();
        regs.P.set(Flags::C, val & 0x01 != 0);
        regs.set_nz((val >> 1) | c << 7)
    })
}

fn inc() -> AccessMode {
    AccessMode::ReadModifyWrite(|regs, val| {
        let r = Wrapping(val) + Wrapping(1);
        regs.set_nz(r.0)
    })
}

fn dec() -> AccessMode {
    AccessMode::ReadModifyWrite(|regs, val| {
        let r = Wrapping(val) - Wrapping(1);
        regs.set_nz(r.0)
    })
}

fn sta() -> AccessMode {
    AccessMode::Write(|regs| {
        regs.A
    })
}

fn stx() -> AccessMode {
    AccessMode::Write(|regs| {
        regs.X
    })
}

fn sty() -> AccessMode {
    AccessMode::Write(|regs| {
        regs.Y
    })
}

fn bcc() -> AccessMode {
    AccessMode::BranchOn(Flags::C, false)
}

fn bcs() -> AccessMode {
    AccessMode::BranchOn(Flags::C, true)
}

fn bne() -> AccessMode {
    AccessMode::BranchOn(Flags::Z, false)
}

fn beq() -> AccessMode {
    AccessMode::BranchOn(Flags::Z, true)
}

fn bpl() -> AccessMode {
    AccessMode::BranchOn(Flags::N, false)
}

fn bmi() -> AccessMode {
    AccessMode::BranchOn(Flags::N, true)
}

fn bvc() -> AccessMode {
    AccessMode::BranchOn(Flags::V, false)
}

fn bvs() -> AccessMode {
    AccessMode::BranchOn(Flags::V, true)
}

fn inx() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        let r = Wrapping(regs.X) + Wrapping(1);
        regs.X = regs.set_nz(r.0)
    })
}

fn iny() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        let r = Wrapping(regs.Y) + Wrapping(1);
        regs.Y = regs.set_nz(r.0)
    })
}

fn dex() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        let r = Wrapping(regs.X) - Wrapping(1);
        regs.X = regs.set_nz(r.0)
    })
}

fn dey() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        let r = Wrapping(regs.Y) - Wrapping(1);
        regs.Y = regs.set_nz(r.0)
    })
}

fn txa() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        regs.A = regs.set_nz(regs.X)
    })
}

fn tya() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        regs.A = regs.set_nz(regs.Y)
    })
}

fn tax() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        regs.X = regs.set_nz(regs.A)
    })
}

fn tay() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        regs.Y = regs.set_nz(regs.A)
    })
}

fn txs() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        regs.SP = regs.X  // no need to set N and Z
    })
}

fn tsx() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        regs.X = regs.set_nz(regs.SP)
    })
}

fn clc() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        regs.P.set(Flags::C, false);
    })
}

fn cli() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        regs.P.set(Flags::I, false);
    })
}

fn clv() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        regs.P.set(Flags::V, false);
    })
}

fn cld() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        regs.P.set(Flags::D, false);
    })
}

fn sec() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        regs.P.set(Flags::C, true);
    })
}

fn sei() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        regs.P.set(Flags::I, true);
    })
}

fn sed() -> AccessMode {
    AccessMode::SetRegister(|regs| {
        regs.P.set(Flags::D, true);
    })
}

fn jmp() -> AccessMode {
    AccessMode::JMP
}

fn plp() -> AccessMode {
    AccessMode::PLP
}

fn pla() -> AccessMode {
    AccessMode::PLA
}

fn php() -> AccessMode {
    AccessMode::PHP
}

fn pha() -> AccessMode {
    AccessMode::PHA
}

fn rti() -> AccessMode {
    AccessMode::RTI
}

fn rts() -> AccessMode {
    AccessMode::RTS
}

fn brk() -> AccessMode {
    AccessMode::BRK
}

fn jsr() -> AccessMode {
    AccessMode::JSR
}

fn kil() -> AccessMode {
    unimplemented!()
}

fn isc() -> AccessMode {
    unimplemented!()
}

fn dcp() -> AccessMode {
    unimplemented!()
}

fn axs() -> AccessMode {
    unimplemented!()
}

fn las() -> AccessMode {
    unimplemented!()
}

fn lax() -> AccessMode {
    unimplemented!()
}

fn ahx() -> AccessMode {
    unimplemented!()
}

fn sax() -> AccessMode {
    unimplemented!()
}

fn xaa() -> AccessMode {
    unimplemented!()
}

fn shx() -> AccessMode {
    unimplemented!()
}

fn rra() -> AccessMode {
    unimplemented!()
}

fn tas() -> AccessMode {
    unimplemented!()
}

fn shy() -> AccessMode {
    unimplemented!()
}

fn arr() -> AccessMode {
    unimplemented!()
}

fn sre() -> AccessMode {
    unimplemented!()
}

fn alr() -> AccessMode {
    unimplemented!()
}

fn rla() -> AccessMode {
    unimplemented!()
}

fn anc() -> AccessMode {
    unimplemented!()
}

fn slo() -> AccessMode {
    unimplemented!()
}
