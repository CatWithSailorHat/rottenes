use std::path::Path;

use crate::{error::LoadError, nes};


use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::rect::Rect;
use sdl2::keyboard::Keycode;
use std::time::Duration;
 

pub struct GuiObject {
    nes: nes::State,
}

impl nes::Context for GuiObject {
    fn state_mut(&mut self) -> &mut nes::State {
        &mut self.nes
    }

    fn state(&self) -> &nes::State {
        &self.nes
    }
}

impl GuiObject {
    pub fn new() -> Self {
        GuiObject {
            nes: nes::State::new(),
        }
    }

    pub fn load_rom_from_file(&mut self, path: &Path) -> Result<(), LoadError> {
        nes::Interface::load_rom_from_file(self, path)
    }

    pub fn run(&mut self) {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        
        let window = video_subsystem.window("rust-sdl2 demo", 256*3, 240*3)
            .position_centered()
            .build()
            .unwrap();
        
        let mut canvas = window.into_canvas().build().unwrap();
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        nes::Interface::reset(self);

        let mut event_pump = sdl_context.event_pump().unwrap();
        
        'running: loop {
            nes::Interface::run_for_one_frame(self);
            let frame_buffer = nes::Interface::get_framebuffer(self);
            for (i, rgb) in frame_buffer.iter().enumerate() {
                let i = i as i32;
                let x = i % 256;
                let y = i / 256;
                canvas.set_draw_color(Color::RGB(rgb.r, rgb.g, rgb.b));
                canvas.fill_rect(Rect::new(x*3, y*3, 3, 3)).unwrap();
            }
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..} |
                    Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running
                    },
                    _ => {}
                }
            }
            canvas.present();
            ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        }
    }
}