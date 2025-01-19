use crate::cpu::registers::{*, status_registers::*};
use crate::cpu::decode::DecodedThumb;
use crate::memory::Memory;

pub fn execute_thumb(
    opcode: u16,
    instruction: DecodedThumb,
    cpu_regs: &mut Cpu,
    status: &mut CpuStatus,
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
        LoadAddress => load_address(opcode, cpu_regs, status),
        AddOffsetSp => offset_sp(opcode, cpu_regs, status),
        PushPop => push_pop(opcode, cpu_regs, status, memory),
        MultipleLoadStore => multiple_load(opcode, cpu_regs, status, memory),
        ConditionalBranch => conditional_branch(opcode, cpu_regs, status),
        Swi => todo!(),
        UnconditionalBranch => unconditional_branch(opcode, cpu_regs, status),
        LongBranchLink => long_branch_link(opcode, cpu_regs, status),
    }
}

fn move_shifted(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    let rs_index = (opcode >> 3) as u8 & 0x7;
    let rs = cpu_regs.get_register(rs_index, status.cpsr.mode);

    let imm = (opcode >> 6) as u32 & 0x1F;

    let (result, carry);
    let op = (opcode >> 11) & 0b11;
    match op {
        0b00 => {
            result = rs << imm;
            carry = (rs << (imm - 1)) >> 31 & 1 == 1;
        }
        0b01 => {
            result = rs >> imm;
            carry = (rs >> (imm - 1)) & 1 == 1;
        }
        0b10 => {
            let mut temp = rs >> imm;
            if (rs >> 31) & 1 == 1 {
                temp |= !((std::u32::MAX) >> imm);
            } 
            result = temp;
            carry = (rs >> (imm - 1)) & 1 == 1;
        }
        _ => unreachable!(),
    }

    status.cpsr.c = carry;
    status.cpsr.z = result == 0;
    status.cpsr.n = (result >> 31) & 1 == 1;

    let rd_index = opcode as u8 & 0x7;
    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = result;
}

fn add_sub(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    let rd_index = opcode & 0x7;
    let rs_index = (opcode >> 3) & 0x7;
    let value = (opcode >> 6) & 0x7;

    let i_bit = (opcode >> 10) & 1 == 1;
    let mut offset = match i_bit {
        true => value as u32,
        false => cpu_regs.get_register(value as u8, status.cpsr.mode),
    };

    let op = (opcode >> 9) & 1 == 1;
    if op { // this means its a subtraction
        offset = (!offset).wrapping_add(1);
    }

    let rs = cpu_regs.get_register(rs_index as u8, status.cpsr.mode);
    let (result, carry) = rs.overflowing_add(offset);

    status.cpsr.c = carry;
    status.cpsr.z = result == 0;
    status.cpsr.n = (result >> 31) & 1 == 1;
    status.cpsr.v = ((rs ^ result) & (offset ^ result)) >> 31 & 1 == 1;

    let rd = cpu_regs.get_register_mut(rd_index as u8, status.cpsr.mode);
    *rd = result;

}

fn alu_imm(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    let mut offset = (opcode as u32) & 0xFF;
    let rd_index = (opcode >> 8) as u8 & 0x7;
    let rd = cpu_regs.get_register(rd_index, status.cpsr.mode);


    let op = (opcode >> 11) & 0x3;
    if op & 1 == 1 { // all the sub instructions
        offset = (!offset).wrapping_add(1);
    }

    let (result, carry) = match op {
        0b00 => (offset, false),
        0b01..=0b11 => rd.overflowing_add(offset),
        _ => unreachable!(),
    };

    status.cpsr.c = carry;
    status.cpsr.n = (result >> 31) & 1 == 1;
    status.cpsr.z = result == 0;
    status.cpsr.v = ((rd ^ result) & (offset ^ result)) >> 31 & 1 == 1;

    // CMP doesnt change the value
    if op == 1 {
        return;
    }
    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = result;
}

