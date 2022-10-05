use std::path::Path;

use nes::{LoadError, Emulator, StandardInput};

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::rect::Rect;
use sdl2::keyboard::Keycode;
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use std::time::Duration; 

pub struct GuiObject {
    emulator: Emulator,
    save_slot: Option<Vec<u8>>,
}

impl GuiObject {
    pub fn new() -> Self {
        GuiObject {
            emulator: Emulator::new(),
            save_slot: None,
        }
    }

    pub fn load_rom_from_file(&mut self, path: &Path) -> Result<(), LoadError> {
        self.emulator.load_rom_from_file(path)
    }

    pub fn run(&mut self) {
        let mut frame_counter = 0usize;
        let mut frame_skipped = 0usize;
        use std::time::Instant;
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let audio_subsystem = sdl_context.audio().unwrap();
        
        let magnifaction = 3u32;
        let window = video_subsystem.window("rust-sdl2 demo", 256 * magnifaction, 240 * magnifaction)
            .position_centered()
            .build()
            .unwrap();
        
        let mut canvas = window.into_canvas().build().unwrap();
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        self.emulator.reset();

        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1),
            samples: None,
        };

        let audio_device: AudioQueue<f32> = audio_subsystem.open_queue(None, &desired_spec).unwrap();
        audio_device.resume();


        let mut event_pump = sdl_context.event_pump().unwrap();
        
        'running: loop {
            let start = Instant::now();
            // let start2 = Instant::now();
            self.emulator.run_for_one_frame();
            frame_counter += 1;
            // println!("time cost: {:?} ms", start2.elapsed().as_millis());
            let frame_buffer = self.emulator.get_framebuffer();
            for (i, rgb) in frame_buffer.iter().enumerate() {
                let i = i as i32;
                let x = i % 256;
                let y = i / 256;
                canvas.set_draw_color(Color::RGB(rgb.r, rgb.g, rgb.b));
                canvas.fill_rect(Rect::new(x * magnifaction as i32, y * magnifaction as i32, magnifaction, magnifaction)).unwrap();
            }

            for event in event_pump.poll_iter() {
                match event {
                    Event::DropFile { timestamp, window_id, filename } => {
                        let path = Path::new(&filename);
                        self.emulator.load_rom_from_file(&path).unwrap();
                        self.emulator.reset();
                    }
                    Event::KeyDown { keycode: Some(Keycode::E), repeat: false, .. } => {
                        self.save_slot = Option::Some(self.emulator.save_state());
                    },
                    Event::KeyDown { keycode: Some(Keycode::Q), repeat: false, .. } => {
                        if let Some(v) = &self.save_slot {
                            self.emulator.load_state(&v)
                        }
                    },
                    Event::Quit {..}  => {
                        break 'running
                    },
                    _ => {}
                }
            }

            let keyboard_state = sdl2::keyboard::KeyboardState::new(&event_pump);
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Return) {
                self.emulator.set_input_1(StandardInput::START, true)
            }
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Space) {
                self.emulator.set_input_1(StandardInput::SELECT, true)
            }
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::W) {
                self.emulator.set_input_1(StandardInput::UP, true)
            }
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::S) {
                self.emulator.set_input_1(StandardInput::DOWN, true)
            }
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::A) {
                self.emulator.set_input_1(StandardInput::LEFT, true)
            }
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::D) {
                self.emulator.set_input_1(StandardInput::RIGHT, true)
            }
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::J) {
                self.emulator.set_input_1(StandardInput::B, true)
            }
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::K) {
                self.emulator.set_input_1(StandardInput::A, true)
            }

            audio_device.queue_audio(self.emulator.get_sample().as_slice()).unwrap();
            self.emulator.clear_sample();
            
            // if frame_counter % 60 == 0 {
            //     println!("{}", frame_skipped);
            //     frame_skipped = 0;
            // }

            if audio_device.size() < 44100 / 2 && frame_counter & 1 == 0 {
                frame_skipped += 1;
                continue;
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