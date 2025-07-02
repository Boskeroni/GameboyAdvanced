#![cfg(feature = "debug")]

use egui::{TextEdit, ViewportBuilder, ViewportClass, ViewportId};
use gba_core::cpu::{convert_psr_u32, Cpu};

pub struct CpuWidget {
    pub open: bool,
}
impl CpuWidget {
    pub fn new() -> Self {
        Self {
            open: true,
        }
    }

    pub fn draw(&self, ctx: &egui::Context, cpu: &Cpu) {
        let mut cpsr: String = format!("{:032b}", convert_psr_u32(&cpu.cpsr));

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("cpu channel"), 
            ViewportBuilder::default()
                .with_title("cpu")
                .with_position([780., 350.])
                .with_inner_size([350., 200.,])
                .with_resizable(false), 
            |ctx, class| {
                assert!(class == ViewportClass::Immediate);
                egui::CentralPanel::default().show(ctx, |ui| {
                    let num_columns = 3;
                    ui.columns(num_columns, |columns| {
                        for i in 0..=15 {
                            columns[i % num_columns].label(
                                format!("reg {i}: {:08X}", cpu.get_register(i as u8))
                            );
                        }
                    });

                    ui.separator();
                    ui.monospace("       NZCV--------------------IFT43210"); // hmm yes very good ui
                    ui.horizontal(|ui| {
                        ui.label("CPSR: ");
                        ui.add(egui::TextEdit::singleline(&mut cpsr));
                    });

                    let mut halt_text = format!("{}", cpu.halted);
                    ui.horizontal(|ui| {
                        ui.label(format!("Halted:"));
                        ui.add(TextEdit::singleline(&mut halt_text));
                    });
                });
            }
        );
    }
}