fn alu_ops(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    let rd_index = opcode as u8 & 0x7;
    let rs_index = (opcode >> 3) as u8 & 0x7;
    let rd = cpu_regs.get_register(rd_index, status.cpsr.mode);
    let rs = cpu_regs.get_register(rs_index, status.cpsr.mode);

    let op = (opcode >> 6) & 0xF;
    let mut undo = false;

    let (result, alu_carry) = match op {
        0b0000 => (rd & rs, false), // and
        0b0001 => (rd ^ rs, false), // eor
        0b0010 => {
            let result = rd.wrapping_sub(rs);
            (result, rd >= rs)
        }, // sub
        0b0011 => {
            let result = rs.wrapping_sub(rd);
            (result, rs >= rd)
        }, // rsb
        0b0100 => rd.overflowing_add(rs), // add
        0b0101 => {
            let (inter_res, inter_of) = rd.overflowing_add(rs);
            let (end_res, end_of) = inter_res.overflowing_add(status.cpsr.c as u32);
            (end_res, inter_of | end_of)
        }, // adc
        0b0110 => {
            let subtract_operand = rs.wrapping_add(1 - status.cpsr.c as u32);
            let result = rd.wrapping_sub(subtract_operand);
            (result, rd >= subtract_operand)
        }, // sbc,
        0b0111 => {
            let subtract_operand = rd.wrapping_add(1 - status.cpsr.c as u32);
            let result = rs.wrapping_sub(subtract_operand);
            (result, rs >= subtract_operand)
        }, // rsc
        0b1000 => {
            undo = true; 
            (rd & rs, false)
        }, // tst
        0b1001 => {
            undo = true; 
            (rd ^ rs, false)
        }, // teq
        0b1010 => {
            undo = true;
            let result = rd.wrapping_sub(rs);
            (result, rd >= rs)
        }, // cmp
        0b1011 => {
            undo = true; 
            rd.overflowing_add(rs)
        }, // cmn
        0b1100 => {
            (rd | rs, false)
        }, // orr
        0b1101 => (rs, false), // mov
        0b1110 => (rd & !rs, false), // bic
        0b1111 => (!rs, false), // mvn
        _ => unreachable!()
    };

    status.cpsr.c = alu_carry;
    status.cpsr.z = result == 0;
    status.cpsr.n = (result >> 31) & 1 == 1;
    
    // only mathematical instructions change the V flag
    if [0b0010, 0b0011, 0b0100, 0b0101, 0b0110, 0b0111, 0b1010, 0b1011].contains(&op) {
        status.cpsr.v = (result >= (1 << 31)) | (result as i32 <= (-1 << 31));
    }

    if undo {
        return;
    }

    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = result;
}

fn hi_operations(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    let mut rs_index = (opcode >> 3) as u8 & 0b111;
    let mut rd_index = opcode as u8 & 0b111;

    let h1 = (opcode >> 7) & 1 == 1;
    let h2 = (opcode >> 6) & 1 == 1;

    if h1 {
        rd_index += 8;
    }
    if h2 {
        rs_index += 8;
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
        0b01 => {
            // this is the only instruction that sets the codes
            let result = rd.wrapping_sub(rs);
            status.cpsr.c = (result >> 31) & 1 == 1;
            status.cpsr.n = (result >> 31) & 1 == 1;
            status.cpsr.z = result == 0;
            status.cpsr.v = ((rs ^ result) & (rd ^ result)) >> 31 & 1 == 1;
            return;
        },
        0b10 => result = rs,
        0b11 => {
            assert!(!h1, "H1=1 for this instruction is undefined");

            let pc = cpu_regs.get_register_mut(15, status.cpsr.mode);
            *pc = rs & !(0b1);

            status.cpsr.t = (rs & 1) == 1;
            cpu_regs.clear_pipeline = true;
            return;
        }
        _ => unreachable!(),
    }

    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = result;
    if rd_index == 15 {
        *rd &= !(0b1);
        cpu_regs.clear_pipeline = true;
    }

}

