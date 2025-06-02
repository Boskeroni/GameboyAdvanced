#![cfg(feature = "debug")]
use std::{sync::{mpsc::Sender, Arc, Mutex}, time::Instant};
use eframe::egui;
use egui::{ViewportBuilder, ViewportClass, ViewportId};
use core::Emulator;
use crate::emulator::{EmulatorSend, EmulatorState};

pub struct Debugger {
    emulator: Arc<Mutex<Emulator>>,
    inp_send: Sender<EmulatorSend>,
}
impl Debugger {
    pub fn new(emulator: Arc<Mutex<Emulator>>, inp_send: Sender<EmulatorSend>) -> Self { 
        Self { 
            emulator,
            inp_send,
        } 
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        let start = Instant::now();

        let mut show_memory_panel = false;
        let mut show_vram_panel = false;
        let mut show_cpu_panel = false;
        let mut show_instruction_panel = false;

        let mut is_paused = false;

        //let emulator = self.emulator.lock().unwrap();
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("control_panel"), 
            ViewportBuilder::default()
                .with_title("control panel"), 
            |ctx, class| {
                assert!(class == ViewportClass::Immediate);
                egui::CentralPanel::default().show(&ctx, |ui| {
                    ui.label("Debug panel");

                    // menu to create new windows with information
                    ui.checkbox(&mut show_memory_panel, "Show memory panel");
                    ui.checkbox(&mut show_vram_panel, "Show VRAM panel");
                    ui.checkbox(&mut show_cpu_panel, "Show CPU panel");
                    ui.checkbox(&mut show_instruction_panel, "Show instruction panel");

                    // pause and step buttons
                    let pause_button_text = match is_paused {
                        false => "⏸",
                        true => "⏵"
                    };
                    if ui.add(egui::Button::new(pause_button_text)).clicked() {
                        is_paused = !is_paused;
                        self.inp_send.send(match is_paused {
                            true => EmulatorSend::StateUpdate(EmulatorState::Pause),
                            false => EmulatorSend::StateUpdate(EmulatorState::Run),
                        }).unwrap();
                    }

                    if ui.add(egui::Button::new("⏭")).clicked() {
                        self.inp_send.send(EmulatorSend::StateUpdate(EmulatorState::Step)).unwrap();
                    }
                });
            }
        );
    }
}