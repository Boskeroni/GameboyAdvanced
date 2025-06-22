pub mod cpu;
pub mod memory;
pub mod ppu;
pub mod joypad;
mod bus; 

use cpu::{
    execute_arm::execute_arm, 
    execute_thumb::execute_thumb, 
    handle_interrupts, 
    Cpu, Fde,
};
use joypad::init_joypad;
use memory::*;
use ppu::*;

use crate::cpu::assemblify;

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
        let mut memory = memory::create_memory(filename);
        init_joypad(&mut memory);
        memory.write_io(0x4000088, 0b0000_0010_0000_0000);

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

    handle_interrupts(&mut emu.mem, &mut emu.cpu);
    if emu.cpu.halted {
        return false;
    }

    handle_cpu(&mut emu.cpu, &mut emu.mem);
    if emu.mem.should_halt_cpu() {
        emu.cpu.halted = true;
    }
    return false;
}

pub fn handle_cpu<M: Memoriable>(cpu: &mut Cpu, mem: &mut M) {
    // Execute
    if let Some(instruction) = cpu.fde.decoded_opcode {        
        match cpu.cpsr.t {
            true => {
                // println!("{}", assemblify::to_thumb_assembly(instruction as u16));
                execute_thumb(instruction as u16, cpu, mem)
            }
            false => {
                // println!("{}", assemblify::to_arm_assembly(instruction));
                execute_arm(instruction, cpu, mem)
            },
        };
    }
    
    // if there was a clear, need to get new fetched
    if let None = cpu.fde.fetched_opcode {
        let fetch = match cpu.cpsr.t {
            true => mem.read_u16(cpu.get_pc_thumb()) as u32,
            false => mem.read_u32_unrotated(cpu.get_pc_arm()),
        };
        cpu.fde.fetched_opcode = Some(fetch);
    }
    
    // move the fetched to decoded
    cpu.fde.decoded_opcode = cpu.fde.fetched_opcode.clone();
    let fetch = match cpu.cpsr.t {
        true => mem.read_u16(cpu.get_pc_thumb()) as u32,
        false => mem.read_u32_unrotated(cpu.get_pc_arm()),
    };
    cpu.fde.fetched_opcode = Some(fetch);
}