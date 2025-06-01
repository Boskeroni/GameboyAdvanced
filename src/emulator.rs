use core::{run_frame, joypad::{self, init_joypad, joypad_press, joypad_release}, run_single_step, Emulator};
use std::sync::{mpsc::{Receiver, Sender}, Arc, Mutex};
use egui::{Color32, Event, Frame, Key, TextureOptions};

fn convert_to_joypad(code: Key) -> joypad::Button {
    use joypad::Button;
    use egui::Key;
    match code {
        Key::Z => Button::Select,
        Key::X => Button::Start,
        Key::ArrowLeft => Button::Left,
        Key::ArrowRight => Button::Right,
        Key::ArrowDown => Button::Down,
        Key::ArrowUp => Button::Up,
        Key::K => Button::A,
        Key::L => Button::B,
        Key::Q => Button::L,
        Key::P => Button::R,
        _ => Button::Other,
    }
}

// this might just be used for the debug, but it seems
// useful to have anyways
pub enum EmulatorCommand {
    Run,
    Pause,
    Step,
}

pub struct EmulatorApp { 
    pub emulator: Arc<Mutex<Emulator>>,
    pub dbg_ctx_send: Option<Sender<egui::Context>>,
    pub dbg_cmd_recv: Option<Receiver<EmulatorCommand>>,
    state: EmulatorCommand,
}
impl EmulatorApp {
    pub fn new(
        emulator: Arc<Mutex<Emulator>>,
        dbg_ctx_send: Option<Sender<egui::Context>>,
        dbg_cmd_recv: Option<Receiver<EmulatorCommand>>
    ) -> Self {
        init_joypad(&mut emulator.lock().unwrap().mem);

        Self {
            emulator,
            dbg_cmd_recv,
            dbg_ctx_send,
            state: EmulatorCommand::Run,
        }
    }
}
impl eframe::App for EmulatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut emulator = self.emulator.lock().unwrap();

        use EmulatorCommand::*;
        let redraw_needed;
        match self.state {
            Run => {
                run_frame(&mut emulator);
                redraw_needed = true;
            },
            Step => redraw_needed = run_single_step(&mut emulator),
            Pause => redraw_needed = false,
        }

        if redraw_needed {
            draw(&emulator.ppu.stored_screen, ctx);
            ctx.request_repaint();
        }

        ctx.input(|i| {
            if !i.focused { return; }
            for event in &i.events {
                if let Event::Key {key, pressed, ..} = event {
                    let button = convert_to_joypad(*key);
                    match pressed {
                        true => joypad_press(button, &mut emulator.mem),
                        false => joypad_release(button, &mut emulator.mem),
                    }

                }
            }
        });

        if let Some(dbg_send) = &self.dbg_ctx_send {
            dbg_send.send(ctx.clone()).unwrap();
        }
        if let Some(dbg_recv) = &self.dbg_cmd_recv {
            if let Ok(cmd) = dbg_recv.try_recv() {
                self.state = cmd;
            }
        }
    }
}

const SCREEN_WIDTH: usize = 240;
const SCREEN_HEIGHT: usize = 160;
const SCREEN_RATIO: f32 = 2.0;
fn draw(screen: &Vec<u32>, ctx: &egui::Context) {
    let converted_pixels = texture_pixels(screen);
    let texture = ctx.load_texture(
        "game", 
        converted_pixels, 
        TextureOptions::default()
    );
    let size = texture.size_vec2();
    let sized_texture = egui::load::SizedTexture::new(&texture, size);
    
    egui::CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
        ui.add(egui::Image::new(sized_texture).fit_to_exact_size(size * SCREEN_RATIO));
    });
}

fn texture_pixels(screen: &Vec<u32>) -> egui::ColorImage {
    let mut pixels: Vec<egui::Color32> = vec![Color32::BLACK; SCREEN_WIDTH * SCREEN_HEIGHT];
    for (i, c) in screen.iter().enumerate() {
        let r = (*c >> 16) & 0xFF;
        let g = (*c >> 8) & 0xFF;
        let b = *c & 0xFF;

        pixels[i] = Color32::from_rgb(r as u8, g as u8, b as u8);
    }
    egui::ColorImage {
        size: [SCREEN_WIDTH, SCREEN_HEIGHT],
        pixels,
    }
}