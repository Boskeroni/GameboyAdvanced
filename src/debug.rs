#![cfg(feature = "debug")]
mod memory_widget;
mod instruction_widget;
mod cpu_widget;

use std::sync::{mpsc::Sender, Arc};
use cpu_widget::CpuWidget;
use egui::{ViewportBuilder, ViewportClass, ViewportId};
use instruction_widget::InstructionWidget;
use memory_widget::MemoryWidget;
use parking_lot::RwLock;
use gba_core::Emulator;
use crate::emulator::{EmulatorSend, EmulatorState};

pub struct Debugger {
    emulator_ref: Arc<RwLock<Emulator>>,
    inp_send: Sender<EmulatorSend>,
    mem_widget: MemoryWidget,
    ins_widget: InstructionWidget,
    cpu_widget: CpuWidget,
    show_vram: bool,
    pause: bool,
    delay: String,
}
impl Debugger {
    pub fn new(emulator: Arc<RwLock<Emulator>>, inp_send: Sender<EmulatorSend>) -> Self { 
        Self { 
            emulator_ref: emulator,
            inp_send,
            mem_widget: MemoryWidget::new(),
            ins_widget: InstructionWidget::new(),
            cpu_widget: CpuWidget::new(),
            show_vram: false,
            pause: false,
            delay: String::from("0"),
        } 
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("control_panel"), 
            ViewportBuilder::default()
                .with_title("control panel")
                .with_resizable(false)
                .with_inner_size([340., 140.])
                .with_position([780., 575.]), 
            |ctx, class| {
                assert!(class == ViewportClass::Immediate);
                egui::CentralPanel::default().show(&ctx, |ui| {
                    ui.label("Debug panel");

                    // menu to create new windows with information
                    ui.columns(2, |columns| {
                        columns[0].checkbox(&mut self.mem_widget.open, "Show memory panel");
                        columns[0].checkbox(&mut self.show_vram, "Show VRAM panel");
                        columns[1].checkbox(&mut self.cpu_widget.open, "Show CPU panel");
                        columns[1].checkbox(&mut self.ins_widget.open, "Show instruction panel");
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
                                false => EmulatorSend::StateUpdate(EmulatorState::Run(self.delay.parse().unwrap_or(0))),
                            }).unwrap();
                        }

                        ui.add(egui::TextEdit::singleline(&mut self.delay).desired_width(100.));

                        if ui.add(egui::Button::new("⏭")).clicked() {
                            self.inp_send.send(EmulatorSend::StateUpdate(EmulatorState::Step)).unwrap();
                        }
                    });
                });
            }
        );

        let emulator = self.emulator_ref.read();
        if self.cpu_widget.open { self.cpu_widget.draw(ctx, &emulator.cpu); }
        if self.ins_widget.open { self.ins_widget.draw(ctx)}
        if self.show_vram {eprintln!("not done yet"); self.show_vram = false;}
        if self.mem_widget.open { self.mem_widget.draw(&emulator.mem, ctx) }
    }
}