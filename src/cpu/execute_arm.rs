use crate::memory::Memory;

use super::registers::{*, status_registers::*};
use super::decode::DecodedArm;
use super::get_shifted_value;

pub fn execute_arm(
    opcode: u32, 
    decoded_arm: DecodedArm,
    cpu_regs: &mut Cpu,
    status: &mut CpuStatus,
    memory: &mut Memory,
) {
    // first check if we even have to do it
    let condition = opcode >> 28;
    if !check_condition(condition, &status.cpsr) {
        return;
    }
    use DecodedArm::*;
    match decoded_arm {
        DataProcessing => data_processing(opcode, cpu_regs, status),
        Multiply => multiply(opcode, cpu_regs, status),
        MultiplyLong => multiply_long(opcode, cpu_regs, status),
        SingleDataSwap => single_swap(opcode, cpu_regs, status, memory),
        BranchExchange => branch_exchange(opcode, cpu_regs, status),
        HalfwordTransferReg => halfword_transfer(opcode, cpu_regs, status, memory),
        HalfwordTransferImm => halfword_transfer(opcode, cpu_regs, status, memory),
        SingleDataTransfer => data_transfer(opcode, cpu_regs, status, memory),
        Undefined => {},
        BlockDataTransfer => block_transfer(opcode, cpu_regs, status, memory),
        Branch => branch_link(opcode, cpu_regs, status),
        CoprocDataOperation => panic!("Coprocessor Data Operations arent handled for GBA"),
        CoprocDataTransfer => panic!("Coprocessor Data Transfers arent handled for GBA"),
        CoprocRegTransfer => panic!("Coprocessor Register Transfers arent handled for GBA"),
        Swi => software_interrupt(cpu_regs, status),
    }
}

fn branch_link(opcode: u32, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    // the bottom 24-bits
    let mut offset = (opcode & 0x00FFFFFF) as u32;
    offset <<= 2;
    if (opcode >> 23) & 1 == 1 {
        offset |= 0xFC000000;
    }

    // link
    if (opcode >> 24) & 1 == 1 {
        let prev_pc = cpu_regs.get_register(15, status.cpsr.mode);
        let link = cpu_regs.get_register_mut(14, status.cpsr.mode);
        // it is 2 instructions ahead, we only want it 1
        *link = (prev_pc & !(0b11)) - 4;
    }

    let pc = cpu_regs.get_register_mut(15, status.cpsr.mode);
    *pc = pc.wrapping_add_signed(offset as i32);
    cpu_regs.clear_pipeline = true;
}

fn branch_exchange(opcode: u32, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    assert!((opcode & 0xF) != 15, "BRANCH EXCHANGE is undefined if Rn == 15");

    let rn_index = opcode as u8 & 0xF;

    let rn = cpu_regs.get_register(rn_index, status.cpsr.mode);
    let pc = cpu_regs.get_register_mut(15, status.cpsr.mode);
    status.cpsr.t = (rn & 1) == 1;

    *pc = rn & 0xFFFFFFFE;
    cpu_regs.clear_pipeline = true;
}

