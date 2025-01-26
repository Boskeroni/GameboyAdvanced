use crate::memory::Memory;

enum JPRegisters {
    KeyInput = 0x4000130,
    KeyCnt = 0x4000132,
}

pub fn setup_joypad(mem: &mut Memory) {
    mem.write_io(JPRegisters::KeyInput as u32, 0xFFFF);
}

pub fn joypad_press(input: winit::keyboard::KeyCode, mem: &mut Memory) {
    let mut joypad = mem.read_u16(JPRegisters::KeyInput as u32);

    use winit::keyboard::KeyCode::*;
    match input {
        KeyK => joypad &= !(1 << 0),
        KeyL => joypad &= !(1 << 1),
        KeyN => joypad &= !(1 << 2),
        KeyM => joypad &= !(1 << 3),
        KeyD => joypad &= !(1 << 4),
        KeyA => joypad &= !(1 << 5),
        KeyW => joypad &= !(1 << 6),
        KeyS => joypad &= !(1 << 7),
        KeyP => joypad &= !(1 << 8),
        KeyQ => joypad &= !(1 << 9),
        _ => {}
    }
    mem.write_io(JPRegisters::KeyInput as u32, joypad);
}

pub fn joypad_release(input: winit::keyboard::KeyCode, mem: &mut Memory) {
    let mut joypad = mem.read_u16(JPRegisters::KeyInput as u32);

    use winit::keyboard::KeyCode::*;
    match input {
        KeyK => joypad |= 1 << 0,
        KeyL => joypad |= 1 << 1,
        KeyN => joypad |= 1 << 2,
        KeyM => joypad |= 1 << 3,
        KeyD => joypad |= 1 << 4,
        KeyA => joypad |= 1 << 5,
        KeyW => joypad |= 1 << 6,
        KeyS => joypad |= 1 << 7,
        KeyP => joypad |= 1 << 8,
        KeyQ => joypad |= 1 << 9,
        _ => {}
    }
    mem.write_io(JPRegisters::KeyInput as u32, joypad);
}