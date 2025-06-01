#[cfg(feature = "debug")]
mod debug;

mod emulator;
use core::joypad::init_joypad;
use core::Emulator;

use std::sync::{mpsc, Arc, Mutex};
use std::env;
use std::thread;

use emulator::{EmulatorApp, EmulatorCommand};

fn main() {
    let file = env::args().nth(1).unwrap();
    let rom_path = format!("roms/{file}");
    let emulator = Arc::new(Mutex::new(Emulator::new(&rom_path)));

    // debug windows
    let mut dbg_ctx_send = None;
    let mut dbg_cmd_recv = None;

    if cfg!(feature = "debug") {
        let debug_emulator = emulator.clone();
        let (ctx_send, ctx_recv) = mpsc::channel::<egui::Context>();
        let (cmd_send, cmd_recv) = mpsc::channel::<EmulatorCommand>();
        dbg_ctx_send = Some(ctx_send);
        dbg_cmd_recv = Some(cmd_recv);
        thread::spawn(|| {
            debug::setup_debug(debug_emulator, ctx_recv, cmd_send);
        });
    }
    
    init_joypad(&mut emulator.lock().unwrap().mem);
    let options = eframe::NativeOptions::default();
    let emulator_app = EmulatorApp::new(emulator, dbg_ctx_send, dbg_cmd_recv);
    eframe::run_native(
        "Emulator", 
        options, 
        Box::new(|_| Ok(Box::new(emulator_app)))
    ).unwrap();
}

