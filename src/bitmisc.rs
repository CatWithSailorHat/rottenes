pub trait U16Address {
    fn fetch_lo(&self) -> u8;
    fn fetch_hi(&self) -> u8;
    fn set_lo(&mut self, value: u8);
    fn set_hi(&mut self, value: u8);
}

impl U16Address for u16 {
    fn fetch_lo(&self) -> u8 {
        *self as u8
    }

    fn fetch_hi(&self) -> u8 {
        (*self >> 8) as u8
    }

    fn set_lo(&mut self, value: u8) {
        *self = (*self & 0b1111_1111_0000_0000) | value as u16;
    }

    fn set_hi(&mut self, value: u8) {
        *self = (*self & 0b0000_0000_1111_1111) | ((value as u16) << 8);
    }
}

pub trait U8BitTest {
    fn is_b7_set(&self) -> bool;
    fn is_b6_set(&self) -> bool;
    fn is_b5_set(&self) -> bool;
    fn is_b4_set(&self) -> bool;
    fn is_b0_set(&self) -> bool;
}

impl U8BitTest for u8 {
    fn is_b7_set(&self) -> bool {
        *self & 0b1000_0000 != 0
    }

    fn is_b6_set(&self) -> bool {
        *self & 0b0100_0000 != 0
    }

    fn is_b5_set(&self) -> bool {
        *self & 0b0010_0000 != 0
    }

    fn is_b4_set(&self) -> bool {
        *self & 0b0001_0000 != 0
    }

    fn is_b0_set(&self) -> bool {
        *self & 0b0000_0001 != 0
    }
}