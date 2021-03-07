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
        use std::time::Instant;
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        
        let magnifaction = 3u32;
        let window = video_subsystem.window("rust-sdl2 demo", 256 * magnifaction, 240 * magnifaction)
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
            let start = Instant::now();
            // let start2 = Instant::now();
            nes::Interface::run_for_one_frame(self);
            // println!("time cost: {:?} ms", start2.elapsed().as_millis());
            let frame_buffer = nes::Interface::get_framebuffer(self);
            for (i, rgb) in frame_buffer.iter().enumerate() {
                let i = i as i32;
                let x = i % 256;
                let y = i / 256;
                canvas.set_draw_color(Color::RGB(rgb.r, rgb.g, rgb.b));
                canvas.fill_rect(Rect::new(x * magnifaction as i32, y * magnifaction as i32, magnifaction, magnifaction)).unwrap();
            }
            // canvas.fill_rect(Rect::new(0, 240*3, 256*3, 12)).unwrap();
            // for (i, rgb) in nes::Interface::dbg_list_palette_ram(self).iter().enumerate() {
            //     canvas.set_draw_color(Color::RGB(rgb.r, rgb.g, rgb.b));
            //     let i = i as i32;
            //     let x = i % 256;
            //     canvas.fill_rect(Rect::new(x*12, 240*3, 12, 12)).unwrap();
            // }

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..} |
                    Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running
                    },
                    Event::KeyDown { keycode: Some(Keycode::Return), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::START, true)
                    },
                    Event::KeyDown { keycode: Some(Keycode::Space), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::SELECT, true)
                    },
                    Event::KeyDown { keycode: Some(Keycode::W), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::UP, true)
                    },
                    Event::KeyDown { keycode: Some(Keycode::S), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::DOWN, true)
                    },
                    Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::LEFT, true)
                    },
                    Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::RIGHT, true)
                    },
                    Event::KeyDown { keycode: Some(Keycode::J), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::B, true)
                    },
                    Event::KeyDown { keycode: Some(Keycode::K), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::A, true)
                    },

                    Event::KeyUp { keycode: Some(Keycode::Return), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::START, false)
                    },
                    Event::KeyUp { keycode: Some(Keycode::Space), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::SELECT, false)
                    },
                    Event::KeyUp { keycode: Some(Keycode::W), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::UP, false)
                    },
                    Event::KeyUp { keycode: Some(Keycode::S), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::DOWN, false)
                    },
                    Event::KeyUp { keycode: Some(Keycode::A), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::LEFT, false)
                    },
                    Event::KeyUp { keycode: Some(Keycode::D), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::RIGHT, false)
                    },
                    Event::KeyUp { keycode: Some(Keycode::J), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::B, false)
                    },
                    Event::KeyUp { keycode: Some(Keycode::K), .. } => {
                        nes::Interface::set_input_1(self, nes::StandardInput::A, false)
                    },
                    _ => {}
                }
            }
            canvas.present();
            let t = start.elapsed().as_nanos();
            let wait = if (1_000_000_000u128 / 60) > t {
                ((1_000_000_000u128 / 60) - t) as u32
            }
            else {
                0
            };
            ::std::thread::sleep(Duration::new(0, wait));
        }
    }
}