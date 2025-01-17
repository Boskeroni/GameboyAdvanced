use registers::{status_registers::CpuStatus, Cpu};

pub mod execute_arm;
pub mod execute_thumb;
pub mod registers;
pub mod decode;
pub mod interrupt;

/// several different instructions make use of this behaviour
/// I'm not sure if they all function the same but I have no reason to believe otherwise
/// both the shifted value and the carry flag are returned
/// 
/// opcode should be the 11 bits which represent the shift + register
pub fn get_shifted_value(cpu_regs: &Cpu, opcode: u32, status: &CpuStatus) -> (u32, bool) {
    let shift_id = (opcode >> 4) & 1 == 1;
    let shift_type = (opcode >> 5) & 0b11;

    let shift_amount;
    match shift_id {
        true => {
            let rs_index = (opcode >> 8) as u8 & 0xF;
            shift_amount = cpu_regs.get_register(rs_index, status.cpsr.mode) & 0xFF;
        }
        false => {
            shift_amount = (opcode >> 7) & 0x1F;
        }
    }

    let rm_index = opcode as u8 & 0xF;
    let rm = cpu_regs.get_register(rm_index, status.cpsr.mode);

    match shift_type {
        0b00 => {
            match shift_amount {
                32 => return (0, rm & 1 == 1),
                32.. => return (0, false),
                0 => return (rm, status.cpsr.c),
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
                let result = (rm >> 31) | (status.cpsr.c as u32) << 31;
                return (result, rm & 1 == 1);
            }
            let result = rm.rotate_right(shift_amount);
            return (result, (result >> 31) & 1 == 1);
        }
        _ => unreachable!(),
    }
}
