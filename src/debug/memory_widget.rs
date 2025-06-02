#![cfg(feature = "debug")]

use gba_core::memory::Memory;
use std::{cmp::min, time::Instant};
use std::ops::Range;
use egui::{scroll_area::ScrollBarVisibility, vec2, RichText, ViewportBuilder, ViewportClass, ViewportId, Widget};

const RANGES_NAMES: [&str; 7] = [
    "BIOS",
    "WRAM - On-board Work RAM",
    "WRAM - On-chip Work RAM",
    "I/O Registers",
    "BG/OBJ Palette RAM",
    "VRAM - Video RAM",
    "OAM - OBJ Attributes",
];

pub struct MemoryWidget {
    pub open: bool,
    ranges_shown: [bool; 7],
}
impl MemoryWidget {
    pub fn new() -> Self {
        Self {
            open: true,
            ranges_shown: [false; 7]
        }
    }

    pub fn draw(&mut self, mem: &Memory, ctx: &egui::Context) {
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("memory panel"), 
            ViewportBuilder::default()
                .with_inner_size([550., 600.])
                .with_title("memory"), 
            |ctx, class| {
                assert!(class == ViewportClass::Immediate);
                egui::CentralPanel::default().show(ctx, |ui| {
                    // this is massive, so split into seperate function
                    ui.label("memory table");
                    ui.separator();

                    draw_grid(mem, &self.ranges_shown, ui);
                });

                egui::SidePanel::right("memory_select").show(ctx, |ui| {
                    for i in 0..RANGES_NAMES.len() {
                        ui.checkbox(&mut self.ranges_shown[i], RANGES_NAMES[i]);
                    }
                });
            }
        );
    }
}

fn draw_grid(mem: &Memory, ranges_shown: &[bool; 7], ui: &mut egui::Ui) {
    if !ranges_shown.contains(&true) {
        return;
    }

    let col_per_row = 16;
    let mem_ranges = Memory::get_memory_ranges();
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
    ranges_shown: &[bool; 7], 
    mem_ranges: &[Range<u32>; 7], 
    scroll_range: Range<usize>
) -> Range<u32> {
    for i in 0..7 {
        if !ranges_shown[i] { continue; }
        let bottom = mem_ranges[i].start + scroll_range.start as u32 * 16;
        // im not going to deal with when two different memory ranges overlap
        let top = min(mem_ranges[i].start + scroll_range.end as u32 * 16, mem_ranges[i].end);

        return bottom..top;
    }
    return 0..0;
}