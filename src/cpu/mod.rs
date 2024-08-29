use registers::{status_registers::Status, Cpu};

pub mod execute_arm;
pub mod execute_thumb;
pub mod registers;
pub mod decode;

/// several different instructions make use of this behaviour
/// I'm not sure if they all function the same but I have no reason to believe otherwise
/// both the shifted value and the carry flag are returned
/// 
/// opcode should be the 11 bits which represent the shift + register
pub fn get_shifted_value(cpu_regs: &Cpu, opcode: u32, status: &Status) -> (u32, bool) {
    let rm_index = opcode & 0xF;
    let rm = cpu_regs.get_register(rm_index as u8, status.cpsr.mode);

    let mut shift_amount = match (opcode >> 4) & 1 != 0 {
        false => (opcode >> 7) & 0x1F, // the simple case :)
        true => {
            let rs = (opcode >> 8) as u8 & 0xF;
            assert!(rs != 15, "Rs cannot equal 15 in this case");
            cpu_regs.get_register(rs, status.cpsr.mode) & 0xFF
        },
    };
    let shift_type = (opcode >> 5) & 0b11;

    match shift_type {
        0b00 => {
            // the carry bit stays the same if the shift instruction is LSL #0
            // the shift amount can be greater than 32 if its from a register
            if shift_amount > 32 {
                return (0, false)
            }
            let res = rm << shift_amount;
            if shift_amount == 0 {
                return (res, status.cpsr.c)
            }
            // this line may look wrong but the math does check out
            return (res, (rm >> (32 - shift_amount)) & 1 != 0)
        } // Logical left shift
        0b01 => {
            if shift_amount > 32 {
                return (0, false)
            }
            // LSR #0 automatically becomes LSL #0 so it doesnt need an edge case
            if shift_amount == 0 {
                shift_amount = 32;
            }
            return (rm >> shift_amount, (rm >> (shift_amount - 1) & 1) != 0)
        } // logical right shift
        0b10 => {
            let padding = (rm & 0x80000000) as i32;
            if shift_amount > 32 {
                return ((padding >> 31) as u32, padding != 0)
            }
            if shift_amount == 0 {
                shift_amount = 32;
            }
            return ((rm >> shift_amount) | (padding >> shift_amount) as u32, rm >> (shift_amount - 1) & 1 != 0)
        } // Arithmetic shift left
        0b11 => {
            // when it is ROR #0 it means RRX
            if shift_amount == 0 {
                return ((rm >> 1) | (status.cpsr.c as u32) << 31, rm & 1 != 0)
            }
            let res = rm.rotate_right(shift_amount);
            return (res, (res >> 31) & 1 != 0)
        },
        _ => unreachable!(),
    };
}