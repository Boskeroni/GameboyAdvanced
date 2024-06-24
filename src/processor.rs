#![allow(unused)]

#[derive(Debug, Copy, Clone)]
enum ProcessorMode {
    User = 0b10000,
    FastInterrupt = 0b10001,
    Interrupt = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
    Undefined = 0b11011,
    System = 0b11111,
}
pub struct CpuRegisters {
    pub unbanked_registers: [u32; 8],
    // 1D array representing [[r8, r8_fiq], [r9, r9_fiq], ...]
    pub double_banked_registers: [u32; 10], 
    // same logic as the previous one, just with more
    pub many_banked_registers: [u32; 12], 
    pub pc: u32,
}
impl CpuRegisters {
    pub fn get_register(&self, register: u8, mode: ProcessorMode) -> u32 {
        match register {
            0..=7 => self.unbanked_registers[register as usize],
            8..=12 => {
                let index = (register - 8) as usize >> 1;
                if let ProcessorMode::FastInterrupt = mode {
                    return self.double_banked_registers[index + 1]
                }
                self.double_banked_registers[index]
            }
            13..=14 => {
                let index = (register - 13) as usize * 6;
                use ProcessorMode::*;
                let offset = match mode {
                    FastInterrupt => 5,
                    Interrupt => 4,
                    Undefined => 3,
                    Abort => 2,
                    Supervisor => 1,
                    _ => 0
                };
                self.many_banked_registers[index + offset]
            }
            15 => self.pc,
            _ => unreachable!()
        }
    }
}
pub struct CPSR {
    pub z: bool,
    pub c: bool,
    pub n: bool,
    pub v: bool,
}


#[derive(Clone, Copy)]
#[repr(u8)]
enum OpcodeCondition {
    EQ = 0b0000,
    NE = 0b0001,
    HS = 0b0010,
    LO = 0b0011,
    MI = 0b0100,
    PL = 0b0101,
    VS = 0b0110,
    VC = 0b0111,
    HI = 0b1000,
    LS = 0b1001,
    GE = 0b1010,
    LT = 0b1011,
    GT = 0b1100,
    LE = 0b1101,
    AL = 0b1110,
    NV = 0b1111,
}
impl std::convert::From<u8> for OpcodeCondition {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::EQ,
            1 => Self::NE,
            2 => Self::HS,
            3 => Self::LO,
            4 => Self::MI,
            5 => Self::PL,
            6 => Self::VS,
            7 => Self::VC,
            8 => Self::HI,
            9 => Self::LS,
            10 => Self::GE,
            11 => Self::LT,
            12 => Self::GT,
            13 => Self::LE,
            14 => Self::AL,
            15 => Self::NV,
            _ => panic!("invalid opcode condition"),
        }
    }
}
impl OpcodeCondition {
    pub fn check_condition(&self, cpu: &CPSR) -> bool {
        match self {
            Self::EQ => cpu.z,
            Self::NE => !cpu.z,
            Self::HS => cpu.z,
            Self::LO => !cpu.c,
            Self::MI => cpu.n,
            Self::PL => !cpu.n,
            Self::VS => cpu.v,
            Self::VC => !cpu.v,
            Self::HI => cpu.c && !cpu.z,
            Self::LS => !cpu.c || cpu.z,
            Self::GE => cpu.n == cpu.v,
            Self::LT => cpu.n != cpu.v,
            Self::GT => !cpu.z && cpu.n == cpu.v,
            Self::LE => cpu.z || cpu.n != cpu.v,
            Self::AL => true,
            Self::NV => false,
        }
    }
}


// not yet implemented in any way but it should be known for later
// https://www.dwedit.org/files/ARM7TDMI.pdf

// not yet implemented in any way but it should be known for later
// https://www.dwedit.org/files/ARM7TDMI.pdf

// not yet implemented in any way but it should be known for later
// https://www.dwedit.org/files/ARM7TDMI.pdf

// not yet implemented in any way but it should be known for later
// https://www.dwedit.org/files/ARM7TDMI.pdf

// not yet implemented in any way but it should be known for later
// https://www.dwedit.org/files/ARM7TDMI.pdf