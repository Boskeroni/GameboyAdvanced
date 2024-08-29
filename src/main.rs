mod cpu;
mod memory;

use cpu::registers::{Cpu, status_registers::Status};
use cpu::decode::{decode_arm, decode_thumb};
use cpu::execute_arm::execute_arm;
use cpu::execute_thumb::execute_thumb;
use cpu::decode::DecodedInstruction;

fn main() {
    let mut cpu_regs = Cpu {
        pc: 0x8000000,
        unbanked_registers: [0, 0, 0, 0, 0, 0, 0 ,0],
        double_banked_registers: [[0, 0], [0, 0], [0, 0], [0, 0], [0, 0], [0, 0]],
        many_banked_registers: [[0x03007F00, 0, 0x03007FE0, 0, 0x03007FA0, 0], [0, 0, 0, 0, 0, 0]],
    };
    let mut status_registers = Status::new();
    let mut memory = memory::create_memory("test/arm.gba");

    let mut fetched: Option<u32> = None;

    let mut decoded: Option<DecodedInstruction> = None;
    let mut decoded_opcode: u32 = 0;

    // each loop represents FDE step
    // since all the steps of the FDE cycle take place in one turn, 
    // it technically doesnt matter the order and so I will do EDF for convenience
    use DecodedInstruction::*;
    loop {
        // Execute
        if let Some(instruction) = decoded {
            let old_pc = cpu_regs.get_register(15, status_registers.cpsr.mode);

            print!("{:#X}, ", decoded_opcode);
            print!("{:#X}, ", cpu_regs.get_register(15, status_registers.cpsr.mode));
            print!("{:?}, ", instruction);
            println!("");

            match instruction {
                Thumb(instr) => execute_thumb(decoded_opcode as u16, instr, &mut cpu_regs, &mut status_registers, &mut memory),
                Arm(instr) => execute_arm(decoded_opcode, instr, &mut cpu_regs, &mut status_registers, &mut memory),
            };

            

            if old_pc != cpu_regs.get_register(15, status_registers.cpsr.mode) {
                fetched = None;
                decoded = None;
                continue;
            }
        }

        // Decode
        if let Some(opcode) = fetched {
            decoded = Some(match status_registers.cpsr.t {
                true => DecodedInstruction::Thumb(decode_thumb(opcode as u16)),
                false => DecodedInstruction::Arm(decode_arm(opcode)),
            });

            decoded_opcode = opcode;
        }

        // Fetch
        fetched = Some(match status_registers.cpsr.t {
            true => memory.read_u16(cpu_regs.get_pc()) as u32,
            false => memory.read_u32(cpu_regs.get_pc()),
        });
    }
}