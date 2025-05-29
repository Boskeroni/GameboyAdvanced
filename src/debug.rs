use core::cpu::Cpu;
use std::{sync::mpsc::{Receiver, Sender}, time::Instant};
use eframe::egui;
use egui::{Color32, Frame, TextureOptions};

pub enum DebugCommand {
    Run,
    Stop,
    Step,
}

pub struct DebugDataBackend {
    pub cpu_dbg: Sender<Cpu>,
    pub ppu_dbg: Sender<Vec<u32>>,
    pub mem_dbg: Sender<Vec<u8>>,
    pub ins_dbg: Sender<String>,
    pub cnt_dbg: Receiver<DebugCommand>,
}
pub struct DebugDataFrontend {
    pub cpu_dbg: Receiver<Cpu>,
    pub ppu_dbg: Receiver<Vec<u32>>,
    pub mem_dbg: Receiver<Vec<u8>>,
    pub ins_dbg: Receiver<String>,
    pub cnt_dbg: Sender<DebugCommand>,
}

pub fn setup_debug(debug_interface: DebugDataFrontend, sender: Sender<egui::Event>) {
    let options = eframe::NativeOptions::default();
    let debug = GbaAdvanceDebug::new(debug_interface, sender);
    eframe::run_native(
        "Debug window",
        options,
        Box::new(|_| Ok(Box::new(debug))),
    ).unwrap();
}

struct GbaAdvanceDebug {
    debug: DebugDataFrontend,
    screen: Vec<u32>,
    input_send: Sender<egui::Event>,

    show_memory_panel: bool,
    show_vram_panel: bool,
    show_cpu_panel: bool,
    show_instruction_panel: bool,
    paused: bool,
    step: bool
}
impl GbaAdvanceDebug {
    pub fn new(debug: DebugDataFrontend, sender: Sender<egui::Event>) -> Self {
        Self {
            debug,
            screen: Vec::new(),
            input_send: sender,

            show_memory_panel: true,
            show_vram_panel: true,
            show_cpu_panel: true,
            show_instruction_panel: true,
            paused: true,
            step: false,
        }
    }

    fn update_debug(&mut self) {
        // cpu debug
        if let Ok(_cpu_state) = self.debug.cpu_dbg.try_recv() {

        }
        // ppu although this happens always
        loop {
            if let Ok(new_screen) = self.debug.ppu_dbg.try_recv() {
                self.screen = new_screen;
                continue;                
            }
            break;
        }
        
        // mem debug
        if let Ok(_mem_state) = self.debug.mem_dbg.try_recv() {
            
        }
        // ins debug
        if let Ok(_instruction) = self.debug.ins_dbg.try_recv() {
            
        }
        // send command

    }

    fn send_inputs(&self, ctx: &egui::Context) {
        ctx.input(|i| {
            if !i.focused { return; }

            for event in &i.events {
                self.input_send.send(event.clone()).unwrap();
            }
        });
    }
}
impl eframe::App for GbaAdvanceDebug {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // options menu
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Debug panel");

            // menu to create new windows with information
            ui.checkbox(&mut self.show_memory_panel, "Show memory panel");
            ui.checkbox(&mut self.show_vram_panel,"Show VRAM panel");
            ui.checkbox(&mut self.show_cpu_panel,"Show CPU panel");
            ui.checkbox(&mut self.show_instruction_panel,"Show instruction panel");

            // pause and step buttons
            let pause_button_text = match self.paused {
                false => "⏸",
                true => "⏵"
            };
            if ui.add(egui::Button::new(pause_button_text)).clicked() {
                self.paused = !self.paused;
                match self.paused {
                    true => self.debug.cnt_dbg.send(DebugCommand::Stop).unwrap(),
                    false => self.debug.cnt_dbg.send(DebugCommand::Run).unwrap(),
                }
            }

            if ui.add(egui::Button::new("⏭")).clicked() {
                self.step = true;
                self.debug.cnt_dbg.send(DebugCommand::Step).unwrap();
            }
        });
        self.update_debug();
        self.send_inputs(ctx);

        draw(&self.screen, ctx);
        egui::Context::request_repaint(ctx);
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
    
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("game_viewport"), 
        egui::ViewportBuilder::default()
            .with_title("game viewport")
            .with_inner_size([(SCREEN_WIDTH as f32) * SCREEN_RATIO, (SCREEN_HEIGHT as f32) * SCREEN_RATIO])
            .with_position([0., 0.])
            .with_resizable(false),
        |ctx, class| {
            assert!(class == egui::ViewportClass::Immediate);
            egui::CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
                ui.add(egui::Image::new(sized_texture).fit_to_exact_size(size * 2.));
            });
        } 
    );
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