use core::{cpu::{decode::{decode_arm, decode_thumb, DecodedInstruction}, execute_arm::execute_arm, execute_thumb::execute_thumb, handle_interrupts, Cpu}, joypad::{self, joypad_press, joypad_release}, memory::{self, dma_tick, update_timer, Memory}, ppu::{tick_ppu, Ppu}, Fde};
use std::sync::mpsc::Receiver;
use egui::{Event, Key};
use crate::debug::{DebugCommand, DebugDataBackend};

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

const FROM_BIOS: bool = false;
pub struct Emulator {
    cpu: Cpu,
    ppu: Ppu,
    memory: Box<Memory>,
    fde: Fde,
    cycles: u32,
}
impl Emulator {
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

    fn send_debug_info() {
        todo!();
    }
}

pub fn run_emulator(
    mut emulator: Emulator,
    debug: DebugDataBackend,
    input: Receiver<egui::Event>,
) {
    let mut is_running = true;
    let mut step = false;

    loop {
        if let Ok(message) = debug.cnt_dbg.try_recv() {
            match message {
                DebugCommand::Run => is_running = true,
                DebugCommand::Step => step = true,
                DebugCommand::Stop => is_running = false,
            }
        }

        if !is_running && !step {
            continue;
        }
        step = false;

        if let Ok(event) = input.try_recv() {
            if let Event::Key {key, pressed, ..} = event {
                let button = convert_to_joypad(&key);
                match pressed {
                    true => joypad_press(button, &mut emulator.memory),
                    false => joypad_release(button, &mut emulator.memory),
                }
            }
        }

        let redraw_needed = gba_step(
            &mut emulator.cpu, 
            &mut emulator.memory, 
            &mut emulator.ppu, 
            &mut emulator.fde, 
            &mut emulator.cycles
        );

        if redraw_needed {
            debug.ppu_dbg.send(emulator.ppu.stored_screen.clone()).unwrap();
            emulator.ppu.acknowledge_frame();
        }
    }
}

fn gba_step(
    cpu: &mut Cpu, 
    mem: &mut Memory, 
    ppu: &mut Ppu, 
    fde: &mut Fde, 
    cycles: &mut u32,
) -> bool {
    update_timer(mem, cycles, 1);
    let dma_is_active = dma_tick(mem);

    tick_ppu(ppu, mem);
    if ppu.new_screen {
        ppu.new_screen = false;
        return true;
    }

    if dma_is_active {
        return false;
    }

    let ahead_by = if fde.fetched == None { 0 } else if fde.decoded == None { 1 } else { 2 };
    handle_interrupts(mem, cpu, ahead_by);
    if cpu.clear_pipeline {
        fde.fetched = None;
        fde.decoded = None;
        cpu.clear_pipeline = false;
        return false;
    }

    // Execute
    if let Some(instruction) = fde.decoded {
        use DecodedInstruction::*;

        match instruction {
            Thumb(instr) => execute_thumb(fde.decoded_opcode as u16, instr, cpu, mem),
            Arm(instr) => execute_arm(fde.decoded_opcode, instr, cpu, mem),
        };

        if cpu.clear_pipeline {
            fde.fetched = None;
            fde.decoded = None;
            cpu.clear_pipeline = false;
            return false;
        }
    }

    // Decode
    if let Some(opcode) = fde.fetched {
        fde.decoded = Some(match cpu.cpsr.t {
            true => DecodedInstruction::Thumb(decode_thumb(opcode as u16)),
            false => DecodedInstruction::Arm(decode_arm(opcode)),
        });

        fde.decoded_opcode = opcode;
    }

    // Fetch
    fde.fetched = Some(match cpu.cpsr.t {
        true => mem.read_u16(cpu.get_pc_thumb()) as u32,
        false => mem.read_u32(cpu.get_pc_arm()),
    });

    return false;
}