fn data_processing(opcode: u32, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    let s_bit = (opcode >> 20) & 1 == 1;
    let operation = (opcode >> 21) & 0xF;

    // its a test one, but doesnt change cpsr, so psr transfer
    if operation >= 8 && operation <= 11 && !s_bit {
        psr_transfer(opcode, cpu_regs, status);
        return
    }
    
    // there are so many edge cases and weird procedures that its easier to 
    // have these predefined so they can be assigned whenever and exited
    let mut op2;
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
            op2_carry = (op2 >> 31) & 1 != 0;
        }
        // operand 2 is a register
        false => (op2, op2_carry) = get_shifted_value(cpu_regs, opcode, status),
    }

    let rn_index = (opcode >> 16) as u8 & 0xF;
    let rd_index = (opcode >> 12) as u8 & 0xF;

    let mut op1 = cpu_regs.get_register(rn_index, status.cpsr.mode);

    let mut undo = false;
    let (result, alu_carry) = match operation {
        0b0000 => (op1 & op2, op2_carry), // and
        0b0001 => (op1 ^ op2, op2_carry), // eor
        0b0010 => {
            op2 = (!op2).wrapping_add(1);
            op1.overflowing_add(op2)
        }, // sub
        0b0011 => {
            op1 = (!op1).wrapping_add(1);
            op2.overflowing_add(op1)
        }, // rsb
        0b0100 => op1.overflowing_add(op2), // add
        0b0101 => {
            let (inter_res, inter_of) = op1.overflowing_add(op2);
            let (end_res, end_of) = inter_res.overflowing_add(status.cpsr.c as u32);
            (end_res, inter_of | end_of)
        }, // adc
        0b0110 => {
            op2 = (!op2).wrapping_add(1).wrapping_add(status.cpsr.c as u32);
            let (first_result, first_carry) = op1.overflowing_add(op2);
            let (second_result, second_carry) = first_result.overflowing_add(std::u32::MAX - 1);
            (second_result, first_carry | second_carry)
        }, // sbc,
        0b0111 => {
            op1 = (!op1).wrapping_add(1).wrapping_add(status.cpsr.c as u32);
            let (first_result, first_carry) = op2.overflowing_add(op1);
            let (second_result, second_carry) = first_result.overflowing_add(std::u32::MAX - 1);
            (second_result, first_carry | second_carry)
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
            op2 = (!op2).wrapping_add(1);
            op1.overflowing_add(op2)
        }, // cmp
        0b1011 => {
            undo = true; 
            op1.overflowing_add(op2)
        }, // cmn
        0b1100 => {
            (op1 | op2, op2_carry)
        }, // orr
        0b1101 => (op2, op2_carry), // mov
        0b1110 => (op1 & !op2, op2_carry), // bic
        0b1111 => (!op2, op2_carry), // mvn
        _ => unreachable!()
    };
    // checking if we are returning from a SWI
    if let ProcessorMode::Supervisor = status.cpsr.mode {
        if operation == 0b1101 && i_bit && opcode & 0xF == 14 {
            status.cpsr.mode = ProcessorMode::User;
        }
    }

    if s_bit {
        status.cpsr.z = result == 0;
        status.cpsr.n = (result >> 31) & 1 == 1;
        status.cpsr.c = alu_carry;
        // the v flag is only affected when the instruction is arithmetic
        if ![0b0000, 0b0001, 0b1000, 0b1001, 0b1100, 0b1101, 0b1110, 0b1111].contains(&operation) {
            status.cpsr.v = ((op1 ^ result) & (op2 ^ result)) >> 31 & 1 != 0;
        }

        if undo {
            return;
        }
    }

    let dst = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *dst = result;
}

fn psr_transfer(opcode: u32, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    // matching between the middle 10 bits
    match (opcode >> 12) & 0b11_1111_1111 {
        0b1010011111 => { // transfer register contents to PSR
            // MSR
            let cpsr_bit = (opcode >> 22) & 1 == 1;
            
            let rm_index = opcode as u8 & 0xF;
            let rm = cpu_regs.get_register(rm_index, status.cpsr.mode);

            if let ProcessorMode::User = status.cpsr.mode {
                if !cpsr_bit {
                    status.set_flags_cpsr(convert_u32_cpsr(rm));
                    return;
                }
            }

            let dst_cpsr;
            match cpsr_bit {
                true => dst_cpsr = status.get_spsr_mut(),
                false => dst_cpsr = &mut status.cpsr,
            }
            *dst_cpsr = convert_u32_cpsr(rm);
        }
        0b1010001111 => {
            let imm_op = (opcode >> 25) & 1 == 0;
            let operand;
            match imm_op {
                false => {
                    let reg_index = opcode as u8 & 0b1111;
                    assert!(reg_index != 15, "source register cannot be register 15");
                    operand = cpu_regs.get_register(reg_index, status.cpsr.mode);
                }
                true => {
                    let imm = opcode & 0xFF;
                    let shift_amount = (opcode >> 8) & 0xF;

                    // this better be done the same way as the one before
                    operand = imm.rotate_right(shift_amount * 2);
                }
            }
            
            let new_psr = convert_u32_cpsr(operand);
            let psr_bit = (opcode >> 25) & 1 == 1;
            match psr_bit {
                true => status.set_flags_spsr(new_psr),
                false => status.set_flags_cpsr(new_psr),
            }
        }
        _ => { // im just assuming all the other ones are MRS
            // just a quick test to make sure it is MRS, not a full check, just gives the program more confidence
            assert!((opcode >> 16) & 0b111111 == 0b001111, "not a MSR instruction");

            let dst_index = (opcode >> 12) & 0xF;
            assert!(dst_index != 15, "destination register cannot be register 15");
            let dest_reg = cpu_regs.get_register_mut(dst_index as u8, status.cpsr.mode);

            let cpsr = match (opcode >> 22) & 1 != 0 {
                true => convert_cpsr_u32(status.get_spsr()),
                false => convert_cpsr_u32(&status.cpsr),
            };

            *dest_reg = cpsr;
        }
    }
}

