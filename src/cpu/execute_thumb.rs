use crate::cpu::*;
use crate::cpu::decode::DecodedThumb;
use crate::memory::Memory;

use super::get_shifted_value;

pub fn execute_thumb(
    opcode: u16,
    instruction: DecodedThumb,
    cpu: &mut Cpu,
    memory: &mut Memory,
) {
    use DecodedThumb::*;

    match instruction {
        MoveShifted => move_shifted(opcode, cpu),
        AddSub => add_sub(opcode, cpu),
        AluImmediate => alu_imm(opcode, cpu),
        AluOperation => alu_ops(opcode, cpu),
        HiRegister => hi_ops(opcode, cpu),
        PcRelativeLoad => pc_relative_load(opcode, cpu, memory),
        MemRegOffset => mem_offset(opcode, cpu, memory, false),
        MemSignExtended => mem_sign_extended(opcode, cpu, memory),
        MemImmOffset => mem_offset(opcode, cpu, memory, true),
        MemHalfword => mem_halfword(opcode, cpu, memory),
        MemSpRelative => mem_sp_relative(opcode, cpu, memory),
        LoadAddress => load_address(opcode, cpu),
        OffsetSp => offset_sp(opcode, cpu),
        PushPop => push_pop(opcode, cpu, memory),
        MemMultiple => mem_multiple(opcode, cpu, memory),
        CondBranch => conditional_branch(opcode, cpu),
        Swi => software_interrupt(cpu),
        UncondBranch => unconditional_branch(opcode, cpu),
        LongBranch => long_branch_link(opcode, cpu),
    }
}

