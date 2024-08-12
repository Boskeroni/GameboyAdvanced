use crate::cpu::registers::{*, status_registers::*};
use crate::cpu::decode::DecodedThumb;

// executing the Thumb instructions
// hopefully they are nice to do :)

/// during these instructions, i may not always use `status.cpsr.mode`,
/// this is because the register retrieval doesnt rely upon it as heavily as with ARM.
/// `ProcessorMode::User` is usually just used
pub fn execute_thumb(
    opcode: u16,
    instruction: DecodedThumb,
    cpu_regs: &mut CpuRegisters,
    status: &mut StatusRegisters
) {
    use DecodedThumb::*;

    match instruction {
        MoveShiftedReg => thumb_move_shifted(opcode, cpu_regs, status),
        AddSubtract => thumb_add_sub(opcode, cpu_regs),
        AluImmediate => thumb_alu_imm(opcode, cpu_regs),
        _ => todo!(),
    }
    todo!("still need to implement the status registers changing for this");
}

fn thumb_move_shifted(opcode: u16, cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
    let rs = cpu_regs.get_register((opcode >> 3) as u8 & 0b1111_0010, ProcessorMode::User);
    let imm = (opcode >> 6) & 0b11111;

    let end_value = match (opcode >> 11) & 0b11 {
        0b00 => rs << imm,
        0b01 => rs >> imm,
        0b10 => rs.rotate_right(imm as u32),
        _ => panic!("thumb move shifted register, invalid opcode")
    };

    let rd = cpu_regs.get_register_mut(opcode as u8 & 0b111, ProcessorMode::User);
    *rd = end_value;

    // updating the condition codes
    status.cpsr.z = end_value == 0;


}

fn thumb_add_sub(opcode: u16, cpu_regs: &mut CpuRegisters) {
    let rs = cpu_regs.get_register((opcode >> 3) as u8 & 0b111, ProcessorMode::User);
    let rn = match (opcode >> 9) & 1 == 1{
        true => {
            cpu_regs.get_register((opcode >> 6) as u8 & 0b111, ProcessorMode::User)
        }
        false => {
            ((opcode >> 6) & 0b111) as u32
        }
    };

    let rd = cpu_regs.get_register_mut(opcode as u8 & 0b111, ProcessorMode::User);
    match (opcode >> 10) & 1 == 1 {
        true => *rd = rn.wrapping_add(rs),
        false => *rd = rs.wrapping_sub(rn),
    }
}

fn thumb_alu_imm(opcode: u16, cpu_regs: &mut CpuRegisters) {
    let imm = opcode as u8 as u32;
    let rd = cpu_regs.get_register_mut((opcode >> 8) as u8 & 0b111, ProcessorMode::User);

    match (opcode >> 11) & 0b11 {
        0b00 => *rd = imm,
        0b01 => todo!(),
        0b10 => *rd = rd.wrapping_add(imm),
        0b11 => *rd = rd.wrapping_sub(imm),
        _ => unreachable!()
    }
}

fn thumb_alu_ops(opcode: u16, cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
    let rs = cpu_regs.get_register((opcode >> 3) as u8 & 0b111, ProcessorMode::User);
    let rd = cpu_regs.get_register_mut(opcode as u8 & 0b111, ProcessorMode::User);

    match (opcode >> 6) & 0b1111 {
        0b0000 => *rd &= rs,
        0b0001 => *rd ^= rs,
        0b0010 => *rd <<= rs,
        0b0011 => *rd >>= rs,
        0b0100 => *rd = rd.rotate_right(rs),
        0b0101 => *rd += rs + status.cpsr.c as u32,
        0b0110 => *rd -= rs + (!status.cpsr.c) as u32,
        _ => todo!()
    }
}