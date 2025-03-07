mod cpu;
mod memory;
mod ppu;
mod joypad;

use cpu::{
    handle_interrupts,
    Cpu,
    decode::{DecodedInstruction, decode_arm, decode_thumb},
    execute_arm::execute_arm,
    execute_thumb::execute_thumb,
};
use joypad::setup_joypad;
use memory::{update_timer, Memory};
use pixels::{Pixels, SurfaceTexture};
use ppu::{tick_ppu, Ppu};

use std::fs::File;
use std::io::{stdout, Write};
use std::thread;
use std::time::{Duration, Instant};

use winit::{dpi::LogicalSize, event::{Event, WindowEvent}, keyboard::PhysicalKey};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

const SCREEN_WIDTH: usize = 240;
const SCREEN_HEIGHT: usize = 160;
const FPS: u128 = 60;
const FRAME_TIME: u128 = 1_000_000_000 / FPS;

const BIOS: bool = false;
const DEBUG: bool = false;
const PRINT: bool = true;
const STEP: bool = false;

#[derive(Default)]
struct Fde {
    fetched: Option<u32>,
    decoded: Option<DecodedInstruction>,
    decoded_opcode: u32,
}

fn gba_frame(
    cpu: &mut Cpu, 
    mem: &mut Memory, 
    ppu: &mut Ppu, 
    fde: &mut Fde, 
    cycles: &mut u32,
    f: &mut File,
) {
    ppu.acknowledge_frame();

    loop {
        // update the timer
        // add 1 for now, make it more accurate later
        update_timer(mem, cycles, 1);
        tick_ppu(ppu, mem);
        if ppu.new_screen {
            ppu.new_screen = false;
            return;
        }
        handle_interrupts(mem, cpu);

        // Execute
        if let Some(instruction) = fde.decoded {
            // debug reasons
            let old_regs = cpu.clone();

            use DecodedInstruction::*;
            match instruction {
                Thumb(instr) => execute_thumb(fde.decoded_opcode as u16, instr, cpu, mem),
                Arm(instr) => execute_arm(fde.decoded_opcode, instr, cpu, mem),
            };

            if DEBUG {
                debug_screen(&cpu, instruction, fde.decoded_opcode, &old_regs, f);
            }

            if cpu.clear_pipeline {
                fde.fetched = None;
                fde.decoded = None;
                cpu.clear_pipeline = false;
                continue;
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
    }
}

fn debug_screen(
    cpu: &Cpu, 
    instr: DecodedInstruction, 
    opcode: u32, 
    old_cpu: &Cpu, 
    f: &mut File,
) {
    writeln!(f, "======== DEBUG ========").unwrap();
    let mut temp = Vec::new();
    for i in 0..=14 {
        let old_value = old_cpu.get_register(i);
        let new_value = cpu.get_register(i);
        temp.push(new_value);

        if old_value == new_value { continue; }
        write!(f, "r{i} ==> {new_value:X}... ").unwrap();
    }

    writeln!(f, "").unwrap();
    writeln!(f, "{temp:X?}").unwrap();
    writeln!(f, "pc: {:X}, from: {:X}", cpu.pc, old_cpu.pc).unwrap();
    writeln!(f, "status: {:?}", cpu.cpsr).unwrap();
    writeln!(f, "======= {instr:?} {opcode:X} ========= ").unwrap();
    writeln!(f, "").unwrap();
    if !PRINT {
        return;
    }

    if STEP {
        let mut temp = String::new();
        std::io::stdin().read_line(&mut temp).unwrap();
    } else {
        println!("");
    }

    print!("{instr:?} | {opcode:X} | ");

    for i in 0..=15 {
        let old_value = old_cpu.get_register(i);
        let new_value = cpu.get_register(i);

        if old_value == new_value { continue; }
        print!("r{i} ==> {old_value:X} = {new_value:X} ");
    }
    print!(" | ");
    if old_cpu.cpsr.c != cpu.cpsr.c {
        let clear = cpu.cpsr.c;
        print!("c = {clear} ");
    }
    if old_cpu.cpsr.z != cpu.cpsr.z {
        let zero = cpu.cpsr.z;
        print!("z = {zero} ");
    }
    if old_cpu.cpsr.n != cpu.cpsr.n {
        let negative = cpu.cpsr.n;
        print!("n = {negative} ");
    }
    if old_cpu.cpsr.v != cpu.cpsr.v {
        let overflow = cpu.cpsr.v;
        print!("v = {overflow} ");
    }
    stdout().flush().unwrap();
}

fn main() {
    // set up the window
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32))
        .with_resizable(false)
        .build(&event_loop)
    .unwrap();
    
    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32, surface_texture).unwrap()
    };

    // debug purposes
    let mut debug_file = File::create("debug/debug.txt").expect("the file couldnt be opened");

    let mut cpu;
    if BIOS {
        cpu = Cpu::from_bios();
    } else {
        cpu = Cpu::new();
    }
    let mut mem = memory::create_memory("test/bin/first.gba");
    let mut ppu = Ppu::new();
    let mut fde = Fde::default();
    setup_joypad(&mut mem);

    let mut last_render = Instant::now();
    let mut cycles = 0;

    event_loop.run(|event, control_flow|
        match event {
            Event::WindowEvent {ref event, window_id} if window_id == window.id() => {
                match event {
                    WindowEvent::KeyboardInput { event, .. } => {
                        if let PhysicalKey::Code(code) = event.physical_key {
                            match event.state.is_pressed() {
                                true => joypad::joypad_press(code, &mut mem),
                                false => joypad::joypad_release(code, &mut mem),
                            }
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        gba_frame(
                            &mut cpu, 
                            &mut mem, 
                            &mut ppu, 
                            &mut fde, 
                            &mut cycles, 
                            &mut debug_file
                        );

                        // keep it running at 60fps
                        if last_render.elapsed().as_nanos() <= FRAME_TIME {
                            last_render = std::time::Instant::now();
                            // the amount it should wait for 60fps
                            let difference = FRAME_TIME - last_render.elapsed().as_nanos();
                            thread::sleep(Duration::from_nanos(difference as u64));
                        }

                        let screen = pixels.frame_mut();
                        for (i, c) in ppu.stored_screen.iter().enumerate() {
                            let r = (*c >> 16) & 0xFF;
                            let g = (*c >> 8) & 0xFF;
                            let b = *c & 0xFF;

                            screen[(i * 4) + 0] = r as u8;
                            screen[(i * 4) + 1] = g as u8;
                            screen[(i * 4) + 2] = b as u8;
                            screen[(i * 4) + 3] = 0xFF;
                        }
                        pixels.render().unwrap();
                        window.request_redraw();
                    }
                    WindowEvent::CloseRequested => control_flow.exit(),
                    _ => {}
                }
            }
            _ => {}
        }
    ).unwrap();
}
