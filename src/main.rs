mod cpu;
mod memory;
mod ppu;

use std::fs::File;
use std::io::Write;

use cpu::handle_interrupts;
use cpu::registers::{Cpu, status_registers::Status};
use cpu::decode::{decode_arm, decode_thumb};
use cpu::execute_arm::execute_arm;
use cpu::execute_thumb::execute_thumb;
use cpu::decode::DecodedInstruction;
use memory::update_timer;
use minifb::{Window, WindowOptions};
use ppu::update_ppu;


const SCREEN_WIDTH: usize = 240;
const SCREEN_HEIGHT: usize = 160;

fn main() {
    let mut window = Window::new(
        "GBA emulter", 
        SCREEN_WIDTH, 
        SCREEN_HEIGHT, 
        WindowOptions::default()
    ).unwrap();

    let mut cpu_regs = Cpu {
        pc: 0x000000,
        unbanked_registers: [0, 0, 0, 0, 0, 0, 0 ,0],
        double_banked_registers: [[0, 0], [0, 0], [0, 0], [0, 0], [0, 0]],
        many_banked_registers: [[0, 0, 0, 0, 0, 0], [0, 0, 0, 0, 0, 0]],
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
    let mut f = File::create("debug/debug.txt").expect("the file couldnt be opened");
    let mut total_cycles = 0;
    loop {
        // update the timer
        // add 1 for now, make it more accurate later
        update_timer(&mut memory, &mut total_cycles, 1);

        // check for any interrupts
        handle_interrupts(&mut memory, &mut status, &mut cpu_regs);

        // Execute
        if let Some(instruction) = decoded {
            let old_pc = cpu_regs.get_register(15, status.cpsr.mode);
            let old_regs = cpu_regs.clone();

            match instruction {
                Thumb(instr) => execute_thumb(decoded_opcode as u16, instr, &mut cpu_regs, &mut status, &mut memory),
                Arm(instr) => execute_arm(decoded_opcode, instr, &mut cpu_regs, &mut status, &mut memory),
            };

            debug_screen(&cpu_regs, instruction, decoded_opcode, &status, &old_regs, &mut f);

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

        let pot_buffer = update_ppu(&mut memory);
        if let Some(buffer) = pot_buffer {
            let converted_buffer = convert_u16_color(buffer);
            window.update_with_buffer(&converted_buffer, SCREEN_WIDTH, SCREEN_HEIGHT).unwrap();
        }
    }
}

fn convert_u16_color(screen: Vec<u16>) -> Vec<u32> {
    screen.iter().map(|c| {
        let c = *c as u32;
        c*c
    }).collect()
}


fn debug_screen(cpu: &Cpu, instr: DecodedInstruction, opcode: u32, status: &Status, old_regs: &Cpu, f: &mut File) {
    writeln!(f, "======== DEBUG ========").unwrap();
    let mut temp = Vec::new();
    for i in 0..=14 {
        let old_value = old_regs.get_register(i, status.cpsr.mode);
        let new_value = cpu.get_register(i, status.cpsr.mode);
        temp.push(new_value);

        if old_value == new_value { continue; }
        write!(f, "r{i} ==> {old_value:X} = {new_value:X}... ").unwrap();
    }

    writeln!(f, "").unwrap();
    writeln!(f, "{temp:X?}").unwrap();
    writeln!(f, "pc: {:X}, from: {:X}", cpu.pc, old_regs.pc).unwrap();
    writeln!(f, "status: {:?}", status.cpsr).unwrap();
    writeln!(f, "======= {instr:?} {opcode:X} ========= ").unwrap();
    writeln!(f, "").unwrap();
    
    // println!("======== DEBUG ========");
    // let mut temp = Vec::new();
    // for i in 0..=14 {
    //     let old_value = old_regs.get_register(i, status.cpsr.mode);
    //     let new_value = cpu.get_register(i, status.cpsr.mode);
    //     temp.push(new_value);

    //     if old_value == new_value { continue; }
    //     print!("r{i} ==> {old_value:X} = {new_value:X}... ");
    // }

    // println!("");
    // println!("{temp:X?}");
    // println!("pc: {:X}, from: {:X}", cpu.pc, old_regs.pc);
    // println!("status: {:?}", status.cpsr);
    // println!("======= {instr:?} {opcode:X} ========= ");
    // println!("");

    // stdout().flush().unwrap();
    // let mut temp = String::new();

    // use std::io::stdin;
    // stdin().read_line(&mut temp).unwrap();
}