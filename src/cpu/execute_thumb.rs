use crate::cpu::registers::{*, status_registers::*};
use crate::cpu::decode::DecodedThumb;
use crate::memory::Memory;

// executing the Thumb instructions
// hopefully they are nice to do :)

/// during these instructions, i may not always use `status.cpsr.mode`,
/// this is because the register retrieval doesnt rely upon it as heavily as with ARM.
/// `ProcessorMode::User` is usually just used
pub fn execute_thumb(
    opcode: u16,
    instruction: DecodedThumb,
    cpu_regs: &mut Cpu,
    status: &mut Status,
    memory: &mut Memory,
) {
    use DecodedThumb::*;

    match instruction {
        MoveShiftedReg => move_shifted(opcode, cpu_regs, status),
        AddSubtract => add_sub(opcode, cpu_regs, status),
        AluImmediate => alu_imm(opcode, cpu_regs, status),
        AluOperation => alu_ops(opcode, cpu_regs, status),
        HiRegisterOperations => hi_operations(opcode, cpu_regs, status),
        PcRelativeLoad => pc_relative_load(opcode, cpu_regs, status, memory),
        LoadRegOffset => load_register_offset(opcode, cpu_regs, status, memory),
        LoadSignExtended => load_sign_extended(opcode, cpu_regs, status, memory),
        LoadImmOffset => load_imm_offset(opcode, cpu_regs, status, memory),
        LoadHalfword => load_halfword(opcode, cpu_regs, status, memory),
        SpRelativeLoad => sp_relative_load(opcode, cpu_regs, status, memory),
        LoadAddress => load_address(opcode, cpu_regs, status, memory),
        AddOffsetSp => offset_sp(opcode, cpu_regs, status),
        PushPop => push_pop(opcode, cpu_regs, status, memory),
        _ => todo!()
    }
}

fn move_shifted(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status) {
    let rd_index = opcode as u8 & 0b111;
    let rs_index = (opcode >> 3) as u8 & 0b111;

    let imm = (opcode >> 6) & 0x1F;

    let src = cpu_regs.get_register(rs_index, status.cpsr.mode);
    let op = (opcode >> 11) & 0b11;

    let result;
    let carry;
    match op {
        0b00 => {
            result = src << imm;
            carry = (src >> (32 - imm)) & 1 == 1;
        }
        0b01 => {
            result = src >> imm;
            carry = (src >> (imm - 1)) & 1 == 1;
        }
        0b10 => {
            let mut temp = src >> imm;
            if (src >> 31) & 1 == 1 {
                temp |= !((std::u32::MAX) >> imm);
            } 
            result = temp;
            carry = (src >> (imm - 1)) & 1 == 1;
        }
        _ => unreachable!(),
    }

    status.cpsr.c = carry;
    status.cpsr.z = result == 0;
    status.cpsr.n = (result >> 31) & 1 == 1;

    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = result;
}

fn add_sub(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status) {
    let rd_index = opcode as u8 & 0b111;
    let rs_index = (opcode >> 3) as u8 & 0b111;

    let value_bit = (opcode >> 10) & 1 == 1;
    let imm = (opcode >> 6) & 0b111;

    let offset;
    match value_bit {
        true => offset = imm as u32,
        false => offset = cpu_regs.get_register(imm as u8, status.cpsr.mode),
    }

    let sub_bit = (opcode >> 9) & 1 == 1;
    let rs = cpu_regs.get_register(rs_index, status.cpsr.mode);

    let result;
    let carry;
    match sub_bit {
        true => (result, carry) = rs.overflowing_sub(offset),
        false => (result, carry) = rs.overflowing_add(offset),
    }

    status.cpsr.c = carry;
    status.cpsr.z = result == 0;
    status.cpsr.n = (result >> 31) & 1 == 1;

    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = result;
}

fn alu_imm(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status) {
    let offset = opcode & 0xFF;

    let rd_index = (opcode >> 8) as u8 & 0b111;
    let rd = cpu_regs.get_register(rd_index, status.cpsr.mode);

    let result;
    let carry;
    let op = (opcode >> 11) & 0b11;
    match op {
        0b00 => (result, carry) = (offset as u32, false),
        0b01 => (result, carry) = rd.overflowing_sub(offset as u32),
        0b10 => (result, carry) = rd.overflowing_add(offset as u32),
        0b11 => (result, carry) = rd.overflowing_sub(offset as u32),
        _ => unreachable!(),
    }

    status.cpsr.c = carry;
    status.cpsr.n = (result >> 31) & 1 == 1;
    status.cpsr.z = result == 0;

    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = result;
}

