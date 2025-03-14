pub mod execute_arm;
pub mod execute_thumb;
pub mod decode;

/// several different instructions make use of this behaviour
/// I'm not sure if they all function the same but I have no reason to believe otherwise
/// both the shifted value and the carry flag are returned
/// 
/// opcode should be the 11 bits which represent the shift + register
/// 
pub fn get_shifted_value(cpu: &mut Cpu, opcode: u32) -> (u32, bool) {
    let (result, carry) = _get_shifted_value(cpu, opcode);

    cpu.barrel_shifter = carry;
    return (result, carry);
}

fn _get_shifted_value(cpu: &Cpu, opcode: u32) -> (u32, bool) {
    let shift_id = (opcode >> 4) & 1 == 1;
    let shift_type = (opcode >> 5) & 0b11;

    let shift_amount;
    match shift_id {
        true => {
            let rs_index = (opcode >> 8) as u8 & 0xF;
            shift_amount = cpu.get_register(rs_index) & 0xFF;
        }
        false => {
            shift_amount = (opcode >> 7) & 0x1F;
        }
    }

    let rm_index = opcode as u8 & 0xF;

    let rm;
    if shift_id && rm_index == 15 {
        rm = cpu.get_register(rm_index) + 4;
    } else {
        rm = cpu.get_register(rm_index);
    }

    // if 0 is from a register, then unchanged
    if shift_id && shift_amount == 0 {
        return (rm, cpu.cpsr.c);
    }

    match shift_type {
        0b00 => {
            match shift_amount {
                32 => return (0, rm & 1 == 1),
                32.. => return (0, false),
                0 => return (rm, cpu.cpsr.c),
                _ => {}
            }

            let carry = (rm << (shift_amount - 1)) >> 31 & 1 == 1;
            return (rm << shift_amount, carry)
        }
        0b01 => {
            match shift_amount {
                0 => return (0, (rm >> 31) & 1 == 1),
                32 => return (0, (rm >> 31) & 1 == 1),
                32.. => return (0, false),
                _ => {}
            }

            if shift_amount == 0 {
                return (0, (rm >> 31) & 1 == 1);
            }
            return (rm >> shift_amount, rm >> (shift_amount - 1) & 1 == 1);
        }
        0b10 => {
            if shift_amount == 0 || shift_amount >= 32 {
                let result = if (rm >> 31) & 1 == 1 {std::u32::MAX} else {0};
                return (result, result != 0);
            }

            let mut temp = rm >> shift_amount;
            if (rm >> 31) & 1 == 1 {
                temp |= !(std::u32::MAX >> shift_amount);
            }
            return (temp, rm >> (shift_amount - 1) & 1 == 1);
        }
        0b11 => {
            if shift_amount == 0 {
                let result = (rm >> 1) | (cpu.cpsr.c as u32) << 31;
                return (result, rm & 1 == 1);
            }
            let result = rm.rotate_right(shift_amount);
            return (result, (result >> 31) & 1 == 1);
        }
        _ => unreachable!(),
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub enum ProcessorMode {
    #[default]
    User = 0b10000,
    FastInterrupt = 0b10001,
    Interrupt = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
    Undefined = 0b11011,
    System = 0b11111,
}

#[derive(Debug, Clone, Default, Copy)]
pub struct Cpsr {
    pub z: bool, // true if the value is 0
    pub c: bool, // true if the was a carry
    pub n: bool, // true if the value is signed
    pub v: bool, // true if overflow
    pub i: bool, // IRQ disable
    pub f: bool, // FIQ disable
    pub t: bool, // the state of the instruction set (0 = arm, 1 = thumb)
    pub mode: ProcessorMode, // processor mode (represented by the 5-bits shown in enum)
}
impl Cpsr {
    pub fn set_flags(&mut self, bits: u32) {
        self.n = (bits >> 31) & 1 == 1;
        self.z = (bits >> 30) & 1 == 1;
        self.c = (bits >> 29) & 1 == 1;
        self.v = (bits >> 28) & 1 == 1;
    }
    pub fn set_control(&mut self, bits: u32) {
        self.i = (bits >> 7) & 1 == 1;
        self.f = (bits >> 6) & 1 == 1;
        self.t = (bits >> 5) & 1 == 1;
        self.mode = match bits & 0b11111 {
            0b10000 => ProcessorMode::User,
            0b10001 => ProcessorMode::FastInterrupt,
            0b10010 => ProcessorMode::Interrupt,
            0b10011 => ProcessorMode::Supervisor,
            0b10111 => ProcessorMode::Abort,
            0b11011 => ProcessorMode::Undefined,
            0b11111 => ProcessorMode::System,
            _ => ProcessorMode::Undefined,
        }
    }
}
pub fn check_condition(condition: u32, cpsr: &Cpsr) -> bool {
    match condition {
        0b0000 => cpsr.z,
        0b0001 => !cpsr.z,
        0b0010 => cpsr.c,
        0b0011 => !cpsr.c,
        0b0100 => cpsr.n,
        0b0101 => !cpsr.n,
        0b0110 => cpsr.v,
        0b0111 => !cpsr.v,
        0b1000 => cpsr.c && !cpsr.z,
        0b1001 => !cpsr.c || cpsr.z,
        0b1010 => cpsr.n == cpsr.v,
        0b1011 => cpsr.n != cpsr.v,
        0b1100 => !cpsr.z && (cpsr.n == cpsr.v),
        0b1101 => cpsr.z || (cpsr.n != cpsr.v),
        0b1110 => true,
        0b1111 => false,
        _ => unreachable!("condition is only 4 bits long")
    }
}
pub fn convert_psr_u32(cpsr: &Cpsr) -> u32 {
    (cpsr.n as u32) << 31 |
    (cpsr.z as u32) << 30 |
    (cpsr.c as u32) << 29 |
    (cpsr.v as u32) << 28 |
    (cpsr.i as u32) << 7  |
    (cpsr.f as u32) << 6  |
    (cpsr.t as u32) << 5  |
    (cpsr.mode as u32)
}

#[derive(Debug, Clone)]
pub struct Cpu {
    pub unbanked_registers: [u32; 8],
    // 1D array representing [[r8, r8_fiq], [r9, r9_fiq], ..., [r12, r12_fiq]]
    pub double_banked_registers: [[u32; 2]; 5],
    // same logic as the previous one, just with more [[r13, f13_fiq, r13_svc, r13_abt, r13_irq, r13_und], ...]
    pub many_banked_registers: [[u32; 6]; 2], 
    pub pc: u32,
    pub clear_pipeline: bool,

    pub cpsr: Cpsr,
    pub spsr: [Cpsr; 6],

    barrel_shifter: bool,
}
impl Cpu {
    pub fn new() -> Self {
        Self {
            unbanked_registers: [0, 0, 0, 0, 0, 0, 0, 0],
            double_banked_registers: [[0, 0], [0, 0], [0, 0], [0, 0], [0, 0]],
            many_banked_registers: [[0x03007F00, 0, 0x03007FE0, 0, 0x03007FA0, 0], [0, 0, 0, 0, 0, 0]],
            pc: 0x8000000,
            cpsr: Cpsr::default(),
            spsr: [Cpsr::default(), Cpsr::default(), Cpsr::default(), Cpsr::default(), Cpsr::default(), Cpsr::default()],
            barrel_shifter: false,

            clear_pipeline: false,
        }
    }
    pub fn from_bios() -> Self {
        Self {
            unbanked_registers: [0, 0, 0, 0, 0, 0, 0, 0],
            double_banked_registers: [[0, 0], [0, 0], [0, 0], [0, 0], [0, 0]],
            many_banked_registers: [[0, 0, 0, 0, 0, 0], [0, 0, 0, 0, 0, 0]],
            pc: 0,
            cpsr: Cpsr::default(),
            spsr: [Cpsr::default(), Cpsr::default(), Cpsr::default(), Cpsr::default(), Cpsr::default(), Cpsr::default()],
            barrel_shifter: false,

            clear_pipeline: false,
        }
    }

    /// the mode is technically not always needed but will always be needed to be passed
    /// in, this helps in generalising the function when opcodes run this.
    pub fn get_register(&self, register: u8) -> u32 {
        let register = register as usize;
        match register {
            0..=7 => self.unbanked_registers[register],
            8..=12 => {
                let index = register - 8;
                if let ProcessorMode::FastInterrupt = self.cpsr.mode {
                    return self.double_banked_registers[index][1];
                }
                return self.double_banked_registers[index][0];
            }
            13..=14 => {
                use ProcessorMode::*;
                
                // [[r13, f13_fiq, r13_svc, r13_abt, r13_irq, r13_und], ...]
                let index = register - 13;
                let offset = match self.cpsr.mode {
                    User|System => 0,
                    FastInterrupt => 1,
                    Supervisor => 2,
                    Abort => 3,
                    Interrupt => 4,
                    Undefined => 5,
                    
                };
                self.many_banked_registers[index][offset]
            }
            15 => self.pc,
            _ => unreachable!()
        }
    }
    pub fn get_register_mut(&mut self, register: u8) -> &mut u32 {
        let register = register as usize;

        match register {
            0..=7 => &mut self.unbanked_registers[register],
            8..=12 => {
                let index = register - 8;
                if let ProcessorMode::FastInterrupt = self.cpsr.mode {
                    return &mut self.double_banked_registers[index][1]
                }
                return &mut self.double_banked_registers[index][0]
            }
            13..=14 => {
                let index = register - 13;
                use ProcessorMode::*;
                let offset = match self.cpsr.mode {
                    User|System => 0,
                    FastInterrupt => 1,
                    Supervisor => 2,
                    Abort => 3,
                    Interrupt => 4,
                    Undefined => 5,
                };
                return &mut self.many_banked_registers[index][offset]
            }
            15 => &mut self.pc,
            _ => unreachable!()
        }
    }
    pub fn get_register_specific(&mut self, register: u8, mode: ProcessorMode) -> u32 {
        let backup = self.cpsr.mode;
        self.cpsr.mode = mode;

        let register = self.get_register(register);
        self.cpsr.mode = backup;

        return register;
    }
    pub fn get_register_mut_specific(&mut self, register: u8, given_mode: ProcessorMode) -> &mut u32 {
        let register = register as usize;

        match register {
            0..=7 => &mut self.unbanked_registers[register],
            8..=12 => {
                let index = register - 8;
                if let ProcessorMode::FastInterrupt = given_mode {
                    return &mut self.double_banked_registers[index][1]
                }
                return &mut self.double_banked_registers[index][0]
            }
            13..=14 => {
                let index = register - 13;
                use ProcessorMode::*;
                let offset = match given_mode {
                    User|System => 0,
                    FastInterrupt => 1,
                    Supervisor => 2,
                    Abort => 3,
                    Interrupt => 4,
                    Undefined => 5,
                };
                return &mut self.many_banked_registers[index][offset]
            }
            15 => &mut self.pc,
            _ => unreachable!()
        }
    }

    pub fn get_pc_arm(&mut self) -> u32 {
        self.pc += 4;
        self.pc - 4
    }
    pub fn get_pc_thumb(&mut self) -> u32 {
        self.pc += 2;
        self.pc - 2
    }

    // CPSR stuff
    pub fn set_specific_spsr(&mut self, new_cpsr: Cpsr, mode: ProcessorMode) {
        use ProcessorMode::*;
        let spsr = match mode {
            FastInterrupt => &mut self.spsr[0],
            Supervisor => &mut self.spsr[1],
            Abort => &mut self.spsr[2],
            Interrupt => &mut self.spsr[3],
            Undefined => &mut self.spsr[4],
            _ => panic!("CPSR doesnt have an associated SPSR"),
        };
        *spsr = new_cpsr;
    }
    // if there is no spsr, it returns the global cpsr
    pub fn get_spsr(&self) -> &Cpsr {
        use ProcessorMode::*;
        match self.cpsr.mode {
            FastInterrupt => &self.spsr[0],
            Supervisor => &self.spsr[1],
            Abort => &self.spsr[2],
            Interrupt => &self.spsr[3],
            Undefined => &self.spsr[4],
            System => &self.cpsr, // edge case got from a discord person
            _ => panic!("this shouldnt be accessed"),
        }   
    }
    pub fn get_spsr_mut(&mut self) -> &mut Cpsr {
        use ProcessorMode::*;
        match self.cpsr.mode {
            FastInterrupt => &mut self.spsr[0],
            Supervisor => &mut self.spsr[1],
            Abort => &mut self.spsr[2],
            Interrupt => &mut self.spsr[3],
            Undefined => &mut self.spsr[4],
            _ => &mut self.cpsr,
        }
    }

    pub fn get_barrel_shift(&self) -> bool {
        self.barrel_shifter
    }
}


use crate::memory::Memory;
enum CpuRegisters {
    Ie = 0x4000200,
    If = 0x4000202,
    Ime = 0x4000208,
}

/// the ahead_by variable represents how many instructions the pc is
/// it is multiplied by 4 for ARM, and 2 for Thumb.
/// Just intended for callbacks
pub fn handle_interrupts(memory: &mut Memory, cpu: &mut Cpu, ahead_by: u32) {
    if cpu.cpsr.i {
        return;
    }

    let interrupt_allowed = memory.read_u32(CpuRegisters::Ime as u32) & 1 == 1;
    if !interrupt_allowed {
        return;
    }
    let interrupts_enabled = memory.read_u16(CpuRegisters::Ie as u32);
    let interrupts_called = memory.read_u16(CpuRegisters::If as u32);

    let called_interrupts = interrupts_enabled & interrupts_called;
    if called_interrupts == 0 {
        return;
    }

    let is_in_thumb = cpu.cpsr.t;
    let pc = cpu.get_register(15);
    let lr = cpu.get_register_mut_specific(14, ProcessorMode::Interrupt);
    match is_in_thumb {
        true => *lr = pc - (ahead_by * 2) + 4,
        false => *lr = pc - (ahead_by * 4) + 4,
    }
    cpu.set_specific_spsr(cpu.cpsr, ProcessorMode::Interrupt);

    cpu.cpsr.mode = ProcessorMode::Interrupt;
    cpu.cpsr.t = false;
    cpu.cpsr.i = true;

    let pc = cpu.get_register_mut(15);
    *pc = 0x18;
    cpu.clear_pipeline = true;
}