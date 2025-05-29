mod debug;
mod emulator;

use core::cpu::Cpu;
use std::sync::mpsc;
use std::thread;
use debug::{setup_debug, DebugCommand, DebugDataBackend, DebugDataFrontend};
use emulator::Emulator;

fn main() {
    let emulator = Emulator::new("roms/bin/bigmap.gba");

    let (cpu_send, cpu_recv) = mpsc::channel::<Cpu>();
    let (ppu_send, ppu_recv) = mpsc::channel::<Vec<u32>>();
    let (mem_send, mem_recv) = mpsc::channel::<Vec<u8>>();
    let (ins_send, ins_recv) = mpsc::channel::<String>();
    let (cnt_send, cnt_recv) = mpsc::channel::<DebugCommand>();
    let (inp_send, inp_recv) = mpsc::channel::<egui::Event>();

    let dbg_back = DebugDataBackend {
        cpu_dbg: cpu_send,
        ppu_dbg: ppu_send,
        mem_dbg: mem_send,
        ins_dbg: ins_send,
        cnt_dbg: cnt_recv,
    };

    // a second thread keeps all of the emulator's stuff going
    thread::spawn(move || {
        emulator::run_emulator(emulator, dbg_back, inp_recv);
    });

    let dbg_front = DebugDataFrontend {
        cpu_dbg: cpu_recv,
        ppu_dbg: ppu_recv,
        mem_dbg: mem_recv,
        ins_dbg: ins_recv,
        cnt_dbg: cnt_send,
    };
    setup_debug(dbg_front, inp_send);
}

