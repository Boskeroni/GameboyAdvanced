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
    let data_method = (opcode >> 4) & 1 == 1;
    let shift_amount;
    match data_method {
        true => {
            let rs_index = (opcode >> 8) & 0xF;
            let rs = cpu_regs.get_register(rs_index as u8, status.cpsr.mode);
            shift_amount = rs & 0xFF;
        }
        false => shift_amount = (opcode >> 7) & 0x1F,
    }

    let rd_index = opcode & 0xF;
    let rd = cpu_regs.get_register(rd_index as u8, status.cpsr.mode);

    let (result, carry);
    let shift_type = (opcode >> 5) & 0b11;
    match shift_type {
        0b00 => {
            // this is a special case
            if shift_amount == 0 {
                return (rd, status.cpsr.c);
            }

            result = rd << shift_amount;
            let carry_interim = rd << (shift_amount - 1);
            carry = (carry_interim >> 31) & 1 == 1;

            return (result, carry);
        }
        0b01 => {
            if shift_amount == 0 {
                return (0, (rd >> 31) & 1 == 1)
            }

            result = rd >> shift_amount;
            carry = (rd >> (shift_amount - 1)) & 1 == 1;
            return (result, carry)
        }
        0b10 => {
            let fill: i32 = (rd as i32) & !0x7FFFFFFF;
            if shift_amount == 0 {
                return ((fill >> 31) as u32, fill != 0)
            }

            let interim_result = rd >> shift_amount;
            result = (fill >> shift_amount) as u32 | interim_result;
            carry = (rd >> (shift_amount - 1)) & 1 == 1;
            return (result, carry)
        }
        0b11 => {
            if shift_amount == 0 {
                carry = rd & 1 == 1;
                result = (rd >> 1) | ((status.cpsr.c as u32) << 31);

                return (result, carry)
            }

            result = rd.rotate_right(shift_amount);
            carry = (result >> 31) & 1 == 1;

            return (result, carry)
        }
        _ => unreachable!()
    }
}
