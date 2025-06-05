#![cfg(feature = "debug")]
#![allow(unused)]
use egui::{ViewportBuilder, ViewportClass, ViewportId};

pub struct InstructionWidget {
    pub open: bool,
    instructions: Vec<String>,
}
impl InstructionWidget {
    pub fn new() -> Self {
        Self {
            open: false,
            instructions: Vec::new(),
        }
    }

    pub fn draw(&self, ctx: &egui::Context) {
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("instruction panel"), 
            ViewportBuilder::default(), 
            |ctx, class| {
                assert!(class == ViewportClass::Immediate);
                egui::CentralPanel::default().show(ctx, |ui| {
                    
                });
            }
        );
    }
}