fn multiply(opcode: u32, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    let rn_index = (opcode >> 12) as u8 & 0xF;
    let rd_index = (opcode >> 16) as u8 & 0xF;
    let rs_index = (opcode >> 8)  as u8 & 0xF;
    let rm_index = opcode         as u8 & 0xF;

    assert_ne!(rd_index, rm_index, "destination and source register cannot be the same");
    if rn_index == 15 || rd_index == 15 || rs_index == 15 || rm_index == 15 {
        panic!("register 15 cannot be used as an operand or destination");
    }

    let rn = cpu_regs.get_register(rn_index, status.cpsr.mode);
    let rs = cpu_regs.get_register(rs_index, status.cpsr.mode);
    let rm = cpu_regs.get_register(rm_index, status.cpsr.mode);
    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);

    *rd = rm.wrapping_mul(rs);
    if (opcode >> 21) & 1 == 1 {
        *rd += rn;
    }

    if (opcode >> 20) & 1 == 1 {
        status.cpsr.z = *rd == 0;
        status.cpsr.n = (*rd >> 31) != 0;
    }
}

fn multiply_long(opcode: u32, cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    let rm_index = opcode        as u8 & 0xF;
    let rs_index = (opcode >> 8) as u8 & 0xF;
    let rdl_index = (opcode >> 12) as u8 & 0xF;
    let rdh_index = (opcode >> 16) as u8 & 0xF;

    // operand restrictions as usual
    if rm_index == 15 || rs_index == 15 || rdh_index == 15 || rdl_index == 15 {
        panic!("register 15 cannot be used as an operand or destination");
    }
    if rm_index == rdh_index || rdh_index == rdl_index || rdl_index == rm_index {
        panic!("rdh, rdl, and rm must all be different from eachother");
    }

    let rm = cpu_regs.get_register(rm_index, status.cpsr.mode);
    let rs = cpu_regs.get_register(rs_index, status.cpsr.mode);

    let mut result = match (opcode >> 22) & 1 != 0 {
        true => ((rm as i64) * (rs as i64)) as u64,
        false => (rm as u64) * (rs as u64)
    };

    if (opcode >> 21) & 1 != 0 {
        let low_acc = cpu_regs.get_register(rdl_index, status.cpsr.mode) as u64;
        let hi_acc = cpu_regs.get_register(rdh_index, status.cpsr.mode) as u64;

        result += (hi_acc << 32) | low_acc;
    }

    let rdl = cpu_regs.get_register_mut(rdl_index, status.cpsr.mode);
    *rdl = result as u32;

    let rdh = cpu_regs.get_register_mut(rdh_index, status.cpsr.mode);
    *rdh = (result >> 32) as u32;

    if (opcode >> 20 ) & 1 != 0 {
        status.cpsr.z = result == 0;
        status.cpsr.n = (result >> 63) & 1 != 0;
    }
}