fn move_shifted(opcode: u16, cpu: &mut Cpu) {
    let rs_index = (opcode >> 3) as u8 & 0x7;
    let imm = (opcode >> 6) as u32 & 0x1F;

    let (result, carry);
    let op = (opcode >> 11) & 0b11;

    // convert this opcode into the arm version
    // its just easier than doing it all over again
    let mut shift = (rs_index as u32) | ((imm as u32) << 7);

    match op {
        0b00 => {
            shift |= 0b00 << 5;
            (result, carry) = get_shifted_value(cpu, shift); 
        }
        0b01 => {
            shift |= 0b01 << 5;
            (result, carry) = get_shifted_value(cpu, shift); 
        }
        0b10 => {
            shift |= 0b10 << 5;
            (result, carry) = get_shifted_value(cpu, shift); 
        }
        _ => unreachable!(),
    }

    cpu.cpsr.c = carry;
    cpu.cpsr.z = result == 0;
    cpu.cpsr.n = (result >> 31) & 1 == 1;

    let rd_index = opcode as u8 & 0x7;
    let rd = cpu.get_register_mut(rd_index);
    *rd = result;
}
fn add_sub(opcode: u16, cpu: &mut Cpu) {
    let rd_index = opcode & 0x7;
    let rs_index = (opcode >> 3) & 0x7;
    let rs = cpu.get_register(rs_index as u8);
    let value = (opcode >> 6) & 0x7;

    let i_bit = (opcode >> 10) & 1 == 1;
    let offset = match i_bit {
        true => value as u32,
        false => cpu.get_register(value as u8),
    };

    let op = (opcode >> 9) & 1 == 1;
    let result;
    match op {
        true => { // sub
            let (result1, carry1) = (!offset).overflowing_add(1);
            let (result2, carry2) = rs.overflowing_add(result1);

            cpu.cpsr.v = (!(rs ^ !offset) & (rs ^ result2)) >> 31 & 1 == 1;
            cpu.cpsr.c = carry1 | carry2;
            result = result2;
        }
        false => { // add
            let (temp, carry) = rs.overflowing_add(offset);
            cpu.cpsr.c = carry;
            cpu.cpsr.v = ((rs ^ temp) & (offset ^ temp)) >> 31 & 1 == 1;
            result = temp;
        }
    }

    cpu.cpsr.z = result == 0;
    cpu.cpsr.n = (result >> 31) & 1 == 1;

    let rd = cpu.get_register_mut(rd_index as u8);
    *rd = result;

}
fn alu_imm(opcode: u16, cpu: &mut Cpu) {
    let offset = (opcode as u32) & 0xFF;
    let rd_index = (opcode >> 8) as u8 & 0x7;
    let rd = cpu.get_register(rd_index);


    let op = (opcode >> 11) & 0x3;
    let (result, carry);
    match op {
        0b00 => {
            carry = false;
            result = offset;
        }
        0b01 => {
            let (result1, carry1) = (!offset).overflowing_add(1);
            let (result2, carry2) = rd.overflowing_add(result1);

            cpu.cpsr.v = (rd ^ result1) >> 31 == 0 && (rd ^ result2) >> 31 == 1;
            result = result2;
            carry = carry1 | carry2;
        }
        0b10 => {
            (result, carry) = rd.overflowing_add(offset);
            cpu.cpsr.v = ((rd ^ result) & (offset ^ result)) >> 31 & 1 == 1;
        }
        0b11 => {
            let (result1, carry1) = (!offset).overflowing_add(1);
            let (result2, carry2) = rd.overflowing_add(result1);

            cpu.cpsr.v = (rd ^ result1) >> 31 == 0 && (rd ^ result2) >> 31 == 1;
            carry = carry1 | carry2;
            result = result2;
        }
        _ => unreachable!(),
    }

    cpu.cpsr.c = carry;
    cpu.cpsr.n = (result >> 31) & 1 == 1;
    cpu.cpsr.z = result == 0;

    // CMP doesnt change the value
    if op == 1 {
        return;
    }
    let rd = cpu.get_register_mut(rd_index);
    *rd = result;
}
fn alu_ops(opcode: u16, cpu: &mut Cpu) {
    let rd_index = opcode as u8 & 0x7;
    let rs_index = (opcode >> 3) as u8 & 0x7;
    let rd = cpu.get_register(rd_index);
    let rs = cpu.get_register(rs_index);

    let op = (opcode >> 6) & 0xF;
    let mut undo = false;
    let (result, alu_carry) = match op {
        0b0000 => (rd & rs, cpu.cpsr.c), // and
        0b0001 => {
            (rd ^ rs, cpu.cpsr.c)
        }, // eor
        0b0010 => {
            let sent_opcode = 
                (rs_index as u32 & 0xF) << 8 |
                (0b0001) << 4 |   
                rd_index as u32 & 0xF;
            get_shifted_value(cpu, sent_opcode)
        } // lsl
        0b0011 => {
            let sent_opcode = 
                (rs_index as u32 & 0xF) << 8 |
                (0b0011) << 4 |   
                rd_index as u32 & 0xF;
            
            let temp = get_shifted_value(cpu, sent_opcode);
            temp
        } // lsr
        0b0100 => {
            // convert the opcode
            let sent_opcode = 
                (rs_index as u32 & 0xF) << 8 |
                (0b0101) << 4 |   
                rd_index as u32 & 0xF;
            get_shifted_value(cpu, sent_opcode)
        } // asr
        0b0101 => {
            let (inter_res, inter_of) = rd.overflowing_add(rs);
            let (end_res, end_of) = inter_res.overflowing_add(cpu.cpsr.c as u32);
            (end_res, inter_of | end_of)
        }, // adc
        0b0110 => {
            let (result1, carry1) = (!rs).overflowing_add(cpu.cpsr.c as u32);
            let (result2, carry2) = rd.overflowing_add(result1);

            cpu.cpsr.v = (!(rd ^ !rs) & (rd ^ result2)) >> 31 & 1 == 1;
            (result2, carry1 | carry2)
        }, // sbc,
        0b0111 => {
            let sent_opcode = 
                (rs_index as u32 & 0xF) << 8 |
                (0b0111) << 4 |   
                rd_index as u32 & 0xF;
            get_shifted_value(cpu, sent_opcode)
        }, // ror
        0b1000 => {
            undo = true; 
            (rd & rs, cpu.cpsr.c)
        }, // tst
        0b1001 => (0_u32.wrapping_sub(rs), cpu.get_barrel_shift()), // teq
        0b1010 => {
            undo = true;
            let (result1, carry1) = (!rs).overflowing_add(1);
            let (result2, carry2) = rd.overflowing_add(result1);

            cpu.cpsr.v = (!(rd ^ !rs) & (rd ^ result2)) >> 31 & 1 == 1;
            (result2, carry1 | carry2)
        }, // cmp
        0b1011 => {
            undo = true; 
            rd.overflowing_add(rs)
        }, // cmn
        0b1100 => {
            (rd | rs, cpu.cpsr.c)
        }, // orr
        0b1101 => rd.overflowing_mul(rs), // mul
        0b1110 => (rd & !rs, cpu.cpsr.c), // bic
        0b1111 => (!rs, cpu.cpsr.c), // mvn
        _ => unreachable!()
    };

    cpu.cpsr.c = alu_carry;
    cpu.cpsr.z = result == 0;
    cpu.cpsr.n = (result >> 31) & 1 == 1;
    
    // only mathematical instructions change the V flag
    // all of the subtracts have already been handled
    if [0b0101, 0b1011].contains(&op) {
        cpu.cpsr.v = (!(rs ^ rd) & (rd ^ result)) >> 31 & 1 == 1;
    }

    if undo {
        return;
    }

    let rd = cpu.get_register_mut(rd_index);
    *rd = result;
}
fn hi_ops(opcode: u16, cpu: &mut Cpu) {
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

    let rs = cpu.get_register(rs_index);
    let rd = cpu.get_register(rd_index);

    let result;
    match op {
        0b00 => result = rd.wrapping_add(rs),
        0b01 => {
            // this is the only instruction that sets the codes
            let (result1, carry1) = (!rs).overflowing_add(1);
            let (result2, carry2) = rd.overflowing_add(result1);

            cpu.cpsr.v = ((rd ^ rs) & (rd ^ result2)) >> 31 & 1 == 1;
            cpu.cpsr.c = carry1 | carry2;
            cpu.cpsr.n = (result2 >> 31) & 1 == 1;
            cpu.cpsr.z = result2 == 0;
            return;
        },
        0b10 => result = rs,
        0b11 => {
            assert!(!h1, "H1=1 for this instruction is undefined");

            let pc = cpu.get_register_mut(15);
            *pc = rs & !(0b1);

            cpu.cpsr.t = (rs & 1) == 1;
            cpu.clear_pipeline = true;
            return;
        }
        _ => unreachable!(),
    }

    let rd = cpu.get_register_mut(rd_index);
    *rd = result;
    if rd_index == 15 {
        *rd &= !(0b1);
        cpu.clear_pipeline = true;
    }

}
fn pc_relative_load(opcode: u16, cpu: &mut Cpu, memory: &mut Memory) {
    let rd_index = (opcode >> 8) as u8 & 0b111;
    let imm = (opcode & 0xFF) << 2;

    let pc = cpu.get_register(15) & 0xFFFFFFFC;
    let address = pc.wrapping_add(imm as u32);
    let read = memory.read_u32(address);

    let rd = cpu.get_register_mut(rd_index);
    *rd = read;
}
fn mem_offset(opcode: u16, cpu: &mut Cpu, memory: &mut Memory, uses_imm: bool) {
    let rb_index = (opcode >> 3) as u8 & 0b111;
    let rd_index = opcode as u8 & 0b111;

    let rb = cpu.get_register(rb_index);

    let (address, l_bit, b_bit);
    match uses_imm {
        true => {
            l_bit = (opcode >> 11) & 1 == 1;
            b_bit = (opcode >> 12) & 1 == 1;
            let imm = (opcode >> 6) as u32 & 0b1_1111;
            match b_bit {
                true => address = rb.wrapping_add(imm),
                false => address = rb.wrapping_add(imm << 2),
            }
        }
        false => {
            l_bit = (opcode >> 11) & 1 == 1;
            b_bit = (opcode >> 10) & 1 == 1;

            let ro_index = (opcode >> 6) & 0x7;
            let ro = cpu.get_register(ro_index as u8);
            address = rb.wrapping_add(ro);
        }
    }

    match l_bit {
        true => {
            let rd = cpu.get_register_mut(rd_index);
            match b_bit {
                true => *rd = memory.read_u8(address) as u32,
                false => *rd = memory.read_u32(address),
            }
        }
        false => {
            let rd = cpu.get_register(rd_index);
            match b_bit {
                true => memory.write_u8(address, rd as u8),
                false => memory.write_u32(address, rd),
            }
        }
    }
}
fn mem_sign_extended(opcode: u16, cpu: &mut Cpu, memory: &mut Memory) {
    let ro_index = (opcode >> 6) as u8 & 0b111;
    let rb_index = (opcode >> 3) as u8 & 0b111;
    let rd_index = opcode as u8 & 0b111;

    let ro = cpu.get_register(ro_index);
    let rb = cpu.get_register(rb_index);

    let address = ro + rb;
    let sh = (opcode >> 10) & 0b11;

    match sh {
        0b00 => { // STRH
            let rd = cpu.get_register(rd_index);
            memory.write_u16(address, rd as u16);
        }
        0b10 => { // LDRH
            let rd = cpu.get_register_mut(rd_index);
            *rd = (memory.read_u16(address & !(0b1)) as u32).rotate_right((address % 2) * 8);
        }
        0b01 => {
            let mut raw_reading = memory.read_u8(address) as u32;
            if (raw_reading >> 7) & 1 == 1 {
                raw_reading |= 0xFFFFFF00;
            }

            let rd = cpu.get_register_mut(rd_index);
            *rd = raw_reading;
        }
        0b11 => {
            let mut raw_reading;
            let is_aligned = address & 1 == 1;
            match is_aligned {
                true => {
                    raw_reading = memory.read_u8(address) as u32;
                    if (raw_reading >> 7) & 1 == 1 {
                        raw_reading |= 0xFFFFFF00;
                    }
                },
                false => {
                    raw_reading = memory.read_u16(address & !(1)) as u32;
                    if (raw_reading >> 15) & 1 == 1 {
                        raw_reading |= 0xFFFF0000;
                    }
                }
            }

            let rd = cpu.get_register_mut(rd_index);
            *rd = raw_reading;
        }
        _ => unreachable!()
    }
}
fn mem_halfword(opcode: u16, cpu: &mut Cpu, memory: &mut Memory) {
    let rd_index = opcode as u8 & 0b111;
    let rb_index = (opcode >> 3) as u8 & 0b111;

    let imm = ((opcode >> 6) & 0b11111) << 1;
    let rb = cpu.get_register(rb_index);

    let address = rb + imm as u32;

    let l_bit = (opcode >> 11) & 1 == 1;
    match l_bit {
        true => {
            let rd = cpu.get_register_mut(rd_index);
            *rd = (memory.read_u16(address & !(0b1)) as u32).rotate_right((address%2) * 8);
        }
        false => {
            let rd = cpu.get_register(rd_index);
            memory.write_u16(address, rd as u16);
        }
    }
}
fn mem_sp_relative(opcode: u16, cpu: &mut Cpu, memory: &mut Memory) {
    let rd_index = (opcode >> 8) as u8 & 0b111;
    let imm = opcode & 0xFF;

    let sp = cpu.get_register(13);
    let address = sp + (imm << 2) as u32;

    let l_bit = (opcode >> 11) & 1 == 1;
    match l_bit {
        true => {
            let rd = cpu.get_register_mut(rd_index);
            *rd = memory.read_u32(address);
        }
        false => {
            let rd = cpu.get_register(rd_index);
            memory.write_u32(address, rd);
        }
    }
}
fn load_address(opcode: u16, cpu: &mut Cpu) {
    let imm = (opcode & 0xFF) << 2;
    let rd_index = (opcode >> 8) as u8 & 0b111;

    let sp_bit = (opcode >> 11) & 1 == 1;
    let src;
    match sp_bit {
        true => src = cpu.get_register(13),
        false => src = cpu.get_register(15) & !(0b11),
    }

    let address = src + imm as u32;
    let rd = cpu.get_register_mut(rd_index);
    *rd = address;
}
fn offset_sp(opcode: u16, cpu: &mut Cpu) {
    let offset = (opcode as u32 & 0x7F) << 2;
    let s_bit = (opcode >> 7) & 1 == 1;
    
    let sp = cpu.get_register_mut(13);
    match s_bit {
        true => *sp = sp.wrapping_sub(offset),
        false => *sp = sp.wrapping_add(offset),
    }
}
fn push_pop(opcode: u16, cpu: &mut Cpu, memory: &mut Memory) {
    let mut rlist = opcode & 0xFF;
    let l_bit = (opcode >> 11) & 1 == 1;
    let r_bit = (opcode >> 8) & 1 == 1;

    let sp = cpu.get_register(13);
    match l_bit {
        true => { // load
            let mut base_address = sp;
            while rlist != 0 {
                let next_r = rlist.trailing_zeros();

                let reg = cpu.get_register_mut(next_r as u8);
                let change = memory.read_u32(base_address);
                *reg = change;
                
                base_address += 4;
                rlist &= !(1<<next_r);
            }
            if r_bit {
                let reg = cpu.get_register_mut(15);
                let change = memory.read_u32(base_address);
                *reg = change & !(1);
                base_address += 4;
                cpu.clear_pipeline = true;
            }
            let sp_mut = cpu.get_register_mut(13);
            *sp_mut = base_address;
        }
        false => {
            let total_increments = rlist.count_ones() + r_bit as u32;

            let mut base_address = sp - (total_increments * 4);
            let base_address_copy = base_address;
            while rlist != 0 {
                let next_r = rlist.trailing_zeros();
                let reg = cpu.get_register(next_r as u8);
                memory.write_u32(base_address, reg);
                base_address += 4;
                rlist &= !(1<<next_r);
            }
            if r_bit {
                let reg = cpu.get_register(14);
                memory.write_u32(base_address, reg);
            }

            let sp = cpu.get_register_mut(13);
            *sp = base_address_copy;
        }
    }
}
fn mem_multiple(opcode: u16, cpu: &mut Cpu, memory: &mut Memory) {
    let mut rlist = opcode & 0xFF;
    let started_empty = rlist == 0;

    let rb_index = (opcode >> 8) as u8 & 0b111;
    let rb = cpu.get_register(rb_index);

    let l_bit = (opcode >> 11) & 1 == 1;
    let mut curr_address = rb;
    let end_result = curr_address + (rlist.count_ones() * 4);

    match l_bit {
        true => { // load
            while rlist != 0 {
                let next_r = rlist.trailing_zeros();

                let reg = cpu.get_register_mut(next_r as u8);
                let change = memory.read_u32(curr_address);
                *reg = change;
                
                curr_address += 4;
                rlist &= !(1<<next_r);
            }
            if started_empty {
                let reg = cpu.get_register_mut(15);
                let change = memory.read_u32(curr_address);
                *reg = change;
                cpu.clear_pipeline = true;

                curr_address += 0x40;
            }
        }
        false => {
            let mut first_run = true;

            while rlist != 0 {
                let next_r = rlist.trailing_zeros();
                if !first_run && (next_r as u8 == rb_index) {
                    // need to calculate the end
                    let rb_mut = cpu.get_register_mut(rb_index);
                    *rb_mut = end_result;
                }

                let reg = cpu.get_register(next_r as u8);
                memory.write_u32(curr_address, reg);
                curr_address += 4;

                rlist &= !(1<<next_r);
                first_run = false;
            }
            if started_empty {
                let reg = cpu.get_register(15) + 2;
                memory.write_u32(curr_address, reg);

                curr_address += 0x40;
            }
        }
    }
    let rb_mut = cpu.get_register_mut(rb_index);
    *rb_mut = curr_address;
}
fn unconditional_branch(opcode: u16, cpu: &mut Cpu) {
    let mut offset = (opcode as u32 & 0x3FF) << 1;
    if (opcode >> 10) & 1 == 1 {
        offset |= 0xFFFFF800;
    }

    let pc = cpu.get_register_mut(15);
    *pc = pc.wrapping_add_signed(offset as i32);
    cpu.clear_pipeline = true;
}
fn conditional_branch(opcode: u16, cpu: &mut Cpu) {
    let condition = (opcode >> 8) & 0xF;
    if !check_condition(condition as u32, &cpu.cpsr) {
        return;
    }

    let pc = cpu.get_register_mut(15);
    let mut offset = (opcode & 0xFF) as u32;
    offset <<= 1;
    if (offset >> 8) & 1 == 1 {
        offset |= 0xFFFFFF00;
    }
    *pc = pc.wrapping_add_signed(offset as i32);
    cpu.clear_pipeline = true;
}
fn long_branch_link(opcode: u16, cpu: &mut Cpu) {
    let mut offset = opcode as u32 & 0x7FF;
    let h_bit = (opcode >> 11) & 1 == 1;

    match h_bit {
        false => {
            if (offset >> 10) & 1 == 1 {
                offset |= 0xFFFFF800;
            }
            offset <<= 12;

            let pc = cpu.get_register(15);
            let lr = cpu.get_register_mut(14);
            *lr = pc.wrapping_add(offset);
        },
        true => {
            offset <<= 1;
            let lr = cpu.get_register(14);
            let pc = cpu.get_register_mut(15);

            let temp = *pc - 2;
            *pc = lr.wrapping_add(offset);
            let lr = cpu.get_register_mut(14);
            *lr = temp | 1;
            cpu.clear_pipeline = true;
        },
    };
}

fn software_interrupt(cpu: &mut Cpu) {
    cpu.set_specific_spsr(cpu.cpsr, ProcessorMode::Supervisor);
    cpu.cpsr.mode = ProcessorMode::Supervisor;

    let pc = cpu.get_register(15);
    let lr = cpu.get_register_mut(14);
    *lr = pc - 2;

    let pc = cpu.get_register_mut(15);
    *pc = 0x08;
    cpu.clear_pipeline = true;
    cpu.cpsr.t = false;
}