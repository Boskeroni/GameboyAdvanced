use crate::mem::memory::InternalMemory;

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

pub fn init_joypad(mem: &mut InternalMemory) {
    mem.sys_write_u16(JPRegisters::KeyInput as u32, 0xFFFF);
}

pub fn joypad_press(input: Button, mem: &mut Box<InternalMemory>) {
    let mut joypad = mem.sys_read_u16(JPRegisters::KeyInput as u32);

    use Button::*;
    match input {
        A => joypad &= !(1 << 0),
        B => joypad &= !(1 << 1),
        Select => joypad &= !(1 << 2),
        Start => joypad &= !(1 << 3),
        Right => joypad &= !(1 << 4),
        Left => joypad &= !(1 << 5),
        Up => joypad &= !(1 << 6),
        Down => joypad &= !(1 << 7),
        R => joypad &= !(1 << 8),
        L => joypad &= !(1 << 9),
        Other => return,
    }
    mem.sys_write_u16(JPRegisters::KeyInput as u32, joypad);

    joypad_interrupt(mem, joypad);
}

pub fn joypad_release(input: Button, mem: &mut Box<InternalMemory>) {
    let mut joypad = mem.sys_read_u16(JPRegisters::KeyInput as u32);

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
    mem.sys_write_u16(JPRegisters::KeyInput as u32, joypad);
    joypad_interrupt(mem, joypad);
}

fn joypad_interrupt(mem: &mut Box<InternalMemory>, joypad: u16) {
    let control = mem.sys_read_u16(JPRegisters::KeyCnt as u32);
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

    let mut i_flag = mem.sys_read_u16(0x4000202);
    i_flag &= !(1 << 12);
    i_flag |= (call_interrupt as u16) << 12;

    mem.sys_write_u16(0x4000202, i_flag); 
}
