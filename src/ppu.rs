// #![allow(dead_code)]
use super::bitmisc::{ U16Address, U8BitTest };
use serde::{Serialize, Deserialize};

pub const SCREEN_SIZE: usize = 256 * 240;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn new(r: u8, g: u8, b:u8) -> Self {
        RgbColor{ r, g, b }
    }

    pub fn default() -> Self {
        RgbColor::new(0, 0, 0)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Palette(Vec<RgbColor>);
impl Palette {
    fn new(data: &[u8]) -> Self {
        assert!(data.len() == 64*3);
        let mut palette = [RgbColor::default(); 64];

        for (index, rgb) in data.chunks(3).enumerate() {
            palette[index].r = rgb[0];
            palette[index].g = rgb[1];
            palette[index].b = rgb[2];
        }
        Palette(palette.to_vec())
    }

    pub fn get_rgb(&self, index: usize) -> RgbColor {
        self.0[index]
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct PpuAddr(u16);
impl PpuAddr {
    // yyy NN YYYYY XXXXX
    // ||| || ||||| +++++-- coarse X scroll
    // ||| || +++++-------- coarse Y scroll
    // ||| ++-------------- nametable select
    // +++----------------- fine Y scroll
    pub fn new() -> Self {
        PpuAddr(0)
    }

    #[inline]
    pub fn max_corase_x() -> u16 {
        0b11111
    }

    #[inline]
    pub fn max_corase_y() -> u16 {
        0b11111
    }
    
    #[inline]
    pub fn max_corase_y_of_nametable() -> u16 {
        29
    }

    #[inline]
    pub fn max_fine_y() -> u16 {
        0b111
    }

    #[inline]
    pub fn get_corase_x(&self) -> u16 {
        self.0 & 0b0_000_00_00000_11111
    }

    #[inline]
    pub fn get_corase_y(&self) -> u16 {
        (self.0 & 0b0_000_00_11111_00000) >> 5
    }

    #[inline]
    pub fn get_fine_y(&self) -> u16 {
        (self.0 & 0b0_111_00_00000_00000) >> 12
    }

    #[inline]
    pub fn get_nn(&self) -> u16 {
        (self.0 & 0b0_000_11_00000_00000) >> 10
    }

    #[inline]
    pub fn get_tile_address(&self) -> u16 {
        0x2000 | (self.0 & 0b0_000_11_11111_11111)
    }

    #[inline]
    pub fn get_attribute_address(&self) -> u16 {
        0x23c0 | (self.get_nn() << 10) | ((self.get_corase_y() / 4) << 3) | (self.get_corase_x() / 4)
    }

    #[inline]
    fn mirror_nametable_horizontally(&mut self) {
        self.0 ^= 0b0_000_01_00000_00000
    }

    #[inline]
    fn mirror_nametable_vertically(&mut self) {
        self.0 ^= 0b0_000_10_00000_00000
    }

    #[inline]
    fn set_corase_x(&mut self, value: u16) {
        self.0 = (self.0 & !0b0_000_00_00000_11111) | (value & 0b11111)
    }

    #[inline]
    fn set_corase_y(&mut self, value: u16) {
        self.0 = (self.0 & !0b0_000_00_11111_00000) | ((value & 0b11111) << 5)
    }

    #[inline]
    fn set_fine_y(&mut self, value: u16) {
        self.0 = (self.0 & !0b0_111_00_00000_00000) | ((value & 0b111) << 12)
    }

    #[inline]
    fn set_nn(&mut self, value: u16) {
        self.0 = (self.0 & !0b0_000_11_00000_00000) | ((value & 0b11) << 10)
    }

    #[inline]
    fn set_low_byte(&mut self, value: u8) {
        self.0 = ((value & 0xff) as u16) | (self.0 & 0xff00)
    }

    #[inline]
    fn set_high_byte(&mut self, value: u8) {
        self.0 = (((value & 0x3f) as u16) << 8) | (self.0 & 0xff)
    }

    #[inline]
    fn increase_corase_x(&mut self) {
        let coarse_x = self.get_corase_x();
        if coarse_x == PpuAddr::max_corase_x() {
            self.set_corase_x(0);
            self.mirror_nametable_horizontally();
        }
        else {
            self.set_corase_x(coarse_x + 1);
        }
    }

    #[inline]
    fn increase_fine_y(&mut self) {
        let fine_y = self.get_fine_y();
        if fine_y != PpuAddr::max_fine_y() {
            self.set_fine_y(fine_y + 1);
        }
        else {
            self.set_fine_y(0);
            let mut coarse_y = self.get_corase_y();
            if coarse_y == PpuAddr::max_corase_y_of_nametable() {
                coarse_y = 0;
                self.mirror_nametable_vertically();
            }
            else if coarse_y == PpuAddr::max_corase_y() {
                coarse_y = 0;
            }
            else {
                coarse_y += 1;
            }
            self.set_corase_y(coarse_y)
        }
    }

    #[inline]
    fn copy_horizontal_postion_bits(&mut self, vaddr: Self) {
        self.0 = (self.0 & !0x041F) | (vaddr.0 & 0x041F)
    }

    #[inline]
    fn copy_vertical_postion_bits(&mut self, vaddr: Self) {
        self.0 = (self.0 & !0x7BE0) | (vaddr.0 & 0x7BE0)
    }
}

#[derive(Serialize, Deserialize)]
pub struct PCtrl(u8);
impl PCtrl {
    pub fn new(v: u8) -> Self {
        PCtrl(v)
    }

    #[inline]
    pub fn get_nn(&self) -> u16 {
        self.0 as u16 & 0b11
    }

    pub fn base_nametable_address(&self) -> u16 {
        match self.0 & 0b11 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2c00,
            _ => unreachable!()
        }
    }

    pub fn vram_addr_increment(&self) -> usize {
        match self.0 & (1 << 2) {
            0 => 0x1,
            _ => 0x20,
        }
    }

    pub fn pattern_table_addr_for_8x8_sprites(&self) -> u16 {
        match self.0 & (1 << 3) {
            0 => 0x0000,
            _ => 0x1000,
        }
    }

    pub fn bg_pattern_table_addr(&self) -> u16 {
        match self.0 & (1 << 4) {
            0 => 0x0000,
            _ => 0x1000,
        }
    }

    pub fn sprite_length(&self) -> usize {
        match self.0 & (1 << 5) {
            0 => 8,
            _ => 16,
        }
    }

    pub fn nmi_output(&self) -> bool {
        self.0 & (1 << 7) != 0
    }

    pub fn is_two_tile_sprite(&self) -> bool {
        self.0 & (1 << 5) != 0
    }
}

#[derive(Serialize, Deserialize)]
pub struct PMask(u8);
impl PMask {
    pub fn new(v: u8) -> Self {
        PMask(v)
    }

    pub fn greyscale_mode(&self) -> bool {
        self.0 & (1 << 0) != 0
    }

    pub fn show_background_in_leftmost_8_pixels(&self) -> bool {
        self.0 & (1 << 1) != 0
    }

    pub fn show_sprite_in_leftmost_8_pixels(&self) -> bool {
        self.0 & (1 << 2) != 0
    }

    pub fn show_background(&self) -> bool {
        self.0 & (1 << 3) != 0
    }

    pub fn show_sprites(&self) -> bool {
        self.0 & (1 << 4) != 0
    }

    pub fn emphasize_red(&self) -> bool {
        self.0 & (1 << 5) != 0
    }

    pub fn emphasize_green(&self) -> bool {
        self.0 & (1 << 6) != 0
    }

    pub fn emphasize_blue(&self) -> bool {
        self.0 & (1 << 7) != 0
    }

    pub fn emphasize_bits(&self) -> u8 {
        (self.0 >> 5) & 0b111
    }
}

#[derive(Serialize, Deserialize)]
pub struct PStatus(u8);
impl PStatus {
    pub fn new(v: u8) -> Self {
        PStatus(v)
    }

    pub fn sprite_overflow(&self) -> bool {
        self.0 & (1 << 5) != 0
    }

    pub fn set_sprite_overflow(&mut self, value: bool) {
        if value == true {
            self.0 = self.0 | (1 << 5);
        }
        else {
            self.0 = self.0 & (0xff ^ (1 << 5));
        }
    }

    pub fn sprite_0_hit(&self) -> bool {
        self.0 & (1 << 6) != 0
    }

    pub fn set_sprite_0_hit(&mut self, value: bool) {
        if value == true {
            self.0 |= 1 << 6;
        }
        else {
            self.0 &= 0xff ^ (1 << 6);
        }
    }

    pub fn vblank_occured(&self) -> bool {
        self.0 & (1 << 7) != 0
    }

    pub fn set_vblank_occured(&mut self, value: bool) {
        if value == true {
            self.0 |= 1 << 7;
        }
        else {
            self.0 &= 0xff ^ (1 << 7);
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SpriteEvaluationState {
    Idle, Copy, Search,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Sprite {
    pub x_pos: u8,
    pub y_pos: u8,
    pub hi_tile_shift: u8,
    pub lo_tile_shift: u8,
    pub attribute: u8,
    countdown: usize,
}

impl Sprite {
    pub fn new() -> Self {
        Sprite {
            x_pos: 0xff,
            y_pos: 0xff,
            hi_tile_shift: 0,
            lo_tile_shift: 0,
            attribute: 0,
            countdown: 0xff,
        }
    }

    fn set_pos(&mut self, x: u8, y: u8) {
        self.x_pos = x;
        self.y_pos = y;
        self.countdown = x as usize;
    }

    fn set_hi_tile_shift(&mut self, hi: u8) {
        self.hi_tile_shift = hi;
    }

    fn set_lo_tile_shift(&mut self, lo: u8) {
        self.lo_tile_shift = lo;
    }

    pub fn color_set_index(&self) -> u8 {
        (self.attribute & 0b11) + 4
    }

    pub fn behind_background(&self) -> bool {
        self.attribute.is_b5_set()
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    frame_buffer: Vec<RgbColor>,
    frame_buffer_cursor: usize,
    pub palette: Palette,

    n_dot: usize,
    n_scanline: usize,
    is_odd_frame: bool,
    pctrl: PCtrl,
    pmask: PMask,
    pstatus: PStatus,

    // pub name_table: [u8; 2048],

    pub oamaddr: usize,
    pub oamdata: Vec<u8>,
    primary_oam_cursor: usize,
    primary_oam_latch: u8,

    pub secondary_oam: [u8; 4 * 8],
    secondary_oam_cursor: usize,
    sprite_nums_on_next_scanline: usize,
    sprite_evaluation_state: SpriteEvaluationState,

    sprite_list: [Sprite; 8],
    sprite_list_cursor: usize,
    sprite_0_on_next_scanline: bool,
    sprite_0_on_current_scanline: bool,

    sprite_y_latch: u8,
    sprite_tile_addr_latch: u8,
    sprite_attribute_latch: u8,
    // sprite_x_latch: u8,

    pub palette_ram: [u8; 32],

    current_addr: PpuAddr,
    temporary_addr: PpuAddr,
    write_toggle: bool,
    fine_x: u8,

    address_latch: u16,

    tile_index_latch: u16,
    tile_lo_latch: u8,
    tile_hi_latch: u8,
    attribute_latch: u8,

    ppudata_latch: u8,

    background_shift_lo: u16,
    background_shift_hi: u16,
    attribute_shift_lo: u16,
    attribute_shift_hi: u16,

    nmi_already_triggered: bool,
    nmi_ready_to_trigger: bool,

    skip_one_tick: bool,

    vblank_suppress_flag: bool,

}

impl State {
    pub fn new() -> Self {
        let palette_bytes = include_bytes!("./palette.pal");
        State {
            frame_buffer: [RgbColor::new(0, 0, 0); SCREEN_SIZE].to_vec(),
            frame_buffer_cursor: 0,
            palette: Palette::new(palette_bytes),
            n_dot: 0,
            n_scanline: 261,
            pctrl: PCtrl::new(0),
            pmask: PMask::new(0),
            pstatus: PStatus::new(0),
            oamaddr: 0,
            oamdata: [0; 64 * 4].to_vec(),
            primary_oam_cursor: 0,
            primary_oam_latch: 0,
            is_odd_frame: false,
            secondary_oam: [0; 4 * 8],
            secondary_oam_cursor: 0,
            sprite_nums_on_next_scanline: 0,
            sprite_evaluation_state: SpriteEvaluationState::Idle,
            sprite_list: [Sprite::new(); 8],
            sprite_list_cursor: 0,
            sprite_0_on_next_scanline: false,
            sprite_0_on_current_scanline: false,
            sprite_y_latch: 0,
            sprite_tile_addr_latch: 0,
            sprite_attribute_latch: 0,
            palette_ram: [0; 32],
            current_addr: PpuAddr::new(),
            temporary_addr: PpuAddr::new(),
            write_toggle: false,
            fine_x: 0,
            address_latch: 0,
            tile_index_latch: 0,
            tile_lo_latch: 0,
            tile_hi_latch: 0,
            attribute_latch: 0,
            ppudata_latch: 0,
            background_shift_lo: 0,
            background_shift_hi: 0,
            attribute_shift_lo: 0,
            attribute_shift_hi: 0,
            nmi_already_triggered: false,
            skip_one_tick: false,
            vblank_suppress_flag: false,
            nmi_ready_to_trigger: false,
        }
    }
}

pub trait Context: Sized {
    fn peek_vram(&mut self, addr: u16) -> u8;
    fn poke_vram(&mut self, addr: u16, val: u8);
    fn state(&self) -> &State;
    fn state_mut(&mut self) -> &mut State;
    fn trigger_nmi(&mut self);
    fn generate_frame(&mut self);
}

pub trait Interface: Sized + Context {
    fn tick(&mut self) {
        Private::tick(self);
    }

    fn get_framebuffer(&self) -> &Vec<RgbColor> {
        &self.state().frame_buffer
    }

    fn write_ppuctrl(&mut self, value: u8) {
        Private::write_ppuctrl(self, value);
    }

    fn write_ppumask(&mut self, value: u8) {
        Private::write_ppumask(self, value);
    }

    fn read_ppustatus(&mut self) -> u8 {
        Private::read_ppustatus(self)
    }

    fn write_oamaddr(&mut self, value: u8) {
        Private::write_oamaddr(self, value);
    }

    fn read_oamdata(&mut self) -> u8 {
        Private::read_oamdata(self)
    }

    fn write_oamdata(&mut self, value: u8) {
        Private::write_oamdata(self, value);
    }

    fn write_ppuscroll(&mut self, value: u8) {
        Private::write_ppuscroll(self, value);
    }

    fn write_ppuaddr(&mut self, value: u8) {
        Private::write_ppuaddr(self, value);
    }

    fn read_ppudata(&mut self) -> u8 {
        Private::read_ppudata(self)
    }

    fn write_ppudata(&mut self, value: u8) {
        Private::write_ppudata(self, value);
    }
}

impl<T: Context> Private for T {}
impl<T: Context> Interface for T {}
trait Private: Sized + Context {
    fn tick(&mut self) {
        self.try_to_trigger_nmi();

        match (self.state().n_scanline, self.state().n_dot) {
            (0, 0) => {
                self.state_mut().sprite_0_on_current_scanline = self.state().sprite_0_on_next_scanline;
                self.state_mut().sprite_0_on_next_scanline = false;
                self.state_mut().secondary_oam_cursor = 0;
                if self.state().skip_one_tick {
                    self.state_mut().n_dot += 1;
                    self.state_mut().skip_one_tick = false;
                    self.draw_pixel();
                    self.prepare_render_data();
                }
            }
            (_, 0) => {
                self.state_mut().sprite_0_on_current_scanline = self.state().sprite_0_on_next_scanline;
                self.state_mut().sprite_0_on_next_scanline = false;
                self.state_mut().secondary_oam_cursor = 0;
            }
            (0..=239, 1..=256) => {
                self.draw_pixel();
                self.prepare_render_data();
            }
            (0..=239, _) => {
                self.prepare_render_data();
            }
            (241, 1) => {
                self.state_mut().frame_buffer_cursor = 0;
                if !self.state_mut().vblank_suppress_flag {
                    self.state_mut().pstatus.set_vblank_occured(true);
                }
                self.generate_frame();
            }
            (260, 340) => {
                self.state_mut().is_odd_frame = !self.state().is_odd_frame;
            }
            (261, 1) => {
                self.state_mut().pstatus.set_vblank_occured(false);
                self.state_mut().pstatus.set_sprite_overflow(false);
                self.state_mut().pstatus.set_sprite_0_hit(false);
                self.state_mut().nmi_already_triggered = false;
                self.prepare_render_data();
            }
            (261, _) => {
                self.prepare_render_data();
            }
            (_, _) => {}
        }

        match (self.state().n_scanline, self.state().n_dot) {
            (261, 340) => {
                self.state_mut().n_scanline = 0;
                self.state_mut().n_dot = 0;
                if self.state().is_odd_frame && self.state().pmask.show_background() {
                    self.state_mut().skip_one_tick = true;
                }
            }
            (_, 340) => {
                self.state_mut().n_scanline += 1;
                self.state_mut().n_dot = 0;
            }
            (_, _) => {
                self.state_mut().n_dot += 1;
            }
        }
        self.state_mut().vblank_suppress_flag = false;
    }

    fn prepare_render_data(&mut self) {
        let n_dot = self.state().n_dot;
        let n_scanline = self.state().n_scanline;

        // shift registers and sprite evaluation
        match n_dot {
            1 => {
                self.shift_sprite_registers();
                self.shift_background_registers();
                self.tick_clear_secondary_oam()
            }
            2..=64 => {
                self.shift_sprite_registers();
                self.shift_background_registers();
                self.tick_clear_secondary_oam() 
            }
            65 => {
                self.shift_sprite_registers();
                self.shift_background_registers();
                self.state_mut().sprite_evaluation_state = SpriteEvaluationState::Search;
                self.state_mut().secondary_oam_cursor = 0;
                self.state_mut().primary_oam_cursor = self.state().oamaddr;
                self.state_mut().sprite_nums_on_next_scanline = 0;
                self.tick_sprite_evaluation()
            }
            66..=256 => {
                self.shift_sprite_registers();
                self.shift_background_registers();
                self.tick_sprite_evaluation() 
            }
            321..=336 => {
                self.shift_background_registers();
            }
            _ => {}
        }

        // fetch tiles and set registers
        match n_dot {
            1 | 321 => {
                self.bg_latch_tile_index_addr();
            }
            2..=255 | 322..=336 => {
                match n_dot & 0b111 {
                    // nametable
                    1 => { self.bg_latch_tile_index_addr() }
                    2 => { self.bg_latch_tile_index() }
                    // attribute
                    3 => { self.bg_latch_attribute_addr() }
                    4 => { self.bg_latch_attribute() }
                    // background tile low bits
                    5 => { self.bg_latch_tile_lo_addr() }
                    6 => { self.bg_latch_tile_lo() }
                    // background tile high bits
                    7 => { self.bg_latch_tile_hi_addr() }
                    0 => { self.bg_latch_tile_hi(); self.h_scroll(); self.reload_background_registers() }
                    _ => unreachable!()
                }
            }
            256 => {
                self.bg_latch_tile_hi(); 
                self.reload_background_registers();
                self.h_scroll();
                self.v_scroll();
            }
            257 => {
                // self.state_mut().oamaddr = 0;
                self.h_update();
                self.state_mut().secondary_oam_cursor = 0;
                self.state_mut().sprite_list_cursor = 0;
                self.sp_latch_y();
            }
            258..=320 => {
                // self.state_mut().oamaddr = 0;
                match n_dot & 0b111 {
                    1 => { self.sp_latch_y() }
                    2 => { self.sp_latch_tile_addr() }
                    3 => { self.sp_latch_attribute() }
                    4 => { self.sp_set_position() }
                    5 => { self.sp_fetch_tile_lo_addr() }
                    6 => { self.sp_set_lo_shift() }
                    7 => { self.sp_fetch_tile_hi_addr() }
                    0 => { self.sp_set_hi_shift() }
                    _ => unreachable!()
                }
            }
            337 => { self.bg_latch_tile_index_addr() }
            338 => { self.bg_latch_tile_index() }
            339 => { self.bg_latch_tile_index_addr() }
            340 => { self.bg_latch_tile_index(); }
            _ => {}
        }
        if n_scanline == 261 && (280..=304).contains(&n_dot) {
            self.v_update()
        }
    }

    fn try_to_trigger_nmi(&mut self) {
        if self.state().pstatus.vblank_occured() && self.state().pctrl.nmi_output() {
            if !self.state().nmi_already_triggered {
                if self.state().nmi_ready_to_trigger {
                    self.trigger_nmi();
                    self.state_mut().nmi_ready_to_trigger = false;
                    self.state_mut().nmi_already_triggered = true;
                }
                else {
                    self.state_mut().nmi_ready_to_trigger = true;
                }
            }
        }
        else {
            self.state_mut().nmi_already_triggered = false;
        }
    }

    fn pixel_sprite(&self) -> (u8, u8, bool, bool) {
        if self.state().pmask.show_sprites() && (self.state().pmask.show_sprite_in_leftmost_8_pixels() || self.state().n_dot > 8) {
            for (nth, sprite) in self.state().sprite_list.iter().enumerate() {
                if sprite.countdown != 0 { continue; }

                let pattern_lo = (sprite.lo_tile_shift >> 7) & 1;
                let pattern_hi = (sprite.hi_tile_shift >> 7) & 1;

                if pattern_lo == 0 && pattern_hi == 0 { continue; }

                let color_index = pattern_lo | (pattern_hi << 1);
                let sp_behind_background = sprite.behind_background();
                let color_set_index = sprite.color_set_index();
                return (color_set_index, color_index, sp_behind_background, nth == 0)
            }
        }
        (0, 0, false, false)
    }

    fn pixel_background(&self) -> (u8, u8) {
        if self.state().pmask.show_background() && (self.state().pmask.show_background_in_leftmost_8_pixels() || self.state().n_dot > 8) {
            let shift = (7 - self.state().fine_x) + 8;
            let pattern_lo = (self.state().background_shift_lo >> shift) & 1;
            let pattern_hi = (self.state().background_shift_hi >> shift) & 1;
            let color_set_index_lo = (self.state().attribute_shift_lo >> shift) & 1;
            let color_set_index_hi = (self.state().attribute_shift_hi >> shift) & 1;

            let color_index = pattern_lo | (pattern_hi << 1);
            let color_set_index = color_set_index_lo | (color_set_index_hi << 1);

            (color_set_index as u8, color_index as u8)
        } else {
            (0, 0)
        }
    }

    fn draw_pixel(&mut self) {
        debug_assert!(self.state().frame_buffer_cursor < SCREEN_SIZE);

        let (sp_color_set_index, sp_color_index, sp_behind_background, is_sprite_0) = self.pixel_sprite();
        let (bg_color_set_index, bg_color_index) = self.pixel_background();
        
        if self.state().sprite_0_on_current_scanline && sp_color_index != 0 && bg_color_index != 0 && is_sprite_0 && self.state().n_dot != 256 {
            self.state_mut().pstatus.set_sprite_0_hit(true);
        }

        let palette_ram_index = match (bg_color_index, sp_color_index, sp_behind_background) {
            (0, 0, _) => 0,
            (0, _, _) => (sp_color_set_index << 2) | sp_color_index,
            (_, 0, _) => (bg_color_set_index << 2) | bg_color_index,
            (_, _, false) => (sp_color_set_index << 2) | sp_color_index,
            (_, _, true) => (bg_color_set_index << 2) | bg_color_index,
        } as u16;

        let palette_index = self.load(0x3F00 | palette_ram_index) as usize;

        // let emphasized_palette_index = (palette_index | (self.state().pmask.emphasize_bits() << 6)) as usize;
        let mut rgb = self.state().palette.get_rgb(palette_index);

        if self.state().pmask.emphasize_red() {
            rgb.r = (rgb.r as f32 *1.1) as u8;
            rgb.g = (rgb.g as f32 *0.9) as u8;
            rgb.b = (rgb.b as f32 *0.9) as u8;
        }
        if self.state().pmask.emphasize_green() {
            rgb.r = (rgb.r as f32 *0.9) as u8;
            rgb.g = (rgb.g as f32 *1.1) as u8;
            rgb.b = (rgb.b as f32 *0.9) as u8;
        }
        if self.state().pmask.emphasize_blue() {
            rgb.r = (rgb.r as f32 *0.9) as u8;
            rgb.g = (rgb.g as f32 *0.9) as u8;
            rgb.b = (rgb.b as f32 *1.1) as u8;
        }
        
        let index = self.state().frame_buffer_cursor;
        self.state_mut().frame_buffer[index] = rgb;
        self.state_mut().frame_buffer_cursor += 1;
    }

    fn tick_clear_secondary_oam(&mut self) {
        if self.state().n_scanline == 261 {
            return;
        }
        let index = self.state().secondary_oam_cursor;
        self.state_mut().secondary_oam[index] = 0xff;
        self.state_mut().secondary_oam_cursor = (index + 1) % 32;
    }

    fn tick_sprite_evaluation(&mut self) {
        if self.state().n_dot & 1 == 1 && self.state().primary_oam_cursor < 256 {
            let index = self.state().primary_oam_cursor;
            self.state_mut().primary_oam_latch = self.state().oamdata[index];
            self.state_mut().primary_oam_cursor = index + 1;
            return;
        }

        let value = self.state().primary_oam_latch;
        let index = self.state().secondary_oam_cursor;
        match self.state().sprite_evaluation_state {
            SpriteEvaluationState::Search => {
                let sprite_top = value as usize;
                let sprite_bottom = sprite_top + self.state().pctrl.sprite_length();
                let scanline_y = self.state().n_scanline;
                let scanline_hit_sprite = (sprite_top <= scanline_y) && (scanline_y < sprite_bottom) && sprite_top != 255;
                
                if scanline_hit_sprite {
                    if self.state().primary_oam_cursor == 1 {
                        self.state_mut().sprite_0_on_next_scanline = self.state().secondary_oam_cursor == 0;
                    }
                    if self.state().sprite_nums_on_next_scanline >= 8 {
                        if self.state().pmask.show_background() || self.state().pmask.show_sprites() {
                            self.state_mut().pstatus.set_sprite_overflow(true);
                        }
                        self.state_mut().sprite_evaluation_state = SpriteEvaluationState::Idle;
                    }
                    else {
                        self.state_mut().secondary_oam_cursor = index + 1;
                        self.state_mut().sprite_evaluation_state = SpriteEvaluationState::Copy;
                    } 
                }
                else {
                    self.state_mut().primary_oam_cursor += 3;
                    if self.state().primary_oam_cursor >= 256 {
                        self.state_mut().sprite_evaluation_state = SpriteEvaluationState::Idle;
                    }
                }

                if self.state().sprite_nums_on_next_scanline < 8 {
                    self.state_mut().secondary_oam[index] = value;
                }
            },
            SpriteEvaluationState::Copy => {
                self.state_mut().secondary_oam[index] = value;
                self.state_mut().secondary_oam_cursor = index + 1;
                if (index + 1) & 0b11 == 0 {
                    self.state_mut().sprite_nums_on_next_scanline += 1;
                    self.state_mut().sprite_evaluation_state = SpriteEvaluationState::Search;
                }
            }
            SpriteEvaluationState::Idle => {}
        }
    }

    #[inline]
    fn sp_latch_y(&mut self) {
        let value = self.state().secondary_oam[self.state().secondary_oam_cursor];
        self.state_mut().sprite_y_latch = value;
        self.state_mut().secondary_oam_cursor += 1;
    }

    #[inline]
    fn sp_latch_tile_addr(&mut self) {
        let value = self.state().secondary_oam[self.state().secondary_oam_cursor];
        self.state_mut().sprite_tile_addr_latch = value;
        self.state_mut().secondary_oam_cursor += 1;
    }

    #[inline]
    fn sp_latch_attribute(&mut self) {
        let value = self.state().secondary_oam[self.state().secondary_oam_cursor];
        self.state_mut().sprite_attribute_latch = value;
        self.state_mut().secondary_oam_cursor += 1;
        let sprite_index = self.state().sprite_list_cursor;
        self.state_mut().sprite_list[sprite_index].attribute = value;
    }

    #[inline]
    fn sp_set_position(&mut self) {
        let value = self.state().secondary_oam[self.state().secondary_oam_cursor];
        let x = value;
        let y = self.state_mut().sprite_y_latch;
        let sprite_index = self.state().sprite_list_cursor;
        self.state_mut().sprite_list[sprite_index].set_pos(x, y)
    }

    #[inline]
    fn sp_fetch_tile_lo_addr(&mut self) {
        let lo = self.sprite_tile_lo_addr().fetch_lo();
        self.state_mut().address_latch.set_lo(lo);
    }

    #[inline]
    fn sp_set_lo_shift(&mut self) {
        let hi = self.sprite_tile_lo_addr().fetch_hi();
        self.state_mut().address_latch.set_hi(hi);
        let mut value = self.load(self.state().address_latch);
        let flip_horizontally = self.state().sprite_attribute_latch.is_b6_set();
        if flip_horizontally { value = value.reverse_bits() }
        let sprite_index = self.state().sprite_list_cursor;
        self.state_mut().sprite_list[sprite_index].set_lo_tile_shift(value);
    }

    #[inline]
    fn sp_fetch_tile_hi_addr(&mut self) {
        let lo = self.sprite_tile_hi_addr().fetch_lo();
        self.state_mut().address_latch.set_lo(lo);
    }

    #[inline]
    fn sp_set_hi_shift(&mut self) {
        let hi = self.sprite_tile_hi_addr().fetch_hi();
        self.state_mut().address_latch.set_hi(hi);
        let mut value = self.load(self.state().address_latch);
        let flip_horizontally = self.state().sprite_attribute_latch.is_b6_set();
        if flip_horizontally { value = value.reverse_bits() }
        let sprite_index = self.state().sprite_list_cursor;
        self.state_mut().sprite_list[sprite_index].set_hi_tile_shift(value);

        self.state_mut().sprite_list_cursor = sprite_index + 1;
        self.state_mut().secondary_oam_cursor += 1;
    }

    fn sprite_tile_lo_addr(&self) -> u16 {
        let state = self.state();
        let filp_vertically = state.sprite_attribute_latch.is_b7_set();
        if state.pctrl.is_two_tile_sprite() {
            let pattern_table_addr = if state.sprite_tile_addr_latch & 1 == 0 {
                0x0000
            } else {
                0x1000
            };
            let top_sprite_index = state.sprite_tile_addr_latch & (!1);
            let bottom_sprite_index = top_sprite_index + 1;
            let sprite_y = (state.n_scanline as i16 - state.sprite_y_latch as i16) & 15;

            let mut is_upper_tile = sprite_y < 8;
            let tile_y = if sprite_y < 8 { sprite_y } else { sprite_y - 8 };
            let tile_y = if filp_vertically {
                is_upper_tile = !is_upper_tile;
                7 - tile_y
            } else {
                tile_y
            };

            let index = if is_upper_tile {
                top_sprite_index
            } else {
                bottom_sprite_index
            };

            pattern_table_addr + (index as u16 * 16) + tile_y as u16
        }
        else {
            let tile_y = (state.n_scanline as i16 - state.sprite_y_latch as i16) & 7;
            let index = state.sprite_tile_addr_latch as u16;
            debug_assert!(tile_y < 8);
            let tile_y = if filp_vertically { 7 - tile_y } else { tile_y }; 
            state.pctrl.pattern_table_addr_for_8x8_sprites() + (index as u16 * 16) + tile_y as u16
        }
    }

    fn sprite_tile_hi_addr(&self) -> u16 {
        self.sprite_tile_lo_addr() + 8
    }

    fn shift_sprite_registers(&mut self) {
        for sprite in self.state_mut().sprite_list.iter_mut() {
            if sprite.countdown == 0 {
                sprite.hi_tile_shift <<= 1;
                sprite.lo_tile_shift <<= 1;
            }
            else if sprite.countdown != 255 {
                sprite.countdown -= 1;
            }
        }
    }

    #[inline]
    fn bg_latch_tile_index_addr(&mut self) {
        self.state_mut().address_latch = self.state().current_addr.get_tile_address();
    }

    #[inline]
    fn bg_latch_tile_index(&mut self) {
        self.state_mut().tile_index_latch = self.load(self.state().address_latch) as u16;
    }

    #[inline]
    fn bg_latch_attribute_addr(&mut self) {
        self.state_mut().address_latch = self.state().current_addr.get_attribute_address();
    }

    #[inline]
    fn bg_latch_attribute(&mut self) {
        self.state_mut().attribute_latch = self.load(self.state().address_latch);
        if (self.state().current_addr.get_corase_y() & 2) != 0 { self.state_mut().attribute_latch >>= 4 };
        if (self.state().current_addr.get_corase_x() & 2) != 0 { self.state_mut().attribute_latch >>= 2 };
        self.state_mut().attribute_latch &= 0b11;
    }

    #[inline]
    fn bg_latch_tile_lo_addr(&mut self) {
        self.state_mut().address_latch = self.state().pctrl.bg_pattern_table_addr() + self.state().tile_index_latch * 16 + self.state().current_addr.get_fine_y();
    }

    #[inline]
    fn bg_latch_tile_lo(&mut self) {
        self.state_mut().tile_lo_latch = self.load(self.state().address_latch);
    }

    #[inline]
    fn bg_latch_tile_hi_addr(&mut self) {
        self.state_mut().address_latch = self.state().pctrl.bg_pattern_table_addr() + self.state().tile_index_latch * 16 + self.state().current_addr.get_fine_y() + 8;
    }

    #[inline]
    fn bg_latch_tile_hi(&mut self) {
        self.state_mut().tile_hi_latch = self.load(self.state().address_latch);
    }

    fn h_scroll(&mut self) {
        if self.is_rendering() {
            self.state_mut().current_addr.increase_corase_x()
        }
    }

    fn v_scroll(&mut self) {
        if self.is_rendering() {
            self.state_mut().current_addr.increase_fine_y()
        }
    }

    fn h_update(&mut self) {
        if self.is_rendering() {
            let t = self.state().temporary_addr;
            self.state_mut().current_addr.copy_horizontal_postion_bits(t);
        }
    }

    fn v_update(&mut self) {
        if self.is_rendering() {
            let t = self.state().temporary_addr;
            self.state_mut().current_addr.copy_vertical_postion_bits(t);
        }
    }

    fn reload_background_registers(&mut self) {
        self.state_mut().background_shift_lo = (self.state().background_shift_lo & 0xff00) | (self.state().tile_lo_latch as u16);
        self.state_mut().background_shift_hi = (self.state().background_shift_hi & 0xff00) | (self.state().tile_hi_latch as u16);
        self.state_mut().attribute_shift_lo = (self.state().attribute_shift_lo & 0xff00) | (((self.state().attribute_latch as u16 >> 0) & 1) * 0xff);
        self.state_mut().attribute_shift_hi = (self.state().attribute_shift_hi & 0xff00) | (((self.state().attribute_latch as u16 >> 1) & 1) * 0xff);
    }

    fn shift_background_registers(&mut self) {
        self.state_mut().background_shift_lo <<= 1;
        self.state_mut().background_shift_hi <<= 1;
        self.state_mut().attribute_shift_lo <<= 1;
        self.state_mut().attribute_shift_hi <<= 1;
    }

    fn load(&mut self, mut address: u16) -> u8 {
        if address < 0x3f00 {
            self.peek_vram(address)
        } else {
            address &= 32 - 1;
            debug_assert!((address as usize) < self.state_mut().palette_ram.len());
            self.state().palette_ram[address as usize]
        }
    }

    fn store(&mut self, mut address: u16, mut value: u8) {
        if address < 0x3f00{
            self.poke_vram(address, value);
        } else {
            address &= 32 - 1;
            debug_assert!((address as usize) < self.state_mut().palette_ram.len());
            value &= 64 - 1;
            self.state_mut().palette_ram[address as usize] = value;
            if (address & 0b11) == 0 { // mirror
                if address >= 16 {
                    address -= 16;
                }
                else {
                    address += 16;
                }
            }
            self.state_mut().palette_ram[address as usize] = value;
        };
    }

    fn is_rendering(&self) -> bool {
        let mask = &self.state().pmask;
        mask.show_background() || mask.show_sprites()
    }

    fn increase_current_address(&mut self) {
        let inc = self.state().pctrl.vram_addr_increment();
        let value = (self.state().current_addr.0 as usize + inc) & 0x7FFF;
        self.state_mut().current_addr.0 = value as u16;
    }

    fn write_ppuaddr(&mut self, value: u8) {
        if self.state().write_toggle == false {
            self.state_mut().temporary_addr.set_high_byte(value);
            self.state_mut().write_toggle = true;
        }
        else {
            self.state_mut().temporary_addr.set_low_byte(value);
            self.state_mut().current_addr.0 = self.state().temporary_addr.0;
            self.state_mut().write_toggle = false;
        }
    }

    fn read_ppudata(&mut self) -> u8 {
        let addr = self.state().current_addr.0 & 0x3FFF;
        // http://wiki.nesdev.com/w/index.php/PPU_registers#Data_.28.242007.29_.3C.3E_read.2Fwrite
        let mut value = self.load(addr);
        self.increase_current_address();
        if addr < 0x3f00 {
            let old = self.state().ppudata_latch;
            self.state_mut().ppudata_latch = value;
            old
        }
        else {
            self.state_mut().ppudata_latch = self.load(addr & 0x2fff);
            if self.state().pmask.greyscale_mode() {
                value &= 0b110000;
            }
            value
        }
    }

    fn write_ppudata(&mut self, value: u8) {
        let addr = self.state().current_addr.0 & 0x3FFF;
        self.store(addr, value);
        self.increase_current_address();
    }

    fn read_ppustatus(&mut self) -> u8 {
        self.state_mut().vblank_suppress_flag = true;
        let value = self.state().pstatus.0;
        self.state_mut().pstatus.set_vblank_occured(false);
        self.state_mut().nmi_ready_to_trigger = false;
        self.state_mut().write_toggle = false;
        value
    }

    fn write_ppuctrl(&mut self, value: u8) {
        self.state_mut().pctrl.0 = value;
        let nn = self.state().pctrl.get_nn();
        self.state_mut().temporary_addr.set_nn(nn);
    }

    fn write_ppumask(&mut self, value: u8) {
        self.state_mut().pmask.0 = value;
    }

    fn read_oamdata(&mut self) -> u8 {
        let index = self.state().oamaddr;
        self.state().oamdata[index]
    }

    fn write_oamdata(&mut self, value: u8) {
        let index = self.state().oamaddr;
        if !(self.state().n_scanline < 240 && self.state().n_scanline == 261) {
            self.state_mut().oamdata[index] = value;
        }
        self.state_mut().oamaddr = (index + 1) & 0xFF;
    }

    fn write_oamaddr(&mut self, value: u8) {
        self.state_mut().oamaddr = value as usize;
    }

    fn write_ppuscroll(&mut self, value: u8) {
        if self.state().write_toggle == false {
            self.state_mut().fine_x = value & 0b111;
            self.state_mut().temporary_addr.0 = (self.state().temporary_addr.0 & 0b0_111_11_11111_00000) | ((value >> 3) as u16);
            self.state_mut().write_toggle = true;
        }
        else {
            let tmp = ((((value & 0b00000111) as u16) >> 0) << 12) |
                      ((((value & 0b00111000) as u16) >> 3) <<  5) |
                      ((((value & 0b11000000) as u16) >> 6) <<  8);

            self.state_mut().temporary_addr.0 = (self.state().temporary_addr.0 & 0b0_000_11_00000_11111) | tmp;
            self.state_mut().write_toggle = false;
        }
    }
}