fn alu_ops(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status) {
    let rd_index = opcode as u8 & 0b111;
    let rs_index = (opcode >> 3) as u8 & 0b111;

    let rd = cpu_regs.get_register(rd_index, status.cpsr.mode);
    let rs = cpu_regs.get_register(rs_index, status.cpsr.mode);

    let op = (opcode >> 6) & 0b1111;
    let result;
    let carry;


    let mut undo = false;
    match op {
        0b0000 => (result, carry) = (rd & rs, false),
        0b0001 => (result, carry) = (rd ^ rs, false),
        0b0010 => {
            result = rd << rs;
            carry = (rd >> (32 - rs)) & 1 == 1;
        }
        0b0011 => {
            result = rd >> rs;
            carry = (rd >> (rs - 1)) & 1 == 1;
        }
        0b0100 => {
            let mut temp = rd >> rs;
            if (rd >> 31) & 1 == 1 {
                temp |= !(std::u32::MAX >> rs);
            }
            result = temp;
            carry = (rd >> (rs - 1)) & 1 == 1;
        }
        0b0101 => {
            let temp = rd.overflowing_add(rs);
            let temp_2 = temp.0.overflowing_add(status.cpsr.c as u32);
            result = temp_2.0;
            carry = temp.1 | temp_2.1;
        }
        0b0110 => {
            let temp = rd.overflowing_sub(rs);
            let temp_2 = temp.0.overflowing_sub(!status.cpsr.c as u32);
            result = temp_2.0;
            carry = temp.1 | temp_2.1;
        }
        0b0111 => {
            result = rd.rotate_right(rs);
            carry = (result >> 31) & 1 == 1;
        }
        0b1000 => {
            undo = true;
            result = rd & rs;
            carry = false;
        }
        0b1001 => (result, carry) = (!rd + 1, false),
        0b1010 => {(result, carry) = rd.overflowing_sub(rs); undo = true},
        0b1011 => {(result, carry) = rd.overflowing_add(rs); undo = true},
        0b1100 => (result, carry) = (rd | rs, false),
        0b1101 => (result, carry) = rd.overflowing_mul(rs),
        0b1110 => (result, carry) = (rd & !rs, false),
        0b1111 => (result, carry) = (!rs, false),
        _ => unreachable!(),
    }

    status.cpsr.c = carry;
    status.cpsr.z = result == 0;
    status.cpsr.n = (result >> 31) & 1 == 1;
    
    // only mathematical instructions change the V flag
    if [0b0101, 0b0110, 0b1010, 0b1011].contains(&op) {
        status.cpsr.v = ((rs ^ result) & (rd ^ result)) >> 31 & 1 == 1; 
    }

    if undo {
        return;
    }

    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = result;
}

fn hi_operations(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status) {
    let mut rs_index = (opcode >> 3) as u8 & 0b111;
    let mut rd_index = opcode as u8 & 0b111;

    let h1 = (opcode >> 7) & 1 == 1;
    let h2 = (opcode >> 6) & 1 == 1;

    if h1 {
        rs_index += 8;
    }
    if h2 {
        rd_index += 8;
    }

    let op = (opcode >> 8) as u8 & 0b11;
    if !h1 && !h2 {
        assert!(op == 0b11, "H1=0, H2=0, instruction is invalid for these values");
    }

    let rs = cpu_regs.get_register(rs_index, status.cpsr.mode);
    let rd = cpu_regs.get_register(rd_index, status.cpsr.mode);

    let result;
    match op {
        0b00 => result = rd.wrapping_add(rs),
        0b01 => result = 0,
        0b10 => result = rs,
        0b11 => {
            assert!(h1, "H1=1 for this instruction is undefined");

            let pc = cpu_regs.get_register_mut(15, status.cpsr.mode);
            *pc = rs;
            status.cpsr.t = rs & 1 == 1;
            return;
        }
        _ => unreachable!(),
    }

    if op == 0b01 {
        // this is the only instruction that sets the codes
        let (result, carry) = rd.overflowing_sub(rs);
        status.cpsr.c = carry;
        status.cpsr.n = (result >> 31) & 1 == 1;
        status.cpsr.z = result == 0;
        status.cpsr.v = ((rs ^ result) & (rd ^ result)) >> 31 & 1 == 1;
    }

    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = result;
}

fn pc_relative_load(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status, memory: &mut Memory) {
    let rd_index = (opcode >> 8) as u8 & 0b111;
    let imm = (opcode & 0xFF) << 2;

    let pc = cpu_regs.get_register(15, status.cpsr.mode);
    let address = pc + imm as u32;
    let read = memory.read_u32(address);

    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = read;
}

fn load_register_offset(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status, memory: &mut Memory) {
    let ro_index = (opcode >> 6) as u8 & 0b111;
    let rb_index = (opcode >> 3) as u8 & 0b111;
    let rd_index = opcode as u8 & 0b111;

    let ro = cpu_regs.get_register(ro_index, status.cpsr.mode);
    let rb = cpu_regs.get_register(rb_index, status.cpsr.mode);

    let address = ro + rb;

    let l_bit = (opcode >> 11) & 1 == 1;
    let b_bit = (opcode >> 10) & 1 == 1;

    match l_bit {
        true => {
            let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
            match b_bit {
                true => *rd = memory.read_u8(address) as u32,
                false => *rd = memory.read_u32(address),
            }
        }
        false => {
            let rd = cpu_regs.get_register(rd_index, status.cpsr.mode);
            match b_bit {
                true => memory.write_u8(address, rd as u8),
                false => memory.write_u32(address, rd),
            }
        }
    }
}

