#![cfg(feature = "debug")]

use egui::{ViewportBuilder, ViewportClass, ViewportId};
use gba_core::cpu::Cpu;

pub struct CpuWidget {
    pub open: bool,
}
impl CpuWidget {
    pub fn new() -> Self {
        Self {
            open: false,
        }
    }

    pub fn draw(&self, ctx: &egui::Context, cpu: &Cpu) {
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("cpu channel"), 
            ViewportBuilder::default(), 
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
                });
            }
        );
    }
}