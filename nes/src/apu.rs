mod timer {
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct State {
        divider: u16
    }

    impl State {
        pub fn new() -> Self {
            State { divider: 0 }
        }
    }

    pub trait Context: Sized {
        fn state(&self) -> &State;
        fn state_mut(&mut self) -> &mut State;
        fn on_timer_clock(&mut self);
        fn period(&self) -> u16;
    }

    pub trait Interface: Sized + Context {
        fn tick(&mut self) {
            if self.state().divider > 0 {
                self.state_mut().divider -= 1;
            } else {
                self.state_mut().divider = self.period() + 1;
                self.on_timer_clock();
            }
        }
    }

    impl<T: Context> Interface for T {}
}

use serde::{Deserialize, Serialize};

type ChannelRegister = [u8; 4];

const LENGTH_TABLE: [u8; 32] = [
    0x0A, 0xFE, 0x14, 0x02, 0x28, 0x04, 0x50, 0x06, 
    0xA0, 0x08, 0x3C, 0x0A, 0x0E, 0x0C, 0x1A, 0x0E,
    0x0C, 0x10, 0x18, 0x12, 0x30, 0x14, 0x60, 0x16, 
    0xC0, 0x18, 0x48, 0x1A, 0x10, 0x1C, 0x20, 0x1E,
];

const PLUSE_SEQUENCES: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [0, 0, 0, 0, 0, 0, 1, 1],
    [0, 0, 0, 0, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 0, 0],
];

const TRIANGLE_SEQUENCE: [u8; 32] = [
    0xF, 0xE, 0xD, 0xC, 0xB, 0xA, 0x9, 0x8, 0x7, 0x6, 0x5, 0x4, 0x3, 0x2, 0x1, 0x0, 
    0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF,
];

const RATE_NTSC: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

const NOISE_CHANNEL_NTSC_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

#[derive(Serialize, Deserialize)]
struct Envelope {
    decay: u8,
    divider: u8,
    reload_flag: bool,
    loop_flag: bool,
    period: u8,
    constant_volume_flag: bool,
}

impl Envelope {
    pub fn new() -> Self {
        Envelope { decay: 0, divider: 0, reload_flag: false, loop_flag: false, constant_volume_flag: false, period: 0 }
    }

    pub fn reload(&mut self, loop_flag: bool, constant_volume_flag: bool, period: u8) {
        self.loop_flag = loop_flag;
        self.constant_volume_flag = constant_volume_flag;
        self.period = period;
        self.reload_flag = true;
    }

    pub fn tick(&mut self) {
        if self.reload_flag {
            self.divider = self.period + 1;
            self.decay = 15;
            self.reload_flag = false;
        } else if self.divider == 0 {
            self.divider = self.period + 1;
            if self.decay > 0 {
                self.decay -= 1;
            } else if self.decay == 0 && self.loop_flag == true {
                self.decay = 15;
            }
        } else {
            self.divider -= 1;
        }
    }

    pub fn output(&self) -> u8 {
        if self.constant_volume_flag == true {
            self.period
        } else {
            self.decay
        }
    }
}

#[derive(Serialize, Deserialize)]
struct LengthCounter {
    divider: u8,
    enable: bool,
    halt_flag: bool
}

impl LengthCounter {
    pub fn new() -> Self {
        LengthCounter { divider: 0, enable: false, halt_flag: false }
    }

    pub fn set_halt(&mut self, halt_flag: bool) {
        self.halt_flag = halt_flag;
    }

    pub fn tick(&mut self) {
        if self.divider > 0 && !self.halt_flag {
            self.divider -= 1;
        }
    }

    pub fn turn_off(&mut self) {
        self.divider = 0;
        self.enable = false;
    }

    pub fn turn_on(&mut self) {
        self.enable = true;
    }

    pub fn reload(&mut self, index: u8) {
        if self.enable {
            self.divider = LENGTH_TABLE[index as usize] + 1;
        }
    }

    pub fn output(&self) -> u8 {
        self.divider
    }
}