fn load_sign_extended(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status, memory: &mut Memory) {
    let ro_index = (opcode >> 6) as u8 & 0b111;
    let rb_index = (opcode >> 3) as u8 & 0b111;
    let rd_index = opcode as u8 & 0b111;

    let ro = cpu_regs.get_register(ro_index, status.cpsr.mode);
    let rb = cpu_regs.get_register(rb_index, status.cpsr.mode);

    let address = ro + rb;

    let s_bit = (opcode >> 10) & 1 == 1;
    let h_bit = (opcode >> 11) & 1 == 1;

    match (s_bit, h_bit) {
        (false, false) => {
            let rd = cpu_regs.get_register(rd_index, status.cpsr.mode);
            memory.write_u16(address, rd as u16);
        }
        (false, true) => {
            let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
            *rd = memory.read_u16(address) as u32;
        }
        (true, false) => {
            let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
            let temp = memory.read_u8(address);

            *rd = temp as u32;
            if (temp >> 7) & 1 == 1 {
                *rd |= 0xFFFFFF00;
            } 
        }
        (true, true) => {
            let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
            let temp = memory.read_u16(address);

            *rd = temp as u32;
            if (temp >> 15) & 1 == 1 {
                *rd |= 0xFFFF0000;
            }
        }
    }
}

fn load_imm_offset(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status, memory: &mut Memory) {
    let rd_index = opcode as u8 & 0b111;
    let rb_index = (opcode >> 3) as u8 & 0b111;
    let offset = (opcode >> 6) & 0b11111;

    let rb = cpu_regs.get_register(rb_index, status.cpsr.mode);

    let l_bit = (opcode >> 11) & 1 == 1;
    let b_bit = (opcode >> 10) & 1 == 1;

    let address;
    match b_bit {
        true => address = rb + offset as u32,
        false => address = rb + (offset as u32) << 2,
    }

    match l_bit {
        true => {
            let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
            match b_bit {
                true => *rd = memory.read_u8(address) as u32,
                false => *rd = memory.read_u32(address),
            }
        }
        false => {
            let rd = cpu_regs.get_register(rd_index, status.cpsr.mode);
            match b_bit {
                true => memory.write_u8(address, rd as u8),
                false => memory.write_u32(address, rd),
            }
        }
    }
}

fn load_halfword(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status, memory: &mut Memory) {
    let rd_index = opcode as u8 & 0b111;
    let rb_index = (opcode >> 3) as u8 & 0b111;

    let imm = (opcode >> 6) & 0b11111;
    let rb = cpu_regs.get_register(rb_index, status.cpsr.mode);

    let address = rb + imm as u32;

    let l_bit = (opcode >> 11) & 1 == 1;
    match l_bit {
        true => {
            let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
            *rd = memory.read_u16(address) as u32;
        }
        false => {
            let rd = cpu_regs.get_register(rd_index, status.cpsr.mode);
            memory.write_u16(address, rd as u16);
        }
    }
}

fn sp_relative_load(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status, memory: &mut Memory) {
    let rd_index = (opcode >> 8) as u8 & 0b111;
    let imm = opcode & 0xFF;

    let sp = cpu_regs.get_register(13, status.cpsr.mode);
    let address = sp + imm as u32;

    let l_bit = (opcode >> 11) & 1 == 1;
    match l_bit {
        true => {
            let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
            *rd = memory.read_u32(address);
        }
        false => {
            let rd = cpu_regs.get_register(rd_index, status.cpsr.mode);
            memory.write_u32(address, rd);
        }
    }
}

fn load_address(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status, memory: &mut Memory) {
    let imm = (opcode & 0xFF) << 2;
    let rd_index = (opcode >> 8) as u8 & 0b111;

    let sp_bit = (opcode >> 11) & 1 == 1;
    let src;
    match sp_bit {
        true => src = cpu_regs.get_register(13, status.cpsr.mode),
        false => src = cpu_regs.get_register(15, status.cpsr.mode),
    }

    let address = src + imm as u32;
    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = memory.read_u32(address);
}

fn offset_sp(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status) {
    let offset = opcode & 0b111_1111;
    let s_bit = (opcode >> 7) & 1 == 1;

    let sp = cpu_regs.get_register_mut(13, status.cpsr.mode);
    match s_bit {
        true => *sp -= offset as u32,
        false => *sp += offset as u32,
    }
}

fn push_pop(opcode: u16, cpu_regs: &mut Cpu, status: &mut Status, memory: &mut Memory) {
    let rlist = opcode & 0xFF;
    
    let r_bit = (opcode >> 8) & 1 == 1;
    let l_bit = (opcode >> 11) & 1 == 1;

    let mut used_address = cpu_regs.get_register(13, status.cpsr.mode);
    
    match l_bit {
        true => { // POP
            if r_bit {
                
            }
        }
        false => { // PUSH

        }
    }
}
