use gba_core::{joypad::{self, init_joypad, joypad_press, joypad_release}, run_single_step, Emulator};
use std::sync::{mpsc::{Receiver, SyncSender}, Arc};
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
    Run,
    Pause,
    Step,
    End,
}

pub fn run_emulator(
    emulator_arc: Arc<RwLock<Emulator>>,
    redraw_send: SyncSender<Vec<u32>>,
    inp_recv: Receiver<EmulatorSend>,
) {
    let mut emulator = emulator_arc.write();
    init_joypad(&mut emulator.mem);
    drop(emulator);

    // since the lock needs to end, this is done in its own function
    // also looks a bit cleaner
    let mut state = EmulatorState::Pause;
    loop {
        let redraw_needed = update_emulator(&emulator_arc, &mut state);
        if redraw_needed {
            let emulator = emulator_arc.read();
            redraw_send.send(emulator.ppu.stored_screen.clone()).unwrap();
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

fn update_emulator(emulator_arc: &Arc<RwLock<Emulator>>, state: &mut EmulatorState) -> bool {
    let mut emulator = emulator_arc.write();
    use EmulatorState::*;
    let redraw_needed = match state {
        Run => {
            emulator.ppu.acknowledge_frame();
            loop {
                let finished = run_single_step(&mut emulator);
                if finished {
                    return true;
                }
            }; 
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