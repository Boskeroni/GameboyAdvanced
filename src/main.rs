mod debug;

use core::memory::Memory;
use core::ppu::Ppu;
use core::{memory, Fde};
use core::cpu::Cpu;
use core::joypad::{self, Button};
use winit::keyboard::KeyCode;


const SCREEN_HEIGHT: usize = 160;
const SCREEN_WIDTH: usize = 240;
const FRAME_TIME: u128 = 1_000_000_000 / 60;
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

fn convert_to_joypad(code: KeyCode) -> joypad::Button {
    use KeyCode::*;
    use joypad::Button::*;
    match code {
        KeyZ => Button::Select,
        KeyX => Start,
        ArrowLeft => Left,
        ArrowRight => Right,
        ArrowDown => Down,
        ArrowUp => Up,
        KeyK => A,
        KeyL => B,
        KeyQ => L,
        KeyP => R,
        _ => Other,
    }
}

fn main() {
    // right now all it will show is the debug view.
    // eventually this should be togglable

    let context = GbaContext::new("roms/FuzzArmAny.gba");
    debug::setup_debug(context);
}