#[derive(Serialize, Deserialize)]
pub struct PulseChannel {
    register: ChannelRegister,
    envelope: Envelope,
    timer: timer::State,
    length_counter: LengthCounter,
    is_first_channel: bool,
    sequence_index: usize,
    sweep_divider: u8,
    sweep_reload_flag: bool,
}

impl timer::Context for PulseChannel {
    fn state(&self) -> &timer::State {
        &self.timer
    }

    fn state_mut(&mut self) -> &mut timer::State {
        &mut self.timer
    }

    fn on_timer_clock(&mut self) {
        if self.sequence_index == 0 {
            self.sequence_index = 7;
        } else {
            self.sequence_index -= 1;
        }
    }

    fn period(&self) -> u16 {
        self.reg_timer()
    }
}

impl PulseChannel {
    pub fn new(is_first_channel: bool) -> Self {
        PulseChannel {
            register: [0, 0, 0, 0],
            envelope: Envelope::new(),
            timer: timer::State::new(),
            length_counter: LengthCounter::new(),
            is_first_channel,
            sequence_index: 0,
            sweep_divider: 0,
            sweep_reload_flag: false,
        }
    }

    pub fn reg_duty(&self) -> u8 {
        self.register[0] >> 6
    }

    pub fn reg_envelope_loop_flag(&self) -> bool {
        self.register[0] & 0b0010_0000 != 0
    }

    pub fn reg_constant_volume_flag(&self) -> bool {
        self.register[0] & 0b0001_0000 != 0
    }

    pub fn reg_envelope_period(&self) -> u8 {
        self.register[0] & 0b0000_1111
    }

    pub fn reg_sweep_enabled(&self) -> bool {
        self.register[1] & 0b1000_0000 != 0
    }

    pub fn reg_sweep_period(&self) -> u8 {
        (self.register[1] & 0b0111_0000) >> 4
    }

    pub fn reg_sweep_negate(&self) -> bool {
        self.register[1] & 0b0000_1000 != 0
    }

    pub fn reg_sweep_shift(&self) -> u8 {
        self.register[1] & 0b0000_0111
    }

    pub fn reg_timer(&self) -> u16 {
        (((self.register[3] & 0b0000_0111) as u16) << 8) | (self.register[2] as u16)
    }

    pub fn reg_length_index(&self) -> u8 {
        self.register[3] >> 3
    }

    pub fn set_register(&mut self, addr: u16, value: u8) {
        let selector = (addr & 0b11) as usize;
        self.register[selector] = value;
        match selector {
            0 => {
                self.envelope.reload(self.reg_envelope_loop_flag(), self.reg_constant_volume_flag(), self.reg_envelope_period());
                self.length_counter.set_halt(self.reg_envelope_loop_flag());
            }
            1 => {
                self.sweep_reload_flag = true;
            }
            3 => {
                self.length_counter.reload(self.reg_length_index());
                self.sequence_index = 0;
            }
            _ => {}
        }
    }

    pub fn set_enabled(&mut self, enable: bool) {
        if enable {
            self.length_counter.turn_on();
        } else {
            self.length_counter.turn_off();
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.length_counter.output() != 0
    }

    pub fn on_quarter_frame_clock(&mut self) {
        self.envelope.tick();
    }

    pub fn on_half_frame_clock(&mut self) {
        self.sweep_tick();
        self.length_counter.tick();
    }

    pub fn output(&self) -> u8 {
        let output = self.envelope.output();
        if self.is_silent() {
            0
        } else {
            output
        }
    }

    pub fn tick(&mut self) {
        timer::Interface::tick(self);
    }

    fn is_silent(&self) -> bool {
        !self.is_enabled() || self.sequence_output() == 0 || (self.sweep_target_period() > 0x7FF && self.reg_sweep_enabled())
    }

    fn set_reg_timer(&mut self, period: u16) {
        self.register[2] = period as u8;
        self.register[3] = self.register[3] & 0b1111_1000 | ((period >> 8 & 0b0000_0111) as u8);
    } 

    fn sweep_target_period(&self) -> u16 {
        let old_timer = self.reg_timer();
        let change = old_timer >> self.reg_sweep_shift();
        if self.reg_sweep_negate() {
            if self.is_first_channel {
                old_timer.wrapping_sub(change).wrapping_sub(1)
            }
            else {
                old_timer.wrapping_sub(change)
            }
            
        } else {
            old_timer.wrapping_add(change)
        }
    }

    fn sweep_tick(&mut self) {
        let target_period = self.sweep_target_period();
        let muting = self.reg_timer() < 8 || target_period > 0x7FF;
        if self.sweep_divider == 0 && self.reg_sweep_enabled() && !muting {
            self.set_reg_timer(target_period);
        }

        if self.sweep_divider == 0 || self.sweep_reload_flag == true {
            self.sweep_divider = self.reg_sweep_period() + 1;
            self.sweep_reload_flag = false;
        } else {
            self.sweep_divider -= 1;
        }
    }

    fn sequence_output(&self) -> u8 {
        PLUSE_SEQUENCES[self.reg_duty() as usize][self.sequence_index]
    }
}

#[derive(Serialize, Deserialize)]
pub struct TriangleChannel {
    register: ChannelRegister,
    timer: timer::State,
    length_counter: LengthCounter,
    linear_counter_divider: u8,
    linear_counter_reload_flag: bool,
    sequence_index: usize,
}

impl timer::Context for TriangleChannel {
    fn state(&self) -> &timer::State {
        &self.timer
    }

