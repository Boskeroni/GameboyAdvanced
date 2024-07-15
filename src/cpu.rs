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

#[derive(Debug, Clone)]
pub struct CpuRegisters {
    pub unbanked_registers: [u32; 8],
    // 1D array representing [[r8, r8_fiq], [r9, r9_fiq], ...]
    pub double_banked_registers: [u32; 10], 
    // same logic as the previous one, just with more
    pub many_banked_registers: [u32; 12], 
    pub pc: u32,
}
impl CpuRegisters {
    /// the mode is technically not always needed but will always be needed to be passed
    /// in, this helps in generalising the function when opcodes run this.
    pub fn get_register(&self, register: u8, mode: ProcessorMode) -> u32 {
        match register {
            0..=7 => self.unbanked_registers[register as usize],
            8..=12 => {
                let index = (register - 8) as usize * 2;
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
                    User|System => 0
                };
                self.many_banked_registers[index + offset]
            }
            15 => self.pc,
            _ => unreachable!()
        }
    }
    pub fn get_register_mut(&mut self, register: u8, mode: ProcessorMode) -> &mut u32 {
        match register {
            0..=7 => &mut self.unbanked_registers[register as usize],
            8..=12 => {
                let index = (register - 8) as usize * 2;
                if let ProcessorMode::FastInterrupt = mode {
                    return &mut self.double_banked_registers[index + 1];
                }
                return &mut self.double_banked_registers[index]
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
                    User|System => 0,
                };
                return &mut self.many_banked_registers[index + offset]
            }
            15 => &mut self.pc,
            _ => unreachable!()
        }
    }

    pub fn get_pc(&mut self) -> u32 {
        self.pc += 1;
        self.pc - 1
    }
}

pub mod status_registers {
    use crate::ProcessorMode;

    pub struct StatusRegisters {
        pub cpsr: Cpsr,
        spsr: [Cpsr; 5] // [spsr_fiq, spsr_svc, spsr_abt, spsr_irq, spsr_und]
    }
    impl StatusRegisters {
        pub fn new() -> Self {
            Self {
                cpsr: Cpsr::default(),
                spsr: [Cpsr::default(), Cpsr::default(), Cpsr::default(), Cpsr::default(), Cpsr::default()],
            }
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
                _ => &self.cpsr,
            }   
        }
        pub fn set_spsr(&mut self, new_spsr: Cpsr) {
            use ProcessorMode::*;
            match self.cpsr.mode {
                FastInterrupt => self.spsr[0] = new_spsr,
                Supervisor => self.spsr[1] = new_spsr,
                Abort => self.spsr[2] = new_spsr,
                Interrupt => self.spsr[3] = new_spsr,
                Undefined => self.spsr[4] = new_spsr,
                _ => self.cpsr = new_spsr
            };
        }
        pub fn set_flags_spsr(&mut self, new_spsr: Cpsr) {
            use ProcessorMode::*;
            let spsr = match self.cpsr.mode {
                FastInterrupt => &mut self.spsr[0],
                Supervisor => &mut self.spsr[1],
                Abort => &mut self.spsr[2],
                Interrupt => &mut self.spsr[3],
                Undefined => &mut self.spsr[4],
                _ => &mut self.cpsr,
            };
            spsr.n = new_spsr.n;
            spsr.c = new_spsr.c;
            spsr.v = new_spsr.v;
            spsr.z = new_spsr.z;
        }
        pub fn set_flags_cpsr(&mut self, new_cpsr: Cpsr) {
            self.cpsr.n = new_cpsr.n;
            self.cpsr.c = new_cpsr.c;
            self.cpsr.z = new_cpsr.z;
            self.cpsr.v = new_cpsr.v;
        }

        pub fn set_specific_spsr(&mut self, new_cpsr: Cpsr, mode: ProcessorMode) {
            use ProcessorMode::*;
            let spsr = match mode {
                FastInterrupt => &mut self.spsr[0],
                Supervisor => &mut self.spsr[1],
                Abort => &mut self.spsr[2],
                Interrupt => &mut self.spsr[3],
                Undefined => &mut self.spsr[4],
                _ => panic!("why are you trying to access the CPSR this way?"),
            };
            *spsr = new_cpsr;
        }
    }

    #[derive(Debug, Clone, Default, Copy)]
    pub struct Cpsr {
        pub z: bool, // true if the value is 0
        pub c: bool, // true if the was a carry
        pub n: bool, // true if the value is signed
        pub v: bool, // true if overflow
        pub q: bool, // sticky overflow (not too sure yet of its purpose, ill see when it is used)
        pub i: bool, // IRQ disable
        pub f: bool, // FIQ disable
        pub t: bool, // the state of the instruction set (0 = arm, 1 = thumb)
        pub mode: ProcessorMode, // processor mode (represented by the 5-bits shown in enum)
    }
    pub fn check_arm_condition(condition: u8, cpsr: &Cpsr) -> bool {
        match condition {
            0b0000 => cpsr.z,
            0b0001 => !cpsr.z,
            0b0010 => cpsr.z,
            0b0011 => !cpsr.c,
            0b0100 => cpsr.n,
            0b0101 => !cpsr.n,
            0b0110 => cpsr.v,
            0b0111 => !cpsr.v,
            0b1000 => cpsr.c && !cpsr.z,
            0b1001 => !cpsr.c || cpsr.z,
            0b1010 => cpsr.n == cpsr.v,
            0b1011 => cpsr.n != cpsr.v,
            0b1100 => !cpsr.z && cpsr.n == cpsr.v,
            0b1101 => cpsr.z || cpsr.n != cpsr.v,
            0b1110 => true,
            0b1111 => false,
            _ => unreachable!("condition is only 4 bits long")
        }
    }
    pub fn convert_cpsr_u32(cpsr: &Cpsr) -> u32 {
        (cpsr.n as u32) << 31 |
        (cpsr.z as u32) << 30 |
        (cpsr.c as u32) << 29 |
        (cpsr.v as u32) << 28 |
        (cpsr.q as u32) << 27 |
        (cpsr.i as u32) << 7  |
        (cpsr.f as u32) << 6  |
        (cpsr.t as u32) << 5  |
        (cpsr.mode as u32)
    }
    pub fn convert_u32_cpsr(cpsr: u32) -> Cpsr {
        Cpsr {
            n: (cpsr >> 31) & 1 != 0,
            z: (cpsr >> 30) & 1 != 0,
            c: (cpsr >> 29) & 1 != 0,
            v: (cpsr >> 28) & 1 != 0,
            q: (cpsr >> 27) & 1 != 0,
            i: (cpsr >> 7 ) & 1 != 0,
            f: (cpsr >> 6 ) & 1 != 0,
            t: (cpsr >> 5 ) & 1 != 0,
            mode: match cpsr & 0b11111 {
                0b10000 => ProcessorMode::User,
                0b10001 => ProcessorMode::FastInterrupt,
                0b10010 => ProcessorMode::Interrupt,
                0b10011 => ProcessorMode::Supervisor,
                0b10111 => ProcessorMode::Abort,
                0b11011 => ProcessorMode::Undefined,
                0b11111 => ProcessorMode::System,
                _ => unreachable!(),
            } 
        }
    }
}
