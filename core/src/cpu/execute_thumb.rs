use crate::cpu::*;
use crate::cpu::decode::{decode_thumb, DecodedThumb};
use crate::mem::bus::CpuInterface;

use super::get_shifted_value;

pub fn execute_thumb<M: CpuInterface>(
    opcode: u16,
    cpu: &mut Cpu,
    memory: &mut M,
) {
    // println!("{:?}", assemblify::to_thumb_assembly(opcode));

    use DecodedThumb::*;
    let instruction = decode_thumb(opcode);
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
    let mut offset = match i_bit {
        true => value as u32,
        false => cpu.get_register(value as u8),
    };

    let op = (opcode >> 9) & 1 == 1;
    match op {
        true => offset = !offset,
        false => {}
    }

    // the op may be confusing but trust
    // since it must be 1 for sub and 0 for add
    let (result, n, z, c, v) = add_with_carry(rs, offset, op);
    cpu.cpsr.n = n;
    cpu.cpsr.z = z;
    cpu.cpsr.c = c;
    cpu.cpsr.v = v;

    let rd = cpu.get_register_mut(rd_index as u8);
    *rd = result;

}
fn alu_imm(opcode: u16, cpu: &mut Cpu) {
    let rd_index = (opcode >> 8) as u8 & 0x7;

    let mut rd = cpu.get_register(rd_index);
    let mut offset = (opcode as u32) & 0xFF;

    let op = (opcode >> 11) & 0x3;
    match op {
        0 => rd = 0, // mov is the same as rd = 0 + offset
        1|3 => offset = !offset, // subtraction
        _ => {}
    }
    let (result, n, z, c, v) = add_with_carry(rd, offset, op & 1 == 1);

    if op != 0 {
        cpu.cpsr.v = v;
        cpu.cpsr.c = c;
    }
    cpu.cpsr.z = z;
    cpu.cpsr.n = n;

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
        0x0 => (rd & rs, cpu.cpsr.c), // and
        0x1 => {
            (rd ^ rs, cpu.cpsr.c)
        }, // eor
        0x2 => {
            let sent_opcode = 
                (rs_index as u32 & 0xF) << 8 |
                (0b0001) << 4 |   
                rd_index as u32 & 0xF;
            get_shifted_value(cpu, sent_opcode)
        } // lsl
        0x3 => {
            let sent_opcode = 
                (rs_index as u32 & 0xF) << 8 |
                (0b0011) << 4 |   
                rd_index as u32 & 0xF;
            
            let temp = get_shifted_value(cpu, sent_opcode);
            temp
        } // lsr
        0x4 => {
            // convert the opcode
            let sent_opcode = 
                (rs_index as u32 & 0xF) << 8 |
                (0b0101) << 4 |   
                rd_index as u32 & 0xF;
            get_shifted_value(cpu, sent_opcode)
        } // asr
        0x5 => {
            let (result, _, _, c, v) = add_with_carry(rs, rd, cpu.cpsr.c);
            cpu.cpsr.v = v;
            (result, c)
        }, // adc
        0x6 => {
            let (result, _, _, c, v) = add_with_carry(rd, !rs, cpu.cpsr.c);
            cpu.cpsr.v = v;
            (result, c)
        }, // sbc,
        0x7 => {
            let sent_opcode = 
                (rs_index as u32 & 0xF) << 8 |
                (0b0111) << 4 |   
                rd_index as u32 & 0xF;
            get_shifted_value(cpu, sent_opcode)
        }, // ror
        0x8 => {
            undo = true; 
            (rd & rs, cpu.cpsr.c)
        }, // tst
        0x9 => {
            let (result, _, _, c, v) = add_with_carry(0, !rs, true);
            cpu.cpsr.v = v;
            (result, c)
        }, // neg
        0xA => {
            undo = true;
            let (result, _, _, c, v) = add_with_carry(rd, !rs, true);
            cpu.cpsr.v = v;

            (result, c)
        }, // cmp
        0xB => {
            undo = true; 
            let (result, _, _, c, v) = add_with_carry(rd, rs, false);
            cpu.cpsr.v = v;

            (result, c)
        }, // cmn
        0xC => {
            (rd | rs, cpu.cpsr.c)
        }, // orr
        0xD => {
            let (result, carry) = rs.overflowing_mul(rd);
            (result, !carry)
        }, // mul
        0xE => (rd & !rs, cpu.cpsr.c), // bic
        0xF => (!rs, cpu.cpsr.c), // mvn
        _ => unreachable!()
    };

    // all of the instructions that could change `cpu.cpsr.v` already have
    if op != 0xD {
        cpu.cpsr.c = alu_carry;
    }
    cpu.cpsr.z = result == 0;
    cpu.cpsr.n = (result >> 31) & 1 == 1;
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
    let rs = cpu.get_register(rs_index);
    let rd = cpu.get_register(rd_index);
 
    let result;
    match op {
        0b00 => result = rd.wrapping_add(rs),
        0b01 => {
            let (_, n, z, c, v) = add_with_carry(rd, !rs, true);

            cpu.cpsr.n = n;
            cpu.cpsr.z = z;
            cpu.cpsr.c = c;
            cpu.cpsr.v = v;
            return;
        },
        0b10 => result = rs,
        0b11 => {
            // assert!(!h1, "H1=1 for this instruction is undefined");

            let pc = cpu.get_register_mut(15);
            match rs & 1 == 1 {
                true => *pc = rs & !(0x1),
                false => {
                    // swapping to arm mode
                    *pc = rs & !(0x1);
                    cpu.cpsr.t = false;
                },
            }
            cpu.clear_pipeline();
            return;
        }
        _ => unreachable!(),
    }

    let rd = cpu.get_register_mut(rd_index);
    *rd = result;
    if rd_index == 15 {
        *rd &= !(0b1);
        cpu.clear_pipeline();
    }

}
fn pc_relative_load<M: CpuInterface>(opcode: u16, cpu: &mut Cpu, memory: &mut M) {
    let rd_index = (opcode >> 8) as u8 & 0b111;
    let imm = (opcode & 0xFF) << 2;

    let pc = cpu.get_register(15) & 0xFFFFFFFC;
    let address = pc.wrapping_add(imm as u32);
    let read = memory.read_u32_rotated(address);

    let rd = cpu.get_register_mut(rd_index);
    *rd = read;
}
fn mem_offset<M: CpuInterface>(opcode: u16, cpu: &mut Cpu, memory: &mut M, uses_imm: bool) {
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
                false => *rd = memory.read_u32_rotated(address),
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
fn mem_sign_extended<M: CpuInterface>(opcode: u16, cpu: &mut Cpu, memory: &mut M) {
    let ro_index = (opcode >> 6) as u8 & 0b111;
    let rb_index = (opcode >> 3) as u8 & 0b111;
    let rd_index = opcode as u8 & 0b111;

    let ro = cpu.get_register(ro_index);
    let rb = cpu.get_register(rb_index);

    let address = ro.wrapping_add(rb);
    let sh = (opcode >> 10) & 0b11;

    match sh {
        0b00 => { // STRH
            let rd = cpu.get_register(rd_index);
            memory.write_u16(address, rd as u16);
        }
        0b10 => { // LDRH
            let rd = cpu.get_register_mut(rd_index);
            *rd = (memory.read_u16(address) as u32).rotate_right((address & 0b1) * 8);
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
                    raw_reading = (memory.read_u16(address) >> 8) as u32;
                    if (raw_reading >> 7) & 1 == 1 {
                        raw_reading |= 0xFFFFFF00;
                    }
                },
                false => {
                    raw_reading = memory.read_u16(address) as u32;
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
fn mem_halfword<M: CpuInterface>(opcode: u16, cpu: &mut Cpu, memory: &mut M) {
    let rd_index = opcode as u8 & 0b111;
    let rb_index = (opcode >> 3) as u8 & 0b111;

    let imm = ((opcode >> 6) & 0b11111) << 1;
    let rb = cpu.get_register(rb_index);

    let address = rb + imm as u32;

    let l_bit = (opcode >> 11) & 1 == 1;
    match l_bit {
        true => {
            let rd = cpu.get_register_mut(rd_index);
            *rd = (memory.read_u16(address) as u32).rotate_right((address & 0b1) * 8);
        }
        false => {
            let rd = cpu.get_register(rd_index);
            memory.write_u16(address, rd as u16);
        }
    }
}
fn mem_sp_relative<M: CpuInterface>(opcode: u16, cpu: &mut Cpu, memory: &mut M) {
    let rd_index = (opcode >> 8) as u8 & 0b111;
    let imm = opcode & 0xFF;

    let sp = cpu.get_register(13);
    let address = sp + (imm << 2) as u32;

    let l_bit = (opcode >> 11) & 1 == 1;
    match l_bit {
        true => {
            let rd = cpu.get_register_mut(rd_index);
            *rd = memory.read_u32_rotated(address);
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
fn push_pop<M: CpuInterface>(opcode: u16, cpu: &mut Cpu, memory: &mut M) {
    let mut rlist = opcode & 0xFF;
    let l_bit = (opcode >> 11) & 1 == 1;
    let r_bit = (opcode >> 8) & 1 == 1;

    let rn = cpu.get_register(13);
    if rlist == 0 && !r_bit {
        if l_bit {
            let new_pc = memory.read_u32_unrotated(rn);
            let pc = cpu.get_register_mut(15);
            *pc = new_pc;
            cpu.clear_pipeline();
        }
        let sp_mut = cpu.get_register_mut(13);
        *sp_mut = match l_bit {
            true => rn + 0x40,
            false => rn - 0x40,
        };
        if !l_bit {
            let pc = cpu.get_register(15);
            memory.write_u32(rn - 0x40, pc + 2);
        }
        return;
    }

    let mut extra: u32 = 0;
    match l_bit {
        true => { 
            // pop increments
            while rlist != 0 {
                let next_r = rlist.trailing_zeros();

                let reg = cpu.get_register_mut(next_r as u8);
                let change = memory.read_u32_unrotated(rn + extra);
                *reg = change;
                
                extra += 4;
                rlist &= !(1<<next_r); // clear it for next time
            }
            if r_bit {
                let change = memory.read_u32_unrotated(rn + extra);

                let reg = cpu.get_register_mut(15);
                *reg = change & !(1);
                cpu.clear_pipeline();

                extra += 4;
            }
        }
        false => {
            // push
            let saved = (rlist.count_ones() + r_bit as u32) * 4;
            extra = saved;

            while rlist != 0 {
                let next_r = rlist.trailing_zeros();
                let reg = cpu.get_register(next_r as u8);
                memory.write_u32(rn - extra, reg);
                rlist &= !(1<<next_r);
                extra -= 4;
            }
            if r_bit {
                let reg = cpu.get_register(14);
                memory.write_u32(rn - extra, reg);
            }
            extra = saved;
        }
    }

    let sp = cpu.get_register_mut(13);
    *sp = match l_bit {
        true => *sp + extra,
        false => *sp - extra,
    };
}
fn mem_multiple<M: CpuInterface>(opcode: u16, cpu: &mut Cpu, memory: &mut M) {
    let mut rlist = opcode & 0xFF;
    let started_empty = rlist == 0;

    let rn_index = (opcode >> 8) as u8 & 0b111;
    let rn = cpu.get_register(rn_index);

    let l_bit = (opcode >> 11) & 1 == 1;
        if started_empty {
        if l_bit {
            let new_pc = memory.read_u32_unrotated(rn); // unrotate
            let pc = cpu.get_register_mut(15);
            *pc = new_pc;
            cpu.clear_pipeline();
        }
        let rn_mut = cpu.get_register_mut(rn_index);
        *rn_mut = rn + 0x40;
        if !l_bit {
            let pc = cpu.get_register(15);
            memory.write_u32(rn, pc + 2);
        }
        return;
    }

    let mut extra = 0;
    let end_result = rn + (rlist.count_ones() * 4);

    match l_bit {
        true => { // load
            while rlist != 0 {
                let next_r = rlist.trailing_zeros();

                let reg = cpu.get_register_mut(next_r as u8);
                let change = memory.read_u32_unrotated(rn + extra);
                *reg = change;
                
                extra += 4;
                rlist &= !(1<<next_r);
            }
        }
        false => {
            // store
            let mut first_run = true;

            while rlist != 0 {
                let next_r = rlist.trailing_zeros();
                if !first_run && (next_r as u8 == rn_index) {
                    // need to calculate the end
                    let rb_mut = cpu.get_register_mut(rn_index);
                    *rb_mut = end_result;
                }

                let reg = cpu.get_register(next_r as u8);
                memory.write_u32(rn + extra, reg);
                extra += 4;

                rlist &= !(1<<next_r);
                first_run = false;
            }
        }
    }
    if (opcode >> rn_index) & 1 == 1 && l_bit {
        return;
    }

    let rb_mut = cpu.get_register_mut(rn_index);
    *rb_mut = end_result;
}
fn unconditional_branch(opcode: u16, cpu: &mut Cpu) {
    let mut offset = (opcode as u32 & 0x3FF) << 1;
    if (opcode >> 10) & 1 == 1 {
        offset |= 0xFFFFF800;
    }

    let pc = cpu.get_register_mut(15);
    *pc = pc.wrapping_add_signed(offset as i32);
    cpu.clear_pipeline();
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
    cpu.clear_pipeline();
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
            *pc &= !1; // clear the last bit
            let lr = cpu.get_register_mut(14);
            *lr = temp | 1;
            cpu.clear_pipeline();
        },
    };
}

fn software_interrupt(cpu: &mut Cpu) {
    cpu.set_specific_spsr(cpu.cpsr, ProcessorMode::Supervisor);
    cpu.cpsr.mode = ProcessorMode::Supervisor;
    cpu.cpsr.i = true;

    let pc = cpu.get_register(15);
    let lr = cpu.get_register_mut(14);
    *lr = pc - 2;

    let pc = cpu.get_register_mut(15);
    *pc = 0x08;
    cpu.clear_pipeline();
    cpu.cpsr.t = false;
}