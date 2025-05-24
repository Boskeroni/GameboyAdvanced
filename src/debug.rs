use core::{cpu::Cpu, gba_frame, joypad::{joypad_press, joypad_release}, ppu::Ppu};
use std::time::Instant;

// just contains all of the debug code
use eframe::egui;
use egui::{Color32, Event, TextureOptions};
use crate::{convert_to_joypad, GbaContext, SCREEN_HEIGHT, SCREEN_WIDTH};

pub fn setup_debug(context: GbaContext) {
    let options = eframe::NativeOptions::default();
    let debug = GbaAdvanceDebug::new(context);
    eframe::run_native(
        "Debug window",
        options,
        Box::new(|_| Ok(Box::new(debug))),
    ).unwrap();
}

struct GbaAdvanceDebug {
    show_memory_panel: bool,
    show_vram_panel: bool,
    show_cpu_panel: bool,
    show_instruction_panel: bool,
    gba_context: GbaContext,

    paused: bool,
    step: bool,
    last_render: Instant,
}
impl GbaAdvanceDebug {
    pub fn new(mut context: GbaContext) -> Self {
        core::prelimenary(&mut context.memory);

        Self {
            show_memory_panel: true,
            show_vram_panel: true,
            show_cpu_panel: true,
            show_instruction_panel: true,
            gba_context: context,

            // this controls the flow of the gba
            paused: true,
            step: false,

            last_render: Instant::now(),
        }
    }
}

impl eframe::App for GbaAdvanceDebug {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // this always gets called so its chill
        if !self.paused || self.step {
            game_panel(ctx, &mut self.gba_context);
            self.step = false;
        }
        draw(&self.gba_context.ppu, ctx);

        // options panel
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
            }

            if ui.add(egui::Button::new("⏭")).clicked() {
                self.step = true;
            }


            let diff = Instant::now().duration_since(self.last_render).as_nanos();
            let fps = 1_000_000_000 / diff;
            ui.label(format!("{fps} fps achieved"));
            self.last_render = Instant::now();
        });

        if self.show_memory_panel      { memory_panel(ctx)     }
        if self.show_cpu_panel         { cpu_panel(ctx, &self.gba_context.cpu)}
        if self.show_vram_panel        { vram_panel(ctx)       }
        if self.show_instruction_panel { instruction_panel(ctx)}

        egui::Context::request_repaint(ctx);
    }
}

fn memory_panel(ctx: &egui::Context) {
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("memory panel"), 
        egui::ViewportBuilder::default()
            .with_title("memory panel")
            .with_inner_size([400., 400.]),
        |ctx, class| {
            assert!(class == egui::ViewportClass::Immediate);
            egui::CentralPanel::default().show(ctx, |ui| {

            });
        }
    );
}
fn cpu_panel(ctx: &egui::Context, cpu: &Cpu) {
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("CPU panel"), 
        egui::ViewportBuilder::default()
            .with_title("Cpu panel")
            .with_inner_size([200., 400.]),
        |ctx, class| {
            assert!(class == egui::ViewportClass::Immediate);

            egui::CentralPanel::default().show(ctx, |ui| {
                for i in 0..15 {
                    let value = cpu.get_register(i);
                    ui.code(format!("Reg {i}: {value:X}"));
                }
            });
        } 
    );
}
fn vram_panel(_ctx: &egui::Context) {

}
fn instruction_panel(_ctx: &egui::Context) {

}

fn game_panel(ctx: &egui::Context, gba_context: &mut GbaContext) {
    // run one frame worth of the gba emulator
    gba_frame(
        &mut gba_context.cpu,
        &mut gba_context.memory, 
        &mut gba_context.ppu, 
        &mut gba_context.fde, 
        &mut gba_context.cycles, 
    );
    ctx.input(|i| {
        if !i.focused {return; }

        for event in &i.events {
            if let Event::Key {key, pressed, ..} = event {
                let joypad_button = convert_to_joypad(key);
                match pressed {
                    true => joypad_press(joypad_button, &mut gba_context.memory),
                    false => joypad_release(joypad_button, &mut gba_context.memory),
                }
            }
        }
    });
}

const SCREEN_RATIO: f32 = 2.0;
fn draw(ppu: &Ppu, ctx: &egui::Context) {
    let converted_pixels = texture_pixels(ppu);
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
            .with_inner_size([(SCREEN_WIDTH as f32) * SCREEN_RATIO, (SCREEN_HEIGHT as f32) * SCREEN_RATIO]),
        |ctx, class| {
            assert!(class == egui::ViewportClass::Immediate);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add(egui::Image::new(sized_texture).fit_to_exact_size(size * 2.));
            });
        } 
    );
}

fn texture_pixels(ppu: &Ppu) -> egui::ColorImage {
    let mut pixels: Vec<egui::Color32> = vec![Color32::BLACK; SCREEN_WIDTH * SCREEN_HEIGHT];
    for (i, c) in ppu.stored_screen.iter().enumerate() {
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