    fn state_mut(&mut self) -> &mut timer::State {
        &mut self.timer
    }

    fn on_timer_clock(&mut self) {
        if self.length_counter.output() > 0 && self.linear_counter_divider > 0 {
            self.sequence_index += 1;
            if self.sequence_index >= 32 {
                self.sequence_index = 0;
            }
        }
    }

    fn period(&self) -> u16 {
        self.reg_timer()
    }
}

impl TriangleChannel {
    pub fn new() -> Self {
        TriangleChannel {
            register: [0, 0, 0, 0],
            timer: timer::State::new(),
            length_counter: LengthCounter::new(),
            linear_counter_divider: 0,
            linear_counter_reload_flag: false,
            sequence_index: 0,
        }
    }

    pub fn reg_control_flag(&self) -> bool {
        self.register[0] & 0b1000_0000 != 0
    }

    pub fn reg_linear_counter(&self) -> u8 {
        self.register[0] & 0b0111_1111
    }

    pub fn reg_timer(&self) -> u16 {
        (((self.register[3] & 0b0000_0111) as u16) << 8) | (self.register[2] as u16)
    }

    pub fn reg_length_index(&self) -> u8 {
        self.register[3] >> 3
    }

    pub fn set_register(&mut self, addr: u16, value: u8) {
        let selector = (addr & 0b11) as usize;
        self.register[selector] = value;
        match selector & 0b11 {
            0 => {
                self.length_counter.set_halt(self.reg_control_flag());
            }
            3 => {
                self.linear_counter_reload_flag = true;
                self.length_counter.reload(self.reg_length_index());
            }
            _ => {}
        }
    }

    pub fn set_enabled(&mut self, enable: bool) {
        if enable {
            self.length_counter.turn_on();
        } else {
            self.length_counter.turn_off();
        }
    }

    pub fn on_quarter_frame_clock(&mut self) {
        self.linear_counter_tick();
    }

    pub fn on_half_frame_clock(&mut self) {
        self.length_counter.tick();
    }

