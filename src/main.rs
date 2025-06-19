#[cfg(feature = "debug")]
mod debug;
use debug::Debugger;

mod json_tests;

mod emulator;
use egui::{Color32, Event, Frame, TextureOptions};
use emulator::{run_emulator, EmulatorSend};
use parking_lot::RwLock;
use gba_core::Emulator;

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use std::env;
use std::thread;

const JSON_TESTS: bool = true;
fn main() {
    if JSON_TESTS {
        json_tests::perform_tests();
        return;
    }

    let file = env::args().nth(1).unwrap();
    let rom_path = format!("roms/{file}");
    let emulator_ref = Arc::new(RwLock::new(Emulator::new(&rom_path)));

    let (emu_send, emu_recv) = mpsc::channel::<EmulatorSend>();
    let (draw_send, draw_recv) = mpsc::sync_channel::<Vec<u32>>(1);

    let emulator = emulator_ref.clone();
    thread::Builder::new().name("emulator_thread".into()).spawn(|| {
        run_emulator(emulator, draw_send, emu_recv);
    }).unwrap();
    
    let debugger;
    match cfg!(feature = "debug") {
        true => debugger = Some(Debugger::new(emulator_ref, emu_send.clone())),
        false => debugger = None,
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_resizable(false)
            .with_inner_size([SCREEN_WIDTH as f32 * SCREEN_RATIO, SCREEN_HEIGHT as f32 * SCREEN_RATIO])
            .with_position([780., 0.]),
        ..Default::default()
    };
    let emulator_app = EmulatorApp::new(draw_recv, emu_send, debugger);
    eframe::run_native(
        "Emulator", 
        options, 
        Box::new(|_| Ok(Box::new(emulator_app)))
    ).unwrap();
}

struct EmulatorApp {
    redraw_recv: Receiver<Vec<u32>>,
    inp_send: Sender<EmulatorSend>,
    debugger: Option<Debugger>,
    previous_screen: Vec<u32>,
}
impl EmulatorApp {
    fn new(
        redraw_recv: Receiver<Vec<u32>>, 
        inp_send: Sender<EmulatorSend>,
        debugger: Option<Debugger>,
    ) -> Self {
        Self {
            redraw_recv,
            inp_send,
            debugger,
            previous_screen: vec![0; SCREEN_HEIGHT * SCREEN_WIDTH],
        }
    }
}
impl eframe::App for EmulatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(ref mut debugger) = self.debugger {
            debugger.update(ctx);
        }
    
        // check if a redraw needs to happen
        // scroll through all of them until it is up to the most recent
        while let Ok(screen) = self.redraw_recv.try_recv() {
            self.previous_screen = screen;
        }
        draw(&self.previous_screen, ctx);


        ctx.input(|i| {
            for event in &i.events {
                if let Event::Key {key, pressed, ..} = event {
                    self.inp_send.send(EmulatorSend::Event(*key, *pressed)).unwrap();
                }
            }
        });

        ctx.request_repaint();
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