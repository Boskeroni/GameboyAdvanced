#![cfg(feature = "debug")]
use std::{sync::{mpsc::Sender, Arc}, time::Instant};
use egui::{Frame, ViewportBuilder, ViewportClass, ViewportId};
use parking_lot::RwLock;
use core::{memory::Memory, Emulator};
use crate::emulator::{EmulatorSend, EmulatorState};

pub struct Debugger {
    emulator_ref: Arc<RwLock<Emulator>>,
    inp_send: Sender<EmulatorSend>,
    show_memory: bool,
    show_vram: bool,
    show_cpu: bool,
    show_instructions: bool,
    pause: bool,
}
impl Debugger {
    pub fn new(emulator: Arc<RwLock<Emulator>>, inp_send: Sender<EmulatorSend>) -> Self { 
        Self { 
            emulator_ref: emulator,
            inp_send,
            show_memory: false,
            show_vram: false,
            show_cpu: false,
            show_instructions: false,
            pause: false,
        } 
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("control_panel"), 
            ViewportBuilder::default()
                .with_title("control panel")
                .with_resizable(false)
                .with_inner_size([340., 140.]), 
            |ctx, class| {
                assert!(class == ViewportClass::Immediate);
                egui::CentralPanel::default().show(&ctx, |ui| {
                    ui.label("Debug panel");

                    // menu to create new windows with information
                    ui.columns(2, |columns| {
                        columns[0].checkbox(&mut self.show_memory, "Show memory panel");
                        columns[0].checkbox(&mut self.show_vram, "Show VRAM panel");
                        columns[1].checkbox(&mut self.show_cpu, "Show CPU panel");
                        columns[1].checkbox(&mut self.show_instructions, "Show instruction panel");
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        // pause and step buttons
                        let pause_button_text = match self.pause {
                            false => "⏸",
                            true => "⏵"
                        };
                        if ui.add(egui::Button::new(pause_button_text)).clicked() {
                            self.pause = !self.pause;
                            self.inp_send.send(match self.pause {
                                true => EmulatorSend::StateUpdate(EmulatorState::Pause),
                                false => EmulatorSend::StateUpdate(EmulatorState::Run),
                            }).unwrap();
                        }

                        if ui.add(egui::Button::new("⏭")).clicked() {
                            self.inp_send.send(EmulatorSend::StateUpdate(EmulatorState::Step)).unwrap();
                        }
                    });
                });
            }
        );

        let emulator = self.emulator_ref.read();
        println!("we get access :? {:?}", Instant::now());
        if self.show_cpu {eprintln!("not done yet"); self.show_cpu = false;}
        if self.show_instructions {eprintln!("not done yet"); self.show_instructions = false;}
        if self.show_vram {eprintln!("not done yet"); self.show_vram = false;}
        if self.show_memory { show_memory(&emulator.mem, ctx) }
        
        
    }
}

fn show_memory(mem: &Memory, ctx: &egui::Context) {
    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("memory panel"), 
        ViewportBuilder::default(), 
        |ctx, class| {
            assert!(class == ViewportClass::Immediate);
            egui::CentralPanel::default().frame(Frame::NONE).show(&ctx, |ui| {

            });
        }
    );
}