/// this instruction shouldnt change any of the CPSR flags
fn software_interrupt(cpu_regs: &mut Cpu, status: &mut CpuStatus) {
    // spsr_svc gets the old cpsr transferred into it
    status.set_specific_spsr(status.cpsr, ProcessorMode::Supervisor);

    status.cpsr.mode = ProcessorMode::Supervisor;
    let pc = cpu_regs.get_register(15, status.cpsr.mode);
    let save_pc = cpu_regs.get_register_mut(14, status.cpsr.mode);
    *save_pc = pc - 8;

    let change_pc = cpu_regs.get_register_mut(15, status.cpsr.mode);
    *change_pc = 0x08;
    cpu_regs.clear_pipeline = true;
}

fn data_transfer(opcode: u32, cpu_regs: &mut Cpu, status: &CpuStatus, memory: &mut Memory) {
    let rd_index = (opcode >> 12) as u8 & 0xF;
    let rn_index = (opcode >> 16) as u8 & 0xF;

    let offset;
    if (opcode >> 25) & 1 != 0 {
        offset = get_shifted_value(cpu_regs, opcode, status).0;
    } else {
        offset = opcode & 0b0111_1111_1111;
    }

    let mut address = cpu_regs.get_register(rn_index, status.cpsr.mode);

    let pre_index = (opcode >> 24) & 1 == 1;
    let add_offset = (opcode >> 23) & 1 == 1;
    if pre_index {
        match add_offset {
            true => address += offset,
            false => address -= offset,
        }
    }

    let load_store_bit = (opcode >> 20) & 1 == 1;
    let data_size_bit = (opcode >> 22) & 1 == 1;

    match load_store_bit {
        true => {
            let data = match data_size_bit {
                true => memory.read_u8(address) as u32,
                false => memory.read_u32(address),
            };
            let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
            *rd = data;
        },
        false => {
            let rd = cpu_regs.get_register(rd_index, status.cpsr.mode);
            match data_size_bit {
                true => memory.write_u8(address, rd as u8),
                false => memory.write_u32(address, rd),
            }
        }
    }

    // have to post-update
    if !pre_index {
        match add_offset {
            true => address += offset,
            false => address -= offset,
        }
    }

    let write_back = (opcode >> 21) & 1 == 1;
    if write_back || !pre_index {
        let rn = cpu_regs.get_register_mut(rn_index, status.cpsr.mode);
        // this address has changed and is being written back
        *rn = address;
    }
}

