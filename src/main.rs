mod debug;

use core::memory::Memory;
use core::ppu::Ppu;
use core::{memory, Fde};
use core::cpu::Cpu;
use core::joypad;
use egui::Key;

const SCREEN_HEIGHT: usize = 160;
const SCREEN_WIDTH: usize = 240;
const FROM_BIOS: bool = false;
struct GbaContext {
    cpu: Cpu,
    ppu: Ppu,
    memory: Box<Memory>,
    fde: Fde,
    cycles: u32,
}
impl GbaContext {
    pub fn new(filename: &str) -> Self {
        let cpu = match FROM_BIOS {
            true => Cpu::from_bios(),
            false => Cpu::new(),
        };
        let ppu = Ppu::new();
        let memory = memory::create_memory(filename);
        let fde = Fde::new();

        Self {
            cpu,
            ppu,
            memory,
            fde,
            cycles: 0,
        }
    }
}

fn convert_to_joypad(code: &Key) -> joypad::Button {
    use joypad::Button;
    use egui::Key;
    match code {
        Key::Z => Button::Select,
        Key::X => Button::Start,
        Key::ArrowLeft => Button::Left,
        Key::ArrowRight => Button::Right,
        Key::ArrowDown => Button::Down,
        Key::ArrowUp => Button::Up,
        Key::K => Button::A,
        Key::L => Button::B,
        Key::Q => Button::L,
        Key::P => Button::R,
        _ => Button::Other,
    }
}

fn main() {
    // right now all it will show is the debug view.
    // eventually this should be togglable

    let context = GbaContext::new("roms/FuzzArmAny.gba");
    debug::setup_debug(context);
}