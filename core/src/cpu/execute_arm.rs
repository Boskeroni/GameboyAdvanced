use crate::cpu::decode::decode_arm;
use crate::mem::bus::CpuInterface;

use super::*;
use super::decode::DecodedArm;
use super::get_shifted_value;

pub fn execute_arm<M: CpuInterface>(
    opcode: u32, 
    cpu: &mut Cpu,
    memory: &mut M,
) {
    // println!("{:?} {:X}", assemblify::to_arm_assembly(opcode), opcode);

    // first check if we even have to do it
    let condition = opcode >> 28;
    if !check_condition(condition, &cpu.cpsr) {
        return;
    }

    use DecodedArm::*;
    let instruction = decode_arm(opcode);
    match instruction {
        DataProcessing => data_processing(opcode, cpu),
        Multiply => multiply(opcode, cpu),
        MultiplyLong => multiply_long(opcode, cpu),
        SingleDataSwap => single_swap(opcode, cpu, memory),
        BranchExchange => branch_exchange(opcode, cpu),
        HalfwordTransferReg => halfword_transfer(opcode, cpu, memory),
        HalfwordTransferImm => halfword_transfer(opcode, cpu, memory),
        SingleDataTransfer => data_transfer(opcode, cpu, memory),
        Undefined => {},
        BlockDataTransfer => block_transfer(opcode, cpu, memory),
        Branch => branch_link(opcode, cpu),
        CoprocDataOperation => panic!("Coprocessor Data Operations arent handled for GBA"),
        CoprocDataTransfer => panic!("Coprocessor Data Transfers arent handled for GBA"),
        CoprocRegTransfer => panic!("Coprocessor Register Transfers arent handled for GBA"),
        Swi => software_interrupt(cpu),
    }
}