fn pc_relative_load(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus, memory: &mut Memory) {
    let rd_index = (opcode >> 8) as u8 & 0b111;
    let imm = (opcode & 0xFF) << 2;

    let pc = cpu_regs.get_register(15, status.cpsr.mode) & 0xFFFFFFFD;
    let address = pc + imm as u32;
    let read = memory.read_u32(address);

    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = read;
}

fn load_register_offset(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus, memory: &mut Memory) {
    let ro_index = (opcode >> 6) as u8 & 0b111;
    let rb_index = (opcode >> 3) as u8 & 0b111;
    let rd_index = opcode as u8 & 0b111;

    let ro = cpu_regs.get_register(ro_index, status.cpsr.mode);
    let rb = cpu_regs.get_register(rb_index, status.cpsr.mode);

    let address = ro.wrapping_add(rb);

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

fn load_sign_extended(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus, memory: &mut Memory) {
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

fn load_imm_offset(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus, memory: &mut Memory) {
    let rd_index = opcode as u8 & 0b111;
    let rb_index = (opcode >> 3) as u8 & 0b111;
    let offset = (opcode >> 6) & 0b1_1111;

    let rb = cpu_regs.get_register(rb_index, status.cpsr.mode);
    let l_bit = (opcode >> 11) & 1 == 1;
    let b_bit = (opcode >> 12) & 1 == 1;

    let address;
    match b_bit {
        true => address = rb + offset as u32,
        false => address = rb + ((offset as u32) << 2),
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

fn load_halfword(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus, memory: &mut Memory) {
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

fn sp_relative_load(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus, memory: &mut Memory) {
    let rd_index = (opcode >> 8) as u8 & 0b111;
    let imm = opcode & 0xFF;

    let sp = cpu_regs.get_register(13, status.cpsr.mode);
    let address = sp + (imm << 2) as u32;

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

fn load_address(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    let imm = (opcode & 0xFF) << 2;
    let rd_index = (opcode >> 8) as u8 & 0b111;

    let sp_bit = (opcode >> 11) & 1 == 1;
    let src;
    match sp_bit {
        true => src = cpu_regs.get_register(13, status.cpsr.mode),
        false => src = cpu_regs.get_register(15, status.cpsr.mode) & !(0b11),
    }

    let address = src + imm as u32;
    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = address;
}

fn offset_sp(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    let offset = (opcode as u32 & 0x7F) << 2;
    let s_bit = (opcode >> 7) & 1 == 1;
    
    let sp = cpu_regs.get_register_mut(13, status.cpsr.mode);
    match s_bit {
        true => *sp = sp.wrapping_sub(offset),
        false => *sp = sp.wrapping_add(offset),
    }
}

fn push_pop(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus, memory: &mut Memory) {
    let mut rlist = opcode & 0xFF;
    let l_bit = (opcode >> 11) & 1 == 1;
    let r_bit = (opcode >> 8) & 1 == 1;

    let sp = cpu_regs.get_register(13, status.cpsr.mode);
    match l_bit {
        true => { // load
            let mut base_address = sp;
            while rlist != 0 {
                let next_r = rlist.trailing_zeros();

                let reg = cpu_regs.get_register_mut(next_r as u8, status.cpsr.mode);
                let change = memory.read_u32(base_address);
                *reg = change;
                
                base_address += 4;
                rlist &= !(1<<next_r);
            }
            if r_bit {
                let reg = cpu_regs.get_register_mut(15, status.cpsr.mode);
                let change = memory.read_u32(base_address);
                *reg = change & !(1);
                base_address += 4;
                cpu_regs.clear_pipeline = true;
            }
            let sp_mut = cpu_regs.get_register_mut(13, status.cpsr.mode);
            *sp_mut = base_address;
        }
        false => {
            let total_increments = rlist.count_ones() + r_bit as u32;

            let mut base_address = sp - (total_increments * 4);
            let base_address_copy = base_address;
            while rlist != 0 {
                let next_r = rlist.trailing_zeros();
                let reg = cpu_regs.get_register(next_r as u8, status.cpsr.mode);
                memory.write_u32(base_address, reg);
                base_address += 4;
                rlist &= !(1<<next_r);
            }
            if r_bit {
                let reg = cpu_regs.get_register(14, status.cpsr.mode);
                memory.write_u32(base_address, reg);
            }

            let sp = cpu_regs.get_register_mut(13, status.cpsr.mode);
            *sp = base_address_copy;
        }
    }
}

fn multiple_load(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus, memory: &mut Memory) {
    let mut rlist = opcode & 0xFF;
    assert!(rlist != 0, "Register list provided cannot be 0");

    let rb_index = (opcode >> 8) as u8 & 0b111;
    let rb = cpu_regs.get_register(rb_index, status.cpsr.mode);

    let l_bit = (opcode >> 11) & 1 == 1;
    match l_bit {
        true => { // load
            let mut curr_address = rb;
            while rlist != 0 {
                let next_r = rlist.trailing_zeros();

                let reg = cpu_regs.get_register_mut(next_r as u8, status.cpsr.mode);
                let change = memory.read_u32(curr_address);
                *reg = change;
                
                curr_address += 4;
                rlist &= !(1<<next_r);
            }
            let rb_mut = cpu_regs.get_register_mut(rb_index, status.cpsr.mode);
            *rb_mut = curr_address;
        }
        false => {
            let total_increments = rlist.count_ones();
            let mut base_address = rb - (total_increments * 4);
            let base_address_copy = base_address;

            while rlist != 0 {
                let next_r = rlist.trailing_zeros();
                let reg = cpu_regs.get_register(next_r as u8, status.cpsr.mode);
                memory.write_u32(base_address, reg);
                base_address += 4;
                rlist &= !(1<<next_r);
            }

            let rb_mut = cpu_regs.get_register_mut(rb_index, status.cpsr.mode);
            *rb_mut = base_address_copy;
        }
    }
}

fn unconditional_branch(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    let mut offset = (opcode as u32 & 0x3FF) << 1;
    if (opcode >> 10) & 1 == 1 {
        offset |= 0xFFFFF800;
    }

    let pc = cpu_regs.get_register_mut(15, status.cpsr.mode);
    *pc = pc.wrapping_add_signed(offset as i32);
    cpu_regs.clear_pipeline = true;
}

fn conditional_branch(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    let condition = (opcode >> 8) & 0xF;
    if !check_condition(condition as u32, &status.cpsr) {
        return;
    }

    let pc = cpu_regs.get_register_mut(15, status.cpsr.mode);
    let mut offset = (opcode & 0xFF) as u32;
    offset <<= 1;
    if (offset >> 8) & 1 == 1 {
        offset |= 0xFFFFFF00;
    }
    *pc = pc.wrapping_add_signed(offset as i32);
    cpu_regs.clear_pipeline = true;
}

fn long_branch_link(opcode: u16, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    let mut offset = opcode as u32 & 0x7FF;
    let h_bit = (opcode >> 11) & 1 == 1;

    match h_bit {
        false => {
            if (offset >> 10) & 1 == 1 {
                offset |= 0xFFFFF800;
            }
            offset <<= 12;

            let pc = cpu_regs.get_register(15, status.cpsr.mode);
            let lr = cpu_regs.get_register_mut(14, status.cpsr.mode);
            *lr = pc.wrapping_add(offset);
        },
        true => {
            offset <<= 1;
            let lr = cpu_regs.get_register(14, status.cpsr.mode);
            let pc = cpu_regs.get_register_mut(15, status.cpsr.mode);

            let temp = *pc - 2;
            *pc = lr.wrapping_add(offset);
            let lr = cpu_regs.get_register_mut(14, status.cpsr.mode);
            *lr = temp | 1;
            cpu_regs.clear_pipeline = true;
        },
    };
}