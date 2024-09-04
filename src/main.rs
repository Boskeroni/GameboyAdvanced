mod cpu;
mod memory;
mod ppu;

use std::io::{stdin, stdout, Write};

use cpu::registers::{Cpu, status_registers::Status};
use cpu::decode::{decode_arm, decode_thumb};
use cpu::execute_arm::execute_arm;
use cpu::execute_thumb::execute_thumb;
use cpu::decode::DecodedInstruction;
use minifb::{Window, WindowOptions};
use ppu::update_ppu;


const SCREEN_WIDTH: usize = 240;
const SCREEN_HEIGHT: usize = 160;

fn main() {
    let mut window = Window::new(
        "GBA emulator",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions::default(),
    ).unwrap();
    window.set_target_fps(60);



    let mut cpu_regs = Cpu {
        pc: 0x8000000,
        unbanked_registers: [0, 0, 0, 0, 0, 0, 0 ,0],
        double_banked_registers: [[0, 0], [0, 0], [0, 0], [0, 0], [0, 0], [0, 0]],
        many_banked_registers: [[0x03007F00, 0x03007F00, 0x03007F00, 0x03007F00, 0x03007F00, 0x03007F00], [0, 0, 0, 0, 0, 0]],
    };
    let mut status = Status::new();
    let mut memory = memory::create_memory("test/armwrestler.gba");

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
            let old_pc = cpu_regs.get_register(15, status.cpsr.mode);

            match instruction {
                Thumb(instr) => execute_thumb(decoded_opcode as u16, instr, &mut cpu_regs, &mut status, &mut memory),
                Arm(instr) => execute_arm(decoded_opcode, instr, &mut cpu_regs, &mut status, &mut memory),
            };

            debug_screen(&cpu_regs, instruction, decoded_opcode);

            let new_pc = cpu_regs.get_register(15, status.cpsr.mode);
            if old_pc != new_pc {
                fetched = None;
                decoded = None;
                continue;
            }
        }

        // Decode
        if let Some(opcode) = fetched {
            decoded = Some(match status.cpsr.t {
                true => DecodedInstruction::Thumb(decode_thumb(opcode as u16)),
                false => DecodedInstruction::Arm(decode_arm(opcode)),
            });

            decoded_opcode = opcode;
        }

        // Fetch
        fetched = Some(match status.cpsr.t {
            true => memory.read_u16(cpu_regs.get_pc_thumb()) as u32,
            false => memory.read_u32(cpu_regs.get_pc_arm()),
        });

        let window_buffer = update_ppu(&mut memory);
        window.update_with_buffer(&window_buffer, SCREEN_WIDTH, SCREEN_HEIGHT).unwrap();
    }
}

fn debug_screen(cpu: &Cpu, instr: DecodedInstruction, opcode: u32) {
    println!("============= DEBUG SCREEN =============");
    println!("r0, r1, r2, r3, r4, r5, r6, r7: {:X?}", cpu.unbanked_registers);
    println!("r8, r9, r10, r11, r12: {:X?}", cpu.double_banked_registers);
    println!("r13, r14: {:X?}", cpu.many_banked_registers);
    println!("pc: {:X}", cpu.pc);
    println!("========= Opcode completed: {instr:?}, {opcode:X} ============");
    
    stdout().flush().unwrap();
    let mut temp = String::new();
    //stdin().read_line(&mut temp).unwrap();
    println!("");
}