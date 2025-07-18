pub mod cpu;
pub mod ppu;
pub mod joypad;
pub mod apu;
pub mod mem;

use cpu::{
    execute_arm::execute_arm, 
    execute_thumb::execute_thumb, 
    handle_interrupts, 
    Cpu, Fde,
};

use mem::bus::*;
use joypad::init_joypad;
use ppu::*;

use crate::{apu::tick_apu, mem::memory::{self, dma_tick, update_timer}};

pub struct Emulator {
    pub cpu: Cpu,
    pub ppu: Ppu,
    pub bus: Bus,
    pub fde: Fde,
    pub cycles: u32,
}
impl Emulator {
    pub fn new(filename: &str, from_bios: bool) -> Self {
        let cpu = match from_bios {
            true => Cpu::from_bios(),
            false => Cpu::new(),
        };
        let ppu = Ppu::new();
        let mut memory = memory::create_memory(filename);
        init_joypad(&mut memory);
        memory.sys_write_u16(0x4000088, 0b0000_0010_0000_0000);

        let bus = Bus::new(memory, from_bios);
        let fde = Fde::new();

        Self {
            cpu,
            ppu,
            bus,
            fde,
            cycles: 0,
        }
    }
}

pub fn run_single_step(emu: &mut Emulator) -> bool {
    // update the timer
    // add 1 for now, make it more accurate later
    update_timer(&mut emu.bus.mem, &mut emu.cycles, 20);
    let active_dma = dma_tick(&mut emu.bus.mem);

    tick_apu();
    tick_ppu(&mut emu.ppu, &mut emu.bus);
    if emu.ppu.new_screen {
        emu.ppu.new_screen = false;
        return true;
    }
    if active_dma {
        return false;
    }

    handle_interrupts(&mut emu.bus, &mut emu.cpu);
    if emu.cpu.halted {
        return false;
    }

    handle_cpu(&mut emu.cpu, &mut emu.bus);
    if emu.bus.should_halt_cpu() {
        emu.cpu.halted = true;
    }
    return false;
}

pub fn handle_cpu<M: CpuInterface>(cpu: &mut Cpu, mem: &mut M) {
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