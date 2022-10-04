use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct State {
    ppu_dma_request: Option<u16>,
    dmc_dma_request: Option<u16>,
    dmc_dma_halt_cycle: u8,
}

impl State {
    pub fn new() -> Self {
        State {
            ppu_dma_request: None,
            dmc_dma_request: None,
            dmc_dma_halt_cycle: 2,
        }
    }
}

pub trait Context: Sized {
    fn state(&mut self) -> &State;
    fn state_mut(&mut self) -> &mut State;
    fn peek_memory(&mut self, addr: u16) -> u8;
    fn is_odd_cpu_cycle(&self) -> bool;
    fn on_dmc_dma_transfer(&mut self, value: u8);
    fn on_ppu_dma_transfer(&mut self, value: u8, offset: usize);
}

pub trait Interface: Sized + Context {
    fn on_cpu_tick(&mut self) {
        if self.state().dmc_dma_request.is_some() && self.state().dmc_dma_halt_cycle > 0 {
            self.state_mut().dmc_dma_halt_cycle -= 1;
        }
    }

    fn dma_hijack(&mut self, cpu_peek_addr: u16) {
        Private::dma_hijack(self, cpu_peek_addr);
    }

    fn activate_ppu_dma(&mut self, data: u8) {
        let addr = (data as u16) << 8;
        self.state_mut().ppu_dma_request = Some(addr);
    }

    fn activate_dmc_dma(&mut self, addr: u16) {
        self.state_mut().dmc_dma_request = Some(addr);
    }
}

impl<T: Context> Interface for T {}
impl<T: Context> Private for T {}

trait Private: Sized + Context {
    fn dma_hijack(&mut self, cpu_peek_addr: u16) {
        self.state_mut().dmc_dma_halt_cycle = 2;
        if self.state().dmc_dma_request.is_some() || self.state().ppu_dma_request.is_some() {
            self.peek_memory(cpu_peek_addr);
            let mut ppu_dma_data_cache = None;
            let mut ppu_dma_data_offset = 0;
            loop {
                let dmc_data_transfer_ready = self.state().dmc_dma_halt_cycle == 0;
                match (
                    self.is_odd_cpu_cycle(),
                    self.state().dmc_dma_request,
                    dmc_data_transfer_ready,
                    self.state().ppu_dma_request,
                    ppu_dma_data_cache,
                ) {
                    (true, Some(addr), true, _, _) => {
                        // dmc read
                        let value = self.peek_memory(addr);
                        self.on_dmc_dma_transfer(value);
                        self.state_mut().dmc_dma_request = None;
                    }
                    (true, None, _, Some(addr), None) => {
                        // sprite read
                        let addr = addr + ppu_dma_data_offset as u16;
                        ppu_dma_data_cache = Some(self.peek_memory(addr));
                    }
                    (false, _, _, Some(_), Some(value)) => {
                        // sprite write
                        self.on_ppu_dma_transfer(value, ppu_dma_data_offset);
                        ppu_dma_data_cache = None;
                        ppu_dma_data_offset += 1;
                        if ppu_dma_data_offset >= 256 {
                            self.state_mut().ppu_dma_request = None;
                        }
                    }
                    (_, None, _, None, _) => {
                        break;
                    }
                    _ => {
                        self.peek_memory(cpu_peek_addr);
                    }
                }
            }
        }
    }
}