/// this function handles both the immediate and register offsets
/// Both pretty much have identical implementation besides for data acquisition
fn halfword_transfer(opcode: u32, cpu_regs: &mut Cpu, status: &CpuStatus, memory: &mut Memory) {
    let rd_index = (opcode >> 12) as u8 & 0xF;
    let rn_index = (opcode >> 16) as u8 & 0xF;

    let mut address = cpu_regs.get_register(rn_index, status.cpsr.mode);

    let offset;
    let offset_type = (opcode >> 22) & 1 == 1;
    match offset_type {
        true => {
            offset = (opcode & 0xF) | (opcode >> 4) & 0xF0;
        }
        false => {
            let rm_index = opcode as u8 & 0xF;
            offset = cpu_regs.get_register(rm_index, status.cpsr.mode);
        }
    }

    let pre_index = (opcode >> 24) & 1 == 1;
    let add_offset = (opcode >> 23) & 1 == 1;
    if pre_index {
        match add_offset {
            true => address += offset,
            false => address -= offset,
        }
    }

    // the reading of the memory
    let sh = (opcode >> 5) as u8 & 0b11;
    let load_bit = (opcode >> 20) & 1 == 1;
    match sh {
        0b00 => unreachable!("unreachable due to decoding protocol"),
        0b01 => {
            //Unsigned halfwords
            match load_bit {
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
        0b10 => {
            // Signed byte
            // loads shouldn't happen when its signed
            assert!(load_bit, "L bit should not be set low");

            let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
            let mut raw_reading = memory.read_u8(address) as u32;
            if (raw_reading >> 7) & 1 == 1 {
                raw_reading |= 0xFFFFFF00;
            }
            *rd = raw_reading;
        }
        0b11 => {
            // signed halfword
            assert!(load_bit, "L bit should not be set low");

            let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
            let mut raw_reading = memory.read_u16(address) as u32;
            if (raw_reading >> 15) & 1 == 1 {
                raw_reading |= 0xFFFF0000;
            }
            *rd = raw_reading;
        }
        _ => unreachable!()
    }

    if !pre_index {
        match add_offset {
            true => address += offset,
            false => address -= offset,
        }
    }
    let write_back = (opcode >> 21) & 1 == 1;
    if write_back || !pre_index {
        let rn = cpu_regs.get_register_mut(rn_index, status.cpsr.mode);
        *rn = address;
    }
}

fn block_transfer(opcode: u32, cpu_regs: &mut Cpu, status: &mut CpuStatus, memory: &mut Memory) {    
    let mut rlist = opcode & 0xFFFF;
    let rn_index = (opcode >> 16) & 0b1111;
    assert!(rn_index != 15, "r15 cannot be used as the base register");

    let l_bit = (opcode >> 20) & 1 == 1;
    let w_bit = (opcode >> 21) & 1 == 1;
    let s_bit = (opcode >> 22) & 1 == 1;
    let u_bit = (opcode >> 23) & 1 == 1;
    let p_bit = (opcode >> 24) & 1 == 1;

    let rn = cpu_regs.get_register(rn_index as u8, status.cpsr.mode);
    let mut base_address;

    let used_mode;
    match s_bit {
        true => used_mode = ProcessorMode::User,
        false => used_mode = status.cpsr.mode,
    }

    match u_bit {
        true => base_address = rn,
        false => base_address = rn - (rlist.count_ones() * 4),
    }
    let original_base = base_address;

    match l_bit {
        true => {
            while rlist != 0 {
                if p_bit == u_bit  {
                    base_address += 4;
                }
    
                let next_r = rlist.trailing_zeros();
                let rb = cpu_regs.get_register_mut(next_r as u8, used_mode);
                *rb = memory.read_u32(base_address);
    
                if p_bit != u_bit {
                    base_address += 4;
                }
                rlist &= !(1<<next_r);
            }
        }
        false => {
            while rlist != 0 {
                if p_bit == u_bit  {
                    base_address += 4;
                }
    
                let next_r = rlist.trailing_zeros();
                let mut rb = cpu_regs.get_register(next_r as u8, used_mode);
                if next_r == 15 {
                    rb += 4;
                }
                memory.write_u32(base_address, rb);
    
                if p_bit != u_bit {
                    base_address += 4;
                }
                rlist &= !(1<<next_r);
            }
        }
    }
    // was rn in the transfer?
    if l_bit && (opcode >> rn_index) & 1 == 1 {
        return;
    }

    if w_bit {
        let rn_mut = cpu_regs.get_register_mut(rn_index as u8, status.cpsr.mode);
        match u_bit {
            true => *rn_mut = base_address,
            false => *rn_mut = original_base,
        }
    }
}

fn single_swap(opcode: u32, cpu_regs: &mut Cpu, status: &CpuStatus, memory: &mut Memory) {
    // for now just have them happen at the same time
    let rn_index = (opcode >> 16) as u8 & 0xF;
    let rm_index = opcode as u8 & 0xF;
    let rd_index = (opcode >> 12) as u8 & 0xF;

    assert!(rn_index != 15, "r15 cannot be used in SWP");
    assert!(rm_index != 15, "r15 cannot be used in SWP");
    assert!(rd_index != 15, "r15 cannot be used in SWP");

    let address = cpu_regs.get_register(rn_index, status.cpsr.mode);

    let quantity_bit = (opcode >> 22) & 1 == 1;

    let data;
    match quantity_bit {
        true => data = memory.read_u8(address) as u32,
        false => data = memory.read_u32(address),
    }
    
    let rm = cpu_regs.get_register(rm_index, status.cpsr.mode);
    match quantity_bit {
        true => memory.write_u8(address, rm as u8),
        false => memory.write_u32(address, rm),
    }

    let rd = cpu_regs.get_register_mut(rd_index, status.cpsr.mode);
    *rd = data;
}