    pub fn output(&self) -> u8 {
        if self.reg_timer() < 2 {
            7
        } else {
            self.sequence_output()
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.length_counter.output() > 0
    }

    pub fn tick(&mut self) {
        timer::Interface::tick(self);
    }

    fn sequence_output(&self) -> u8 {
        TRIANGLE_SEQUENCE[self.sequence_index]
    }

    fn linear_counter_tick(&mut self) {
        if self.linear_counter_reload_flag {
            self.linear_counter_divider = self.reg_linear_counter();
        } else if self.linear_counter_divider > 0 {
            self.linear_counter_divider -= 1;
        }
        if !self.reg_control_flag() {
            self.linear_counter_reload_flag = false;
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct NoiseChannel {
    register: ChannelRegister,
    timer: timer::State,
    envelope: Envelope,
    length_counter: LengthCounter,
    feedback_register: u16,
}

impl timer::Context for NoiseChannel {
    fn state(&self) -> &timer::State {
        &self.timer
    }

    fn state_mut(&mut self) -> &mut timer::State {
        &mut self.timer
    }

    fn on_timer_clock(&mut self) {
        let bit_a = self.feedback_register & 1;
        let bit_b = if self.reg_loop_noise_flag() {
            (self.feedback_register >> 6) & 1
        } else {
            (self.feedback_register >> 1) & 1
        };

        self.feedback_register = (self.feedback_register >> 1) | ((bit_a ^ bit_b) << 14);
    }

    fn period(&self) -> u16 {
        NOISE_CHANNEL_NTSC_PERIOD_TABLE[self.reg_noise_period_index() as usize]
    }
}

impl NoiseChannel {
    pub fn new() -> Self {
        NoiseChannel {
            register: [0, 0, 0, 0],
            timer: timer::State::new(),
            envelope: Envelope::new(),
            length_counter: LengthCounter::new(),
            feedback_register: 0b0000_0001,
        }
    }

    pub fn reg_envelope_loop_flag(&self) -> bool {
        self.register[0] & 0b0010_0000 != 0
    }

    pub fn reg_constant_volume_flag(&self) -> bool {
        self.register[0] & 0b0001_0000 != 0
    }

    pub fn reg_envelope_period(&self) -> u8 {
        self.register[0] & 0b0000_1111
    }

    pub fn reg_loop_noise_flag(&self) -> bool {
        self.register[2] & 0b1000_0000 != 0
    }

    pub fn reg_noise_period_index(&self) -> u8 {
        self.register[2] & 0b0000_1111
    }

    pub fn reg_length_index(&self) -> u8 {
        self.register[3] >> 3
    }

    pub fn set_register(&mut self, addr: u16, value: u8) {
        let selector = (addr & 0b11) as usize;
        self.register[selector] = value;
        match selector {
            0 => {
                self.envelope.reload(self.reg_envelope_loop_flag(), self.reg_constant_volume_flag(), self.reg_envelope_period());
                self.length_counter.set_halt(self.reg_envelope_loop_flag());
            }
            3 => {
                self.length_counter.reload(self.reg_length_index());
            }
            _ => {}
        }
    }

    pub fn is_silent(&self) -> bool {
        !self.is_enabled() || (self.feedback_register & 1) == 1
    }

    pub fn set_enabled(&mut self, enable: bool) {
        if enable {
            self.length_counter.turn_on();
        } else {
            self.length_counter.turn_off();
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.length_counter.output() > 0
    }

    pub fn on_quarter_frame_clock(&mut self) {
        self.envelope.tick();
    }

    pub fn on_half_frame_clock(&mut self) {
        self.length_counter.tick();
    }

    pub fn output(&self) -> u8 {
        if self.is_silent() {
            0
        } else {
            self.envelope.output()
        }
    }

    pub fn tick(&mut self) {
        timer::Interface::tick(self);
    }
}

#[derive(Serialize, Deserialize)]
pub struct DeltaModulationChannel {
    register: ChannelRegister,
    enable: bool,
    timer: timer::State,
    sample_current_address: u16,
    sample_remaining_bytes: u8,
    sample_buffer: Option<u8>,
    sample_shifter: u8,
    sample_shifter_remaining_bits: u8,
    output: u8,
    silence_flag: bool,
    interrupt_flag: bool,
}

impl timer::Context for DeltaModulationChannel {
    fn state(&self) -> &timer::State {
        &self.timer
    }

    fn state_mut(&mut self) -> &mut timer::State {
        &mut self.timer
    }

    fn on_timer_clock(&mut self) {
        if self.sample_shifter_remaining_bits > 0 && !self.silence_flag {
            let bit = self.sample_shifter & 1;
            if bit == 1 && self.output <= 125 {
                self.output += 2;
            } else if bit == 0 && self.output >= 2 {
                self.output -= 2;
            }
            self.sample_shifter >>= 1;
            self.sample_shifter_remaining_bits -= 1;
        } else {
            self.sample_shifter_remaining_bits = 8;
            if let Some(sample) = self.sample_buffer.take() {
                self.silence_flag = false;
                self.sample_shifter = sample;
            } else {
                self.silence_flag = true;
            }
        }
    }

    fn period(&self) -> u16 {
        RATE_NTSC[self.reg_rate_index()] >> 1 - 1
    }
}

impl DeltaModulationChannel {
    pub fn new() -> Self {
        DeltaModulationChannel {
            register: [0, 0, 0, 0],
            enable: false,
            timer: timer::State::new(),
            sample_current_address: 0,
            sample_remaining_bytes: 0,
            sample_shifter_remaining_bits: 0,
            sample_buffer: None,
            sample_shifter: 0,
            output: 0,
            silence_flag: false,
            interrupt_flag: false,
        }
    }

    pub fn reg_irq_enabled(&self) -> bool {
        self.register[0] & 0b1000_0000 != 0
    }

    pub fn reg_loop_flag(&self) -> bool {
        self.register[0] & 0b0100_0000 != 0
    }

    pub fn reg_rate_index(&self) -> usize {
        (self.register[0] & 0b0000_1111) as usize
    }

    pub fn reg_direct_load(&self) -> u8 {
        self.register[1] & 0b0111_1111
    }

    pub fn reg_sample_address(&self) -> u8 {
        self.register[2]
    }

    pub fn reg_sample_length(&self) -> u8 {
        self.register[3]
    }

    pub fn set_register(&mut self, addr: u16, value: u8) {
        let selector = (addr & 0b11) as usize;
        self.register[selector] = value;
        match selector {
            0 => {
                if !self.reg_irq_enabled() {
                    self.interrupt_flag = false;
                }
            }
            1 => {
                self.output = self.reg_direct_load();
            }
            _ => {}
        }
    }

    pub fn set_enabled(&mut self, enable: bool) {
        self.enable = enable;
        self.interrupt_flag = false;
        if enable && self.sample_remaining_bytes == 0 {
            self.sample_reader_init();
        } else {
            self.sample_remaining_bytes = 0;
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.sample_remaining_bytes != 0
    }

    pub fn output(&self) -> u8 {
        if self.enable {
            self.output
        } else {
            0
        }
    }

    pub fn on_dma_data_transfer(&mut self, value: u8) {
        self.sample_buffer = Some(value);
        if self.sample_current_address == 0xFFFF {
            self.sample_current_address = 0x8000;
        } else {
            self.sample_current_address += 1;
        }

        if self.sample_remaining_bytes > 0 {
            self.sample_remaining_bytes -= 1;
            if self.sample_remaining_bytes == 0 && self.reg_loop_flag() {
                self.sample_reader_init();
            } else if self.sample_remaining_bytes == 0 && self.reg_irq_enabled() {
                self.interrupt_flag = true;
            }
        }
    }

    pub fn should_activate_dma(&self) -> bool {
        if self.sample_buffer.is_none() && self.sample_remaining_bytes > 0 {
            true
        } else {
            false
        }
    }

    pub fn tick(&mut self) {
        timer::Interface::tick(self);
    }

    fn sample_reader_init(&mut self) {
        self.sample_current_address = (self.reg_sample_address() as u16 * 64) + 0xC000;
        self.sample_remaining_bytes = self.reg_sample_length() * 16 + 1;
    }
}

#[derive(Serialize, Deserialize)]
pub struct FrameRegister(u8);
impl FrameRegister {
    pub fn new() -> Self {
        FrameRegister(0)
    }

    pub fn is_5_step(&self) -> bool {
        self.0 & 0b1000_0000 != 0
    }

    pub fn interrupt_inhibit_flag(&self) -> bool {
        self.0 & 0b0100_0000 != 0
    }

    pub fn set_value(&mut self, value: u8) {
        self.0 = value;
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub pulse1: PulseChannel,
    pub pulse2: PulseChannel,
    pub triangle: TriangleChannel,
    pub noise: NoiseChannel,
    pub dmc: DeltaModulationChannel,
    pub frame: FrameRegister,
    pub frame_counter_timer: usize,
    pub timer_reset_flag: bool,
    pub timer_reset_countdown: usize,
    pub frame_interrupt_flag: bool,
    pub sample_counter: f64,
}

impl State {
    pub fn new() -> Self {
        State {
            pulse1: PulseChannel::new(true),
            pulse2: PulseChannel::new(false),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DeltaModulationChannel::new(),
            frame: FrameRegister::new(),
            frame_counter_timer: 0,
            timer_reset_flag: false,
            timer_reset_countdown: 0,
            frame_interrupt_flag: false,
            sample_counter: 0.0,
        }
    }
}

pub trait Context: Sized {
    fn state(&self) -> &State;
    fn state_mut(&mut self) -> &mut State;
    fn set_irq(&mut self, irq_enable: bool);
    fn activate_dma(&mut self, addr: u16);
    fn on_sample(&mut self, sample: f32);
    fn is_on_odd_cpu_cycle(&mut self) -> bool;
}

pub trait Interface: Sized + Context {
    fn on_cpu_tick(&mut self) {
        Private::on_cpu_tick(self);
    }

    fn set_pulse1(&mut self, addr: u16, value: u8) {
        self.state_mut().pulse1.set_register(addr, value);
    }

    fn set_pulse2(&mut self, addr: u16, value: u8) {
        self.state_mut().pulse2.set_register(addr, value);
    }

    fn set_triangle(&mut self, addr: u16, value: u8) {
        self.state_mut().triangle.set_register(addr, value);
    }

    fn set_noise(&mut self, addr: u16, value: u8) {
        self.state_mut().noise.set_register(addr, value);
    }

    fn set_dmc(&mut self, addr: u16, value: u8) {
        self.state_mut().dmc.set_register(addr, value);
    }

    fn set_frame(&mut self, value: u8) {
        self.state_mut().frame.set_value(value);
        if self.state().frame.interrupt_inhibit_flag() {
            self.set_frame_interrupt(false);
            Private::update_irq_line(self);
        }
        self.state_mut().timer_reset_flag = true;
        self.state_mut().timer_reset_countdown = if Context::is_on_odd_cpu_cycle(self) {
            3
        } else {
            4
        };
        if self.state().frame.is_5_step() {
            Private::quarter_frame_clock(self);
            Private::half_frame_clock(self);
        }
    }

    fn write_state_register(&mut self, value: u8) {
        self.state_mut()
            .pulse1
            .set_enabled(value & 0b0000_0001 != 0);
        self.state_mut()
            .pulse2
            .set_enabled(value & 0b0000_0010 != 0);
        self.state_mut()
            .triangle
            .set_enabled(value & 0b0000_0100 != 0);
        self.state_mut().noise.set_enabled(value & 0b0000_1000 != 0);
        self.state_mut().dmc.set_enabled(value & 0b0001_0000 != 0);
        Private::update_irq_line(self);
    }

    fn read_state_register(&mut self) -> u8 {
        let mut value: u8 = 0;
        if self.state().pulse1.is_enabled() {
            value |= 0b0000_0001;
        }
        if self.state().pulse2.is_enabled() {
            value |= 0b0000_0010;
        }
        if self.state().triangle.is_enabled() {
            value |= 0b0000_0100;
        }
        if self.state().noise.is_enabled() {
            value |= 0b0000_1000;
        }
        if self.state().dmc.is_enabled() {
            value |= 0b0001_0000;
        }
        if self.state().frame_interrupt_flag {
            value |= 0b0100_0000;
        }
        if self.state().dmc.interrupt_flag {
            value |= 0b1000_0000;
        }
        Private::set_frame_interrupt(self, false);
        self.update_irq_line();
        value
    }

    fn on_dma_finish(&mut self, value: u8) {
        self.state_mut().dmc.on_dma_data_transfer(value);
    }

    fn mixer_output(&self) -> f32 {
        Private::mixer_output(self)
    }
}

impl<T: Context> Interface for T {}
impl<T: Context> Private for T {}

trait Private: Sized + Context {
    fn on_cpu_tick(&mut self) {
        self.state_mut().triangle.tick();
        if !Context::is_on_odd_cpu_cycle(self) {
            self.state_mut().pulse1.tick();
            self.state_mut().pulse2.tick();
            self.state_mut().noise.tick();
            self.state_mut().dmc.tick();
            if self.state().dmc.should_activate_dma() {
                self.activate_dma(self.state().dmc.sample_current_address);
            }
        }

        self.output_clock();

        if self.state().timer_reset_flag {
            if self.state().timer_reset_countdown == 0 {
                self.state_mut().timer_reset_flag = false;
                self.state_mut().frame_counter_timer = 1;
            } else {
                self.state_mut().timer_reset_countdown -= 1;
            }
        }

        // TODO: add PAL support
        match self.state().frame_counter_timer {
            7457 => {
                Private::quarter_frame_clock(self);
            }
            14913 => {
                Private::quarter_frame_clock(self);
                Private::half_frame_clock(self);
            }
            22371 => {
                Private::quarter_frame_clock(self);
            }
            29828 => {
                if !self.state().frame.is_5_step() {
                    Private::set_frame_interrupt(self, true);
                }
            }
            29829 => {
                if !self.state().frame.is_5_step() {
                    Private::quarter_frame_clock(self);
                    Private::half_frame_clock(self);
                    Private::set_frame_interrupt(self, true);
                }
            }
            29830 => {
                if !self.state().frame.is_5_step() {
                    self.state_mut().frame_counter_timer = 0;
                    Private::set_frame_interrupt(self, true);
                }
            }
            37281 => {
                if self.state().frame.is_5_step() {
                    Private::quarter_frame_clock(self);
                    Private::half_frame_clock(self);
                }
            }
            37282 => {
                if self.state().frame.is_5_step() {
                    self.state_mut().frame_counter_timer = 0;
                }
            }
            _ => {}
        }
        self.state_mut().frame_counter_timer += 1;
        self.update_irq_line();
    }

    fn update_irq_line(&mut self) {
        Context::set_irq(
            self,
            self.state().frame_interrupt_flag || self.state().dmc.interrupt_flag,
        );
    }

    fn set_frame_interrupt(&mut self, enable: bool) {
        if enable && !self.state().frame.interrupt_inhibit_flag() {
            self.state_mut().frame_interrupt_flag = true;
        } else if !enable {
            self.state_mut().frame_interrupt_flag = false;
        }
    }

    fn quarter_frame_clock(&mut self) {
        self.state_mut().pulse1.on_quarter_frame_clock();
        self.state_mut().pulse2.on_quarter_frame_clock();
        self.state_mut().triangle.on_quarter_frame_clock();
        self.state_mut().noise.on_quarter_frame_clock();
    }

    fn half_frame_clock(&mut self) {
        self.state_mut().pulse1.on_half_frame_clock();
        self.state_mut().pulse2.on_half_frame_clock();
        self.state_mut().triangle.on_half_frame_clock();
        self.state_mut().noise.on_half_frame_clock();
    }

    fn mixer_output(&self) -> f32 {
        let pulse1_sample = self.state().pulse1.output() as f32;
        let pulse2_sample = self.state().pulse2.output() as f32;
        let triangle_sample = self.state().triangle.output() as f32;
        let noise_sample = self.state().noise.output() as f32;
        let dmc_sample = self.state().dmc.output() as f32;

        let pulse_out = if pulse1_sample > 0.0 || pulse2_sample > 0.0 {
            95.88 / (8128.0 / (pulse1_sample + pulse2_sample) + 100.0)
        } else {
            0.0
        };

        let tnd_out = if triangle_sample > 0.0 || noise_sample > 0.0 || dmc_sample > 0.0 {
            159.79
                / ((1.0
                    / (triangle_sample / 8227.0 + noise_sample / 12241.0 + dmc_sample / 22638.0))
                    + 100.0)
        } else {
            0.0
        };

        pulse_out + tnd_out
    }

    fn output_clock(&mut self) {
        let sample_rate = 44.1;
        let cpu_frequence = 21477.272 / 12.0;
        let adjust = 1.9;  // experienced parameter
        let sample_every = cpu_frequence / sample_rate - adjust;
        if self.state().sample_counter > sample_every {
            self.state_mut().sample_counter -= sample_every;
            let sample = self.mixer_output();
            self.on_sample(sample);
        } else {
            self.state_mut().sample_counter += 1.0;
        }
    }
}