fn branch_link(opcode: u32, cpu: &mut Cpu) {
    let l_bit = (opcode >> 24) & 1 == 1;

    // the bottom 24-bits
    let mut offset = (opcode & 0x00FFFFFF) as u32;
    offset <<= 2;
    if (opcode >> 23) & 1 == 1 {
        offset |= 0xFC000000;
    }

    // link
    if l_bit {
        let prev_pc = cpu.get_register(15);
        let link = cpu.get_register_mut(14);
        // it is 2 instructions ahead, we only want it 1
        *link = (prev_pc & !(0b11)) - 4;
    }

    let pc = cpu.get_register_mut(15);
    *pc = pc.wrapping_add(offset);
    cpu.clear_pipeline();
}
fn branch_exchange(opcode: u32, cpu: &mut Cpu) {
    let rn_index = opcode as u8 & 0xF;

    let rn = cpu.get_register(rn_index);
    let pc = cpu.get_register_mut(15);

    *pc = rn;
    match rn & 1 == 1 {
        true => {
            *pc &= !(0x1);
            cpu.cpsr.t = true;
        }
        false => {
            *pc &= !(0x1);
            cpu.cpsr.t = false; // not fully needed but safe
        },
    }
    cpu.clear_pipeline();
}
fn data_processing(opcode: u32, cpu: &mut Cpu) {
    let s_bit = (opcode >> 20) & 1 == 1;
    let operation = (opcode >> 21) & 0xF;

    // its a test one, but doesnt change cpsr, so psr transfer
    if operation >= 8 && operation <= 11 && !s_bit {
        psr_transfer(opcode, cpu);
        return
    }
    
    // there are so many edge cases and weird procedures that its easier to 
    // have these predefined so they can be assigned whenever and exited
    let op2;
    let op2_carry;
    let i_bit = (opcode >> 25) & 1 == 1;
    match i_bit {
        // operand 2 is an immediate value
        true => {
            let imm = opcode & 0xFF;
            let shift_amount = (opcode >> 8) & 0xF;
            op2 = imm.rotate_right(shift_amount * 2);

            // i am just assuming that the carry bit functions in the same way
            // that the ROR carry bit works
            match shift_amount {
                0 => op2_carry = cpu.cpsr.c,
                _ => op2_carry = (op2 >> 31) & 1 == 1,
            }
        }
        // operand 2 is a register
        false => (op2, op2_carry) = get_shifted_value(cpu, opcode),
    }

    let rn_index = (opcode >> 16) as u8 & 0xF;
    let rd_index = (opcode >> 12) as u8 & 0xF;

    let op1;
    if rn_index == 15 && !i_bit && (opcode >> 4) & 1 == 1 {
        op1 = cpu.get_register(15) + 4;
    } else {
        op1 = cpu.get_register(rn_index);
    }

    let mut undo = false;
    let v_backup = cpu.cpsr.v;
    let (result, alu_carry) = match operation {
        0b0000 => (op1 & op2, op2_carry), // and
        0b0001 => {
            (op1 ^ op2, op2_carry)
        }, // eor
        0b0010 => {
            let (result, _, _, c, v) = add_with_carry(op1, !op2, true);
            cpu.cpsr.v = v;

            (result, c)
        }, // sub
        0b0011 => {
            let (result, _, _, c, v) = add_with_carry(!op1, op2, true);
            cpu.cpsr.v = v;

            (result, c)
        }, // rsb
        0b0100 => {
            let (result, _, _, c, v) = add_with_carry(op1, op2, false);
            cpu.cpsr.v = v;

            (result, c)
        }, // add
        0b0101 => {
            let (result, _, _, c, v) = add_with_carry(op1, op2, cpu.cpsr.c);
            cpu.cpsr.v = v;

            (result, c)
        }, // adc
        0b0110 => {
            let (result, _, _, c, v) = add_with_carry(op1, !op2, cpu.cpsr.c);
            cpu.cpsr.v = v;

            (result, c)
        }, // sbc,
        0b0111 => {
            let (result, _, _, c, v) = add_with_carry(!op1, op2, cpu.cpsr.c);
            cpu.cpsr.v = v;

            (result, c)
        }, // rsc
        0b1000 => {
            undo = true; 
            (op1 & op2, op2_carry)
        }, // tst
        0b1001 => {
            undo = true; 
            (op1 ^ op2, op2_carry)
        }, // teq
        0b1010 => {
            undo = true;
            let (result, _, _, c, v) = add_with_carry(op1, !op2, true);
            cpu.cpsr.v = v;

            (result, c)
        }, // cmp
        0b1011 => {
            undo = true; 
            let (result, _, _, c, v) = add_with_carry(op1, op2, false);
            cpu.cpsr.v = v;

            (result, c)
        }, // cmn
        0b1100 => (op1 | op2, op2_carry), // orr
        0b1101 => (op2, op2_carry), // mov
        0b1110 => (op1 & !op2, op2_carry), // bic
        0b1111 => (!op2, op2_carry), // mvn
        _ => unreachable!()
    };
    if !s_bit {
        cpu.cpsr.v = v_backup;
    }

    if s_bit {
        cpu.cpsr.z = result == 0;
        cpu.cpsr.n = (result >> 31) & 1 == 1;
        cpu.cpsr.c = alu_carry;      
    }

    if rd_index == 15 && s_bit {
        let pc = cpu.get_register_mut(rd_index);
        if !undo {
            *pc = result;
            cpu.clear_pipeline();
        }

        cpu.cpsr = *cpu.get_spsr();
        return;
    }
    if undo {
        return;
    }

    if rd_index == 15 {
        cpu.clear_pipeline();
    }
    let dst = cpu.get_register_mut(rd_index);
    *dst = result;
}
fn psr_transfer(opcode: u32, cpu: &mut Cpu) {
    let psr_bit = (opcode >> 22) & 1 == 1;
    let op = (opcode >> 21) & 1 == 1;

    match op {
        true => {
            // MSR
            let i_bit = (opcode >> 25) & 1 == 1;
            let f_bit = (opcode >> 19) & 1 == 1;
            let c_bit = (opcode >> 16) & 1 == 1;

            let operand = match i_bit {
                true => {
                    let imm = opcode & 0xFF;
                    let rotate = (opcode >> 8) & 0xF;
                    imm.rotate_right(rotate * 2)
                }
                false => {
                    let rm_index = opcode as u8 & 0xF;
                    cpu.get_register(rm_index)
                }
            };
            let starting_cpsr = cpu.cpsr.clone();
            let psr = match psr_bit {
                true => {
                    if let ProcessorMode::User|ProcessorMode::System = cpu.cpsr.mode {
                        return;
                    }
                    cpu.get_spsr_mut()
                }
                false => &mut cpu.cpsr,
            };

            if f_bit {
                psr.set_flags(operand);
            }
            if c_bit {
                psr.set_control(operand);
            }
            if starting_cpsr.t != cpu.cpsr.t {
                cpu.clear_pipeline();
            }
        }
        false => {
            // MRS
            let psr = match psr_bit {
                true => cpu.get_spsr(),
                false => &cpu.cpsr,
            };

            let result = convert_psr_u32(psr);
            let rd_index = (opcode >> 12) as u8 & 0xF;
            let rd = cpu.get_register_mut(rd_index);
            *rd = result;

            if rd_index == 15 {
                // cpu.clear_pipeline();
            }
        }
    }
}
fn multiply(opcode: u32, cpu: &mut Cpu) {
    let rn_index = (opcode >> 12) as u8 & 0xF;
    let rd_index = (opcode >> 16) as u8 & 0xF;
    let rs_index = (opcode >> 8)  as u8 & 0xF;
    let rm_index = opcode         as u8 & 0xF;

    let rn = cpu.get_register(rn_index);
    let rs = cpu.get_register(rs_index);
    let rm = cpu.get_register(rm_index);

    let mut result = rm.wrapping_mul(rs);
    let acc_bit = (opcode >> 21) & 1 == 1;
    if acc_bit {
        result = result.wrapping_add(rn);
    }
    if rd_index != 15 {
        let rd = cpu.get_register_mut(rd_index);
        *rd = result;
    }

    if (opcode >> 20) & 1 == 1 {
        cpu.cpsr.z = result == 0;
        cpu.cpsr.n = (result >> 31) == 1;
        cpu.cpsr.c = false;
    }

    if rd_index == 15 {
        cpu.clear_pipeline();
    }
}
fn multiply_long(opcode: u32, cpu: &mut Cpu) {
    let rm_index = opcode          as u8 & 0xF;
    let rs_index = (opcode >> 8)   as u8 & 0xF;
    let rdl_index = (opcode >> 12) as u8 & 0xF;
    let rdh_index = (opcode >> 16) as u8 & 0xF;

    let rm = cpu.get_register(rm_index);
    let rs = cpu.get_register(rs_index);

    let u_bit = (opcode >> 22) & 1 == 1;
    
    let mut result = match u_bit {
        true => {
            let (op1, op2);
            match (rm >> 31) & 1 == 1 {
                true => op1 = 0xFFFFFFFF00000000 | rm as u64,
                false => op1 = rm as u64,
            }
            match (rs >> 31) & 1 == 1 {
                true => op2 = 0xFFFFFFFF00000000 | rs as u64,
                false => op2 = rs as u64,
            }
            op1.wrapping_mul(op2)
        },
        false => (rm as u64).wrapping_mul(rs as u64),
    };

    let a_bit = (opcode >> 21) & 1 == 1;
    if a_bit {
        let low_acc = cpu.get_register(rdl_index) as u64;
        let hi_acc = cpu.get_register(rdh_index) as u64;

        result = result.wrapping_add((hi_acc << 32) | low_acc);
    }

    let rdl = cpu.get_register_mut(rdl_index);
    *rdl = result as u32;
    let rdh = cpu.get_register_mut(rdh_index);
    *rdh = (result >> 32) as u32;

    let s_bit = (opcode >> 20) & 1 == 1;
    if s_bit {
        cpu.cpsr.c = false;
        cpu.cpsr.z = result == 0;
        cpu.cpsr.n = (result >> 63) & 1 == 1;
    }
    if (rdh_index == 15) | (rdl_index == 15) {
        cpu.clear_pipeline();
    }
}
/// this instruction shouldnt change any of the CPSR flags
fn software_interrupt(cpu: &mut Cpu) {
    // spsr_svc gets the old cpsr transferred into it
    cpu.set_specific_spsr(cpu.cpsr, ProcessorMode::Supervisor);

    cpu.cpsr.mode = ProcessorMode::Supervisor;
    let pc = cpu.get_register(15);
    let save_pc = cpu.get_register_mut(14);
    *save_pc = pc - 4;

    let change_pc = cpu.get_register_mut(15);
    *change_pc = 0x08;
    cpu.cpsr.i = true;
    cpu.clear_pipeline();
}
fn data_transfer<M: CpuInterface>(opcode: u32, cpu: &mut Cpu, memory: &mut M) {
    let rd_index = (opcode >> 12) as u8 & 0xF;
    let rn_index = (opcode >> 16) as u8 & 0xF;

    let offset;
    if (opcode >> 25) & 1 == 1 {        
        offset = get_shifted_value(cpu, opcode).0;
    } else {
        offset = opcode & 0xFFF;
    }

    let mut address = cpu.get_register(rn_index);

    let pre_index = (opcode >> 24) & 1 == 1;
    let add_offset = (opcode >> 23) & 1 == 1;
    if pre_index {
        match add_offset {
            true => address = address.wrapping_add(offset),
            false => address = address.wrapping_sub(offset),
        }
    }

    let l_bit = (opcode >> 20) & 1 == 1;
    let b_bit = (opcode >> 22) & 1 == 1;
    match l_bit {
        true => {
            let data = match b_bit {
                true => memory.read_u8(address) as u32,
                false => memory.read_u32_rotated(address)
            };
            let rd = cpu.get_register_mut(rd_index);
            *rd = data;

            if rd_index == 15 {
                cpu.clear_pipeline();
            }
            if rn_index == rd_index {
                return;
            }
        },
        false => {
            let rd;
            match rd_index {
                15 => rd = cpu.get_register(rd_index) + 4,
                _ => rd = cpu.get_register(rd_index),
            }
            match b_bit {
                true => memory.write_u8(address, rd as u8),
                false => memory.write_u32(address, rd),
            }
        }
    }

    // have to post-update
    if !pre_index {
        match add_offset {
            true => address = address.wrapping_add(offset),
            false => address = address.wrapping_sub(offset),
        }
    }

    let write_back = (opcode >> 21) & 1 == 1;
    if write_back || !pre_index {
        let rn = cpu.get_register_mut(rn_index);
        // this address has changed and is being written back
        *rn = address;
        if rn_index == 15 {
            *rn += 4;
        }
        if rn_index == 15 {
            cpu.clear_pipeline();
        }
    }
}
/// this function handles both the immediate and register offsets
/// Both pretty much have identical implementation besides for data acquisition
fn halfword_transfer<M: CpuInterface>(opcode: u32, cpu: &mut Cpu, memory: &mut M) {
    let p_bit = (opcode >> 24) & 1 == 1;
    let u_bit = (opcode >> 23) & 1 == 1;
    let i_bit = (opcode >> 22) & 1 == 1;
    let w_bit = (opcode >> 21) & 1 == 1;
    let l_bit = (opcode >> 20) & 1 == 1;
    let s_bit = (opcode >> 6)  & 1 == 1;
    let h_bit = (opcode >> 5)  & 1 == 1;

    let rd_index = (opcode >> 12) as u8 & 0xF;
    let rn_index = (opcode >> 16) as u8 & 0xF;

    let mut address = cpu.get_register(rn_index);
    let offset = match i_bit {
        true => (opcode & 0xF) | ((opcode >> 4) & 0xF0),
        false => cpu.get_register(opcode as u8 & 0xF),
    };

    // pre-indexed
    if p_bit {
        match u_bit {
            true => address = address.wrapping_add(offset),
            false => address =  address.wrapping_sub(offset),
        }
    }

    // need to clear pipeline
    if l_bit && rd_index == 15 {
        cpu.clear_pipeline();
    }

    // the processing part
    match l_bit {
        false => {
            let rd = match rd_index {
                15 => cpu.get_register(rd_index) + 4,
                _ => cpu.get_register(rd_index),
            };
            memory.write_u16(address, rd as u16);
        }
        true => {
            match (s_bit, h_bit) {
                (false, true) => {
                    let rd = cpu.get_register_mut(rd_index);
                    *rd = (memory.read_u16(address) as u32).rotate_right((address & 1) * 8);
                }
                (true, false) => {
                    let mut raw_reading = memory.read_u8(address) as u32;
                    if (raw_reading >> 7) & 1 == 1 {
                        raw_reading |= 0xFFFFFF00;
                    }

                    let rd = cpu.get_register_mut(rd_index);
                    *rd = raw_reading;
                }
                (true, true) => {
                    let mut raw_reading;
                    let is_aligned = address & 1 == 1;
                    match is_aligned {
                        true => {
                            raw_reading = (memory.read_u16(address) as u32) >> 8;
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
                _ => unreachable!(),
            }
            if rd_index == 15 {
                cpu.clear_pipeline();
            }
        }
    }
    if !p_bit {
        match u_bit {
            true => address = address.wrapping_add(offset),
            false => address = address.wrapping_sub(offset),
        }
    }
    if rd_index == rn_index && l_bit {
        return;
    }

    if w_bit || !p_bit {
        let rn = cpu.get_register_mut(rn_index);
        *rn = address;
        if rn_index == 15 {
            *rn += 4;
        }
        if rn_index == 15 {
            cpu.clear_pipeline();
        }
    }
}
fn block_transfer<M: CpuInterface>(opcode: u32, cpu: &mut Cpu, memory: &mut M) {
    let mut rlist = opcode & 0xFFFF;
    let r15_in_list = (rlist >> 15) & 1 == 1;
    let started_empty = rlist == 0;
    if started_empty {
        rlist |= 0x8000;
    }


    let l_bit = (opcode >> 20) & 1 == 1;
    let w_bit = (opcode >> 21) & 1 == 1;
    let s_bit = (opcode >> 22) & 1 == 1;
    let u_bit = (opcode >> 23) & 1 == 1;
    let p_bit = (opcode >> 24) & 1 == 1;

    let rn_index = (opcode >> 16) & 0xF;
    let rn = cpu.get_register(rn_index as u8);

    let used_mode = match (s_bit, r15_in_list, l_bit) {
        (true, true, false) => ProcessorMode::User, // STM with R15 in transfer and S bit set
        (true, false, _) => ProcessorMode::User, // R15 not in list and S bit set
        _ => cpu.cpsr.mode, // All others
    };
    let mut current_address;
    match started_empty {
        true => {
            current_address = match u_bit {
                true => rn,
                false => rn - 0x40,
            };
        }
        false => {
            current_address = match u_bit {
                true => rn,
                false => rn - (rlist.count_ones() * 4),
            };
        }
    }

    let starting_base = current_address;
    let ending_base = match u_bit {
        true => starting_base + (rlist.count_ones() * 4),
        false => current_address,
    };

    match l_bit {
        true => {
            while rlist != 0 {
                if p_bit == u_bit {
                current_address += 4;
                }

                let next_r = rlist.trailing_zeros();

                // LDM with r15 in transfer list and s bit set (mode changes)
                if next_r == 15 {
                    if s_bit {
                        cpu.cpsr = *cpu.get_spsr();
                    }
                    cpu.clear_pipeline();
                }

                let rb = cpu.get_register_mut_specific(next_r as u8, used_mode);
                *rb = memory.read_u32_unrotated(current_address);

                if p_bit != u_bit {
                    current_address += 4;
                }
                rlist &= !(1<<next_r);
            }
        }
        false => {
            let mut first_run = true;

            while rlist != 0 {
                // why do the docs not make a mention of this???
                if !first_run && (rlist >> rn_index) & 1 == 1 && w_bit {
                    let rn_mut = cpu.get_register_mut_specific(rn_index as u8, used_mode);
                    *rn_mut = ending_base;
                }

                if p_bit == u_bit {
                    current_address += 4;
                }

                let next_r = rlist.trailing_zeros();
                let rb = match next_r {
                    15 => cpu.get_register_specific(15, used_mode) + 4,
                    _ => cpu.get_register_specific(next_r as u8, used_mode),
                };

                memory.write_u32(current_address & !(0b11), rb);
                if p_bit != u_bit {
                    current_address += 4;
                }

                first_run = false;
                rlist &= !(1<<next_r);
            }
        }
    }

    // was rn in the transfer?
    if l_bit && (opcode >> rn_index) & 1 == 1 {
        return;
    }

    if w_bit {
        let rn_mut = cpu.get_register_mut(rn_index as u8);
        if started_empty {
            match u_bit {
                true => *rn_mut = starting_base + 0x40,
                false => *rn_mut = starting_base, // this one has already been accounted for
            }
            return;
        }

        match u_bit {
            true => *rn_mut = current_address,
            false => *rn_mut = starting_base,
        }
    }
}
fn single_swap<M: CpuInterface>(opcode: u32, cpu: &mut Cpu, memory: &mut M) {
    // for now just have them happen at the same time
    let rn_index = (opcode >> 16) as u8 & 0xF;
    let rm_index = opcode as u8 & 0xF;
    let rd_index = (opcode >> 12) as u8 & 0xF;

    let address = match rn_index {
        15 => cpu.get_register(rn_index) + 4,
        _ => cpu.get_register(rn_index),
    };
    let quantity_bit = (opcode >> 22) & 1 == 1;
    let rm = match rm_index {
        15 => cpu.get_register(rm_index) + 4,
        _ => cpu.get_register(rm_index),
    };

    let data;
    match quantity_bit {
        true => data = memory.read_u8(address) as u32,
        false => data = memory.read_u32_rotated(address),
    }
    
    let rd = cpu.get_register_mut(rd_index);
    *rd = data;
    if rd_index == 15 {
        cpu.clear_pipeline();
    }

    match quantity_bit {
        true => memory.write_u8(address, rm as u8),
        false => memory.write_u32(address, rm),
    }
}