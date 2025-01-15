mod cpu;
mod memory;
mod ppu;

use std::fs::File;
use std::io::{stdout, Write};

use cpu::interrupt::handle_interrupts;
use cpu::registers::{Cpu, status_registers::CpuStatus};
use cpu::decode::{decode_arm, decode_thumb};
use cpu::execute_arm::execute_arm;
use cpu::execute_thumb::execute_thumb;
use cpu::decode::DecodedInstruction;
use memory::update_timer;
use minifb::{Key, Window, WindowOptions};
use ppu::{update_ppu, PpuState};


const SCREEN_WIDTH: usize = 240;
const SCREEN_HEIGHT: usize = 160;

fn main() {
    let mut window = Window::new(
        "GBA emulter", 
        SCREEN_WIDTH, 
        SCREEN_HEIGHT, 
        WindowOptions::default()
    ).unwrap();
    window.set_target_fps(60);

    let mut cpu_regs = Cpu {
        pc: 0x000000,
        unbanked_registers: [0, 0, 0, 0, 0, 0, 0 ,0],
        double_banked_registers: [[0, 0], [0, 0], [0, 0], [0, 0], [0, 0]],
        many_banked_registers: [[0x03007F00, 0, 0x03007FE0, 0, 0x03007FA0, 0], [0, 0, 0, 0, 0, 0]],
        clear_pipeline: false,
    };
    let mut status = CpuStatus::new();
let mut memory = memory::create_memory("test/armwrestler.gba");
    let mut ppu = PpuState::new();

    let mut fetched: Option<u32> = None;

    let mut decoded: Option<DecodedInstruction> = None;
    let mut decoded_opcode: u32 = 0;

    // each loop represents FDE step
    // since all the steps of the FDE cycle take place in one turn, 
    // it technically doesnt matter the order and so I will do EDF for convenience
    use DecodedInstruction::*;
    let mut f = File::create("debug/debug.txt").expect("the file couldnt be opened");
    let mut total_cycles = 0;
    let mut has_booted = false;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // update the timer
        // add 1 for now, make it more accurate later
        update_timer(&mut memory, &mut total_cycles, 1);
        update_ppu(&mut ppu, &mut memory);

        if ppu.new_screen {
            let mut new_frame = Vec::new();
            for pixel in &ppu.stored_screen {
                new_frame.push(*pixel as u32);
            }
            window.update_with_buffer(&new_frame, SCREEN_WIDTH, SCREEN_HEIGHT).expect("i wonder what error this will be");
            ppu.new_screen = false;
        }

        // check for any interrupts
        handle_interrupts(&mut memory, &mut status, &mut cpu_regs);

        // Execute
        if let Some(instruction) = decoded {
            // debug reasons
            let old_regs = cpu_regs.clone();
            let old_stat = status.clone();

            match instruction {
                Thumb(instr) => execute_thumb(decoded_opcode as u16, instr, &mut cpu_regs, &mut status, &mut memory),
                Arm(instr) => execute_arm(decoded_opcode, instr, &mut cpu_regs, &mut status, &mut memory),
            };

            debug_screen(&cpu_regs, instruction, decoded_opcode, &status, &old_regs, &mut f, &old_stat);
            let new_pc = cpu_regs.get_register(15, status.cpsr.mode);

            if cpu_regs.clear_pipeline {
                fetched = None;
                decoded = None;
                cpu_regs.clear_pipeline = false;
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

        if has_booted && cpu_regs.pc == 0 {
            panic!("called back to the start");
        }

        // Fetch
        fetched = Some(match status.cpsr.t {
            true => memory.read_u16(cpu_regs.get_pc_thumb()) as u32,
            false => memory.read_u32(cpu_regs.get_pc_arm()),
        });
        has_booted = true;
    }
}

fn debug_screen(
    cpu: &Cpu, 
    instr: DecodedInstruction, 
    opcode: u32, 
    status: &CpuStatus, 
    old_regs: &Cpu, 
    f: &mut File,
    old_stat: &CpuStatus
) {
    writeln!(f, "======== DEBUG ========").unwrap();
    let mut temp = Vec::new();
    for i in 0..=14 {
        let old_value = old_regs.get_register(i, status.cpsr.mode);
        let new_value = cpu.get_register(i, status.cpsr.mode);
        temp.push(new_value);

        if old_value == new_value { continue; }
        write!(f, "r{i} ==> {new_value:X}... ").unwrap();
    }

    writeln!(f, "").unwrap();
    writeln!(f, "{temp:X?}").unwrap();
    writeln!(f, "pc: {:X}, from: {:X}", cpu.pc, old_regs.pc).unwrap();
    writeln!(f, "status: {:?}", status.cpsr).unwrap();
    writeln!(f, "======= {instr:?} {opcode:X} ========= ").unwrap();
    writeln!(f, "").unwrap();

    let mut temp = String::new();
    std::io::stdin().read_line(&mut temp).unwrap();
    print!("{instr:?} | {opcode:X} | ");

    for i in 0..=15 {
        let old_value = old_regs.get_register(i, status.cpsr.mode);
        let new_value = cpu.get_register(i, status.cpsr.mode);

        if old_value == new_value { continue; }
        print!("r{i} ==> {old_value:X} = {new_value:X} ");
    }
    print!(" | ");
    if old_stat.cpsr.c != status.cpsr.c {
        let clear = status.cpsr.c;
        print!("c = {clear} ");
    }
    if old_stat.cpsr.z != status.cpsr.z {
        let zero = status.cpsr.z;
        print!("z = {zero} ");
    }
    if old_stat.cpsr.n != status.cpsr.n {
        let negative = status.cpsr.n;
        print!("n = {negative} ");
    }
    if old_stat.cpsr.v != status.cpsr.v {
        let overflow = status.cpsr.v;
        print!("v = {overflow} ");
    }
    stdout().flush().unwrap();
}