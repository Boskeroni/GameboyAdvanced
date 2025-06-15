use crate::memory::Memory;
use crate::memory::Memoriable;

#[derive(Debug, Clone, Copy)]
pub enum Button {
    A,
    B,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
    L,
    R,
    Other,
}

enum JPRegisters {
    KeyInput = 0x4000130,
    KeyCnt = 0x4000132,
}

pub fn init_joypad(mem: &mut Memory) {
    mem.write_io(JPRegisters::KeyInput as u32, 0xFFFF);
}

pub fn joypad_press(input: Button, mem: &mut Box<Memory>) {
    let mut joypad = mem.read_u16(JPRegisters::KeyInput as u32);

    use Button::*;
    match input {
        A => joypad &= !(1 << 0), // BUTTON A
        B => joypad &= !(1 << 1), // BUTTON B
        Select => joypad &= !(1 << 2), // SELECT
        Start => joypad &= !(1 << 3), // START
        Right => joypad &= !(1 << 4), // RIGHT
        Left => joypad &= !(1 << 5), // LEFT
        Up => joypad &= !(1 << 6), // UP
        Down => joypad &= !(1 << 7), // DOWN
        R => joypad &= !(1 << 8), // BUTTON R
        L => joypad &= !(1 << 9), // BUTTON L
        Other => return,
    }
    mem.write_io(JPRegisters::KeyInput as u32, joypad);

    joypad_interrupt(mem, joypad);
}

pub fn joypad_release(input: Button, mem: &mut Box<Memory>) {
    let mut joypad = mem.read_u16(JPRegisters::KeyInput as u32);

    use Button::*;
    match input {
        A => joypad |= 1 << 0,
        B => joypad |= 1 << 1,
        Select => joypad |= 1 << 2,
        Start => joypad |= 1 << 3,
        Right => joypad |= 1 << 4,
        Left => joypad |= 1 << 5,
        Up => joypad |= 1 << 6,
        Down => joypad |= 1 << 7,
        R => joypad |= 1 << 8,
        L => joypad |= 1 << 9,
        Other => return,
    }
    mem.write_io(JPRegisters::KeyInput as u32, joypad);
    joypad_interrupt(mem, joypad);
}

fn joypad_interrupt(mem: &mut Box<Memory>, joypad: u16) {
    let control = mem.read_u16(JPRegisters::KeyCnt as u32);
    if (control >> 14) & 1 == 0 {
        return;
    }
    
    let mask = control & 0x3FF;
    let keys = joypad & mask;

    let interrupt_condition = (control >> 15) & 1 == 1;
    let call_interrupt = match interrupt_condition {
        true => mask == keys,
        false => keys != 0,
    };

    let mut i_flag = mem.read_u16(0x4000202);
    i_flag &= !(1 << 12);
    i_flag |= (call_interrupt as u16) << 12;

    mem.write_io(0x4000202, i_flag); 
}
