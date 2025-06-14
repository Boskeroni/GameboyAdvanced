use gba_core::{joypad::{self, joypad_press, joypad_release}, run_single_step, Emulator};
use std::{sync::{mpsc::{Receiver, SyncSender}, Arc}, thread, time::{Duration, Instant}};
use egui::Key;
use parking_lot::RwLock;

fn convert_to_joypad(code: Key) -> joypad::Button {
    use joypad::Button;
    use egui::Key;
    match code {
        Key::Z => Button::Select,
        Key::X => Button::Start,
        Key::ArrowLeft => Button::Left,
        Key::ArrowRight => Button::Right,
        Key::ArrowDown => Button::Down,
        Key::ArrowUp => Button::Up,
        Key::K => Button::A,
        Key::L => Button::B,
        Key::Q => Button::L,
        Key::P => Button::R,
        _ => Button::Other,
    }
}

pub enum EmulatorSend {
    StateUpdate(EmulatorState),
    Event(Key, bool),
}
#[derive(Debug, Clone, Copy)]
pub enum EmulatorState {
    Run(u32), // the delay (in milliseconds) each tick should wait
    Pause,
    Step,
    End,
}

pub fn run_emulator(
    emulator_arc: Arc<RwLock<Emulator>>,
    redraw_send: SyncSender<Vec<u32>>,
    inp_recv: Receiver<EmulatorSend>,
) {
    let mut state = EmulatorState::Pause;
    let mut drew_last_time = false;
    loop {
        let redraw_needed = update_emulator(&emulator_arc, &mut state, &mut drew_last_time);
        if redraw_needed {
            let emulator = emulator_arc.read();
            redraw_send.send(emulator.ppu.stored_screen.clone()).unwrap();
            drew_last_time = true;
        }

        if let Ok(i) = inp_recv.try_recv() {
            match i {
                EmulatorSend::Event(key, pressed) => {
                    let mut emulator = emulator_arc.write();
                    let button = convert_to_joypad(key);
                    match pressed {
                        true => joypad_press(button, &mut emulator.mem),
                        false => joypad_release(button, &mut emulator.mem),
                    }
                }
                EmulatorSend::StateUpdate(EmulatorState::End) => return,
                EmulatorSend::StateUpdate(new_state) => state = new_state,
            }
        }
    }
}

fn update_emulator(emulator_arc: &Arc<RwLock<Emulator>>, state: &mut EmulatorState, drew_before: &mut bool) -> bool {
    let mut emulator = emulator_arc.write();

    // done like this cause it makes deadlocks impossible
    // only one write
    if *drew_before {
        emulator.ppu.acknowledge_frame();
        *drew_before = false;
    }

    use EmulatorState::*;
    let redraw_needed = match state {
        Run(delay) => {
            if *delay == 0 {
                let finished = run_single_step(&mut emulator);
                finished
            } else {
                let finished = run_single_step(&mut emulator);
                thread::sleep(Duration::from_nanos(*delay as u64));
                finished
            }
            
        }
        Step => run_single_step(&mut emulator),
        Pause => false,
        End => unreachable!(),
    };

    if let EmulatorState::Step = *state {
        *state = EmulatorState::Pause;
    }

    return redraw_needed;
}