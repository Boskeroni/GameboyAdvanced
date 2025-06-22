#![cfg(feature = "debug")]
use gba_core::memory::{self, Memoriable};

use egui::{Label, TextEdit};
use gba_core::memory::{DMABaseAddress, Memory};
use std::cmp::min;
use std::ops::Range;
use egui::{scroll_area::ScrollBarVisibility, vec2, RichText, ViewportBuilder, ViewportClass, ViewportId, Widget};

const RANGES_NAMES: [&str; 9] = [
    "BIOS",
    "WRAM - On-board Work RAM",
    "WRAM - On-chip Work RAM",
    "I/O Registers",
    "BG/OBJ Palette RAM",
    "VRAM - Video RAM",
    "OAM - OBJ Attributes",
    "Game Pack ROM",
    "Game Pak SRAM",
];

pub struct MemoryWidget {
    pub open: bool,
    ranges_shown: [bool; 9],
}
impl MemoryWidget {
    pub fn new() -> Self {
        Self {
            open: true,
            ranges_shown: [false; 9]
        }
    }

    pub fn draw(&mut self, mem: &Box<Memory>, ctx: &egui::Context) {
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("memory panel"), 
            ViewportBuilder::default()
                .with_inner_size([560., 1000.])
                .with_resizable(false)
                .with_position([0., 0.])
                .with_title("memory"), 
            |ctx, class| {
                assert!(class == ViewportClass::Immediate);
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw_grid(mem, &self.ranges_shown, ui);
                });

                egui::SidePanel::right("memory_select").show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .scroll_bar_visibility(ScrollBarVisibility::VisibleWhenNeeded)
                        .min_scrolled_height(ctx.screen_rect().height())
                        .show(ui, |ui| {
                        ui.heading("Memory ranges");
                        for i in 0..RANGES_NAMES.len() {
                            ui.checkbox(&mut self.ranges_shown[i], RANGES_NAMES[i]);
                        }
                        ui.separator();

                        ui.heading("DMA registers");
                        for dma in 0..4 {
                            let dma_control = mem.read_u16(DMABaseAddress::Control as u32 + dma*0xC);
                            let is_running = (dma_control >> 15) & 1 == 1;
                            let headline_run_text = match is_running {
                                true => "running",
                                false => "dorment",
                            };

                            egui::CollapsingHeader::new(format!("DMA{dma}: {headline_run_text}")).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let mut run_text = format!("{is_running}");

                                    ui.add(Label::new(format!("is running:")));
                                    ui.add(TextEdit::singleline(&mut run_text));
                                });
                                ui.horizontal(|ui| {
                                    let mut control_text = format!("{:04X}", dma_control);
                                    ui.label(format!("control:"));
                                    ui.add(TextEdit::singleline(&mut control_text));
                                });

                                ui.horizontal(|ui| {
                                    let src = mem.read_u32_unrotated(DMABaseAddress::SAD as u32 + dma*0xC) & 0x0FFFFFFF;
                                    let mut src_text = format!("{:08X}", src);
                                    ui.label(format!("source:"));
                                    ui.add(TextEdit::singleline(&mut src_text));
                                });
                                ui.horizontal(|ui| {
                                    let dst = mem.read_u32_unrotated(DMABaseAddress::DAD as u32 + dma*0xC) & 0x0FFFFFFF;
                                    let mut dst_text = format!("{:08X}", dst);
                                    ui.label(format!("destination:"));
                                    ui.add(TextEdit::singleline(&mut dst_text));
                                });ui.horizontal(|ui| {
                                    let amount = mem.read_u32_unrotated(DMABaseAddress::Amount as u32 + dma*0xC) & 0x0FFFFFFF;
                                    let mut amount_text = format!("{:08X}", amount);
                                    ui.label(format!("amount:"));
                                    ui.add(TextEdit::singleline(&mut amount_text));
                                });
                            });
                        }
                    });
                    

                    ui.separator();
                    ui.heading("timer registers");
                    for i in 0..4 {
                        let is_running = (mem.read_u16(0x4000100 + (i*4) + 2)) >> 7 & 1 == 1;
                        let mut run_text = format!("{is_running}");
                        let label_text = format!("Timer{i} running:");
                        
                        ui.horizontal(|ui| {
                            ui.label(label_text);
                            ui.add(TextEdit::singleline(&mut run_text));
                        });
                    }
                });
            }
        );
    }
}

fn draw_grid(mem: &Box<Memory>, ranges_shown: &[bool; 9], ui: &mut egui::Ui) {
    if !ranges_shown.contains(&true) {
        return;
    }

    let col_per_row = 16;
    let mem_ranges = memory::get_memory_ranges();
    let total_rows = {
        let mut count = 0;
        for i in 0..ranges_shown.len() {
            if !ranges_shown[i] { continue; }
            count += mem_ranges[i].end - mem_ranges[i].start;
        }
        (count / col_per_row) as usize
    };
    let text_style = egui::TextStyle::Body;
    let text_style_height = ui.text_style_height(&text_style);

    egui::ScrollArea::vertical()
        .scroll_bar_visibility(ScrollBarVisibility::VisibleWhenNeeded)
        .show_rows(ui, text_style_height, total_rows, |ui, range| {
        egui::Grid::new("memory").striped(true).spacing(vec2(4., 2.,)).min_col_width(3.).show(ui, |ui| {
            // draw the top column
            egui::Label::new("").ui(ui);
            for i in 0..col_per_row {
                let colname = format!("{i:01X}");
                egui::Label::new(colname).ui(ui);
            }
            ui.end_row();

            let shown_memory_address = scroll_range_to_addresses(ranges_shown, &mem_ranges, range);
            for address in shown_memory_address {
                if address % col_per_row == 0 {
                    ui.label(format!("{:07X}", address & 0xFFFFFFF0));
                }
                let data = mem.read_u8(address);
                ui.label(RichText::new(format!("{data:02X}")).monospace());

                if address % col_per_row == col_per_row - 1 {
                    ui.end_row();
                }
            }
            ui.end_row();
        });
    });
}

fn scroll_range_to_addresses(
    ranges_shown: &[bool; 9], 
    mem_ranges: &[Range<u32>; 9], 
    scroll_range: Range<usize>
) -> Range<u32> {
    for i in 0..9 {
        if !ranges_shown[i] { continue; }
        let bottom = mem_ranges[i].start + scroll_range.start as u32 * 16;
        // im not going to deal with when two different memory ranges overlap
        let top = min(mem_ranges[i].start + scroll_range.end as u32 * 16, mem_ranges[i].end);

        return bottom..top;
    }
    return 0..0;
}