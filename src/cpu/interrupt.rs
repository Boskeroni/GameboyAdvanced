use crate::memory::Memory;

use super::registers::{Cpu, ProcessorMode};


enum CpuRegisters {
    Ime = 0x4000208,
    Ie = 0x4000200,
    If = 0x4000202,
}

pub fn handle_interrupts(memory: &mut Memory, cpu: &mut Cpu) {
    let interrupt_allowed = memory.read_u16(CpuRegisters::Ime as u32) & 1 == 1;
    if !interrupt_allowed {
        return;
    }
    let interrupts_enabled = memory.read_u16(CpuRegisters::Ie as u32);
    let interrupts_called = memory.read_u16(CpuRegisters::If as u32);

    let called_interrupts = interrupts_enabled & interrupts_called;
    if called_interrupts == 0 {
        return;
    }

    cpu.cpsr.mode = ProcessorMode::Interrupt;
    cpu.cpsr.t = false;

    let pc = cpu.get_register_mut(15);
    *pc = 0x18;
}