pub mod cpu;
pub mod memory;
pub mod ppu;
pub mod joypad;
mod bus; 

use cpu::{
    decode::*, 
    execute_arm::execute_arm, 
    execute_thumb::execute_thumb, 
    handle_interrupts, 
    Cpu, Fde,
};
use memory::*;
use ppu::*;

const FROM_BIOS: bool = false;
pub struct Emulator {
    pub cpu: Cpu,
    pub ppu: Ppu,
    pub mem: Box<Memory>,
    pub fde: Fde,
    pub cycles: u32,
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
            mem: memory,
            fde,
            cycles: 0,
        }
    }
}

pub fn run_frame(emulator: &mut Emulator) {
    emulator.ppu.acknowledge_frame();

    loop {
        let res = run_single_step(emulator);
        if res {
            return;
        }
    }
}

pub fn run_single_step(emu: &mut Emulator) -> bool {
    // update the timer
    // add 1 for now, make it more accurate later
    update_timer(&mut emu.mem, &mut emu.cycles, 1);
    let active_dma = dma_tick(&mut emu.mem);

    tick_ppu(&mut emu.ppu, &mut emu.mem);
    if emu.ppu.new_screen {
        emu.ppu.new_screen = false;
        return true;
    }
    if active_dma {
        return false;
    }

    let ahead_by = if emu.fde.fetched == None { 0 } else if emu.fde.decoded == None { 1 } else { 2 };
    handle_interrupts(&mut emu.mem, &mut emu.cpu, ahead_by);
    if emu.cpu.clear_pipeline {
        emu.fde.fetched = None;
        emu.fde.decoded = None;
        emu.cpu.clear_pipeline = false;
        return false;
    }

    // Execute
    if let Some(instruction) = emu.fde.decoded {
        use DecodedInstruction::*;

        match instruction {
            Thumb(instr) => execute_thumb(emu.fde.decoded_opcode as u16, instr, &mut emu.cpu, &mut emu.mem),
            Arm(instr) => execute_arm(emu.fde.decoded_opcode, instr, &mut emu.cpu, &mut emu.mem),
        };

        if emu.cpu.clear_pipeline {
            emu.fde.fetched = None;
            emu.fde.decoded = None;
            emu.cpu.clear_pipeline = false;
            return false;
        }
    }

    // Decode
    if let Some(opcode) = emu.fde.fetched {
        emu.fde.decoded = Some(match emu.cpu.cpsr.t {
            true => DecodedInstruction::Thumb(decode_thumb(opcode as u16)),
            false => DecodedInstruction::Arm(decode_arm(opcode)),
        });

        emu.fde.decoded_opcode = opcode;
    }

    // Fetch
    emu.fde.fetched = Some(match emu.cpu.cpsr.t {
        true => emu.mem.read_u16(emu.cpu.get_pc_thumb()) as u32,
        false => emu.mem.read_u32(emu.cpu.get_pc_arm()),
    });

    return false;
}