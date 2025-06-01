#![cfg(feature = "debug")]
use std::{sync::{mpsc::{Receiver, Sender}, Arc, Mutex}};
use eframe::egui;
use egui_memory_editor::MemoryEditor;
use core::Emulator;
use crate::emulator::EmulatorCommand;

pub fn setup_debug(
    emulator: Arc<Mutex<Emulator>>, 
    ui_recv: Receiver<egui::Context>,
    cmd_send: Sender<EmulatorCommand>,
) {
    let mut memory_editor = MemoryEditor::new()
        .with_address_range("BIOS", 0..0x4000)
        .with_address_range("WRAM", 0x2000000..0x2040000)
        .with_address_range("WRAM", 0x3000000..0x3008000)
        .with_address_range("IO", 0x4000000..0x40003FF)
        .with_address_range("Pallete", 0x5000000..0x5000400)
        .with_address_range("VRAM", 0x6000000..0x6018000)
        .with_address_range("OAM", 0x7000000..0x7000400)
        .with_window_title("Memory");
    memory_editor.options.is_resizable_column = false;
    memory_editor.options.column_count = 16;

    debug_loop(emulator, memory_editor, ui_recv, cmd_send);
}

fn debug_loop(
    emulator_arc: Arc<Mutex<Emulator>>, 
    mem_editor: MemoryEditor, 
    ui_recv: Receiver<egui::Context>,
    cmd_send: Sender<EmulatorCommand>,
) {
    let mut show_memory_panel = false;
    let mut show_vram_panel = false;
    let mut show_cpu_panel = false;
    let mut show_instruction_panel = false;

    let mut is_paused = false;

    loop {
        // this will wait, which is good
        let ctx = ui_recv.recv().unwrap();

        // this is mostly so i can show i access it
        let _emulator = emulator_arc.lock().unwrap();
        egui::CentralPanel::default().show(&ctx, |ui| {
            ui.label("Debug panel");

            // menu to create new windows with information
            ui.checkbox(&mut show_memory_panel, "Show memory panel");
            ui.checkbox(&mut show_vram_panel,"Show VRAM panel");
            ui.checkbox(&mut show_cpu_panel,"Show CPU panel");
            ui.checkbox(&mut show_instruction_panel,"Show instruction panel");

            // pause and step buttons
            let pause_button_text = match is_paused {
                false => "⏸",
                true => "⏵"
            };
            if ui.add(egui::Button::new(pause_button_text)).clicked() {
                is_paused = !is_paused;
                cmd_send.send(match is_paused {
                    true => EmulatorCommand::Pause,
                    false => EmulatorCommand::Run,
                }).unwrap();
            }

            if ui.add(egui::Button::new("⏭")).clicked() {
                cmd_send.send(EmulatorCommand::Step).unwrap();
            }
        });
    }
}