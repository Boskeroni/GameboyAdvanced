use crate::memory::Memory;

use super::registers::{*, status_registers::*};
use super::decode::DecodedArm;
use super::get_shifted_value;

pub fn execute_arm(
    opcode: u32, 
    decoded_arm: DecodedArm,
    cpu_regs: &mut Cpu,
    status: &mut Status,
    memory: &mut Memory,
) {
    // first check if we even have to do it
    let condition = opcode >> 28;
    if !check_arm_condition(condition as u8, &status.cpsr) {
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
        CoprocDataOperation => eprintln!("Coprocessor Data Operations arent handled for GBA"),
        CoprocDataTransfer => eprintln!("Coprocessor Data Transfers arent handled for GBA"),
        CoprocRegTransfer => eprintln!("Coprocessor Register Transfers arent handled for GBA"),
        Swi => software_interrupt(cpu_regs, status),
    }
}

fn branch_link(opcode: u32, cpu_regs: &mut Cpu, status: &mut Status) {
    // the bottom 24-bits
    let mut offset = (opcode & 0xFFFFFF) as u32; 
    offset <<= 2;
    if (offset >> 25) & 1 == 1 {
        offset |= 0b1111_1100_0000_0000_0000_0000_0000_0000;
    }

    // link
    if (opcode >> 24) & 1 == 1 {
        let prev_pc = cpu_regs.get_register(15, status.cpsr.mode);
        let link = cpu_regs.get_register_mut(14, status.cpsr.mode);
        *link = prev_pc;
    }

    let pc = cpu_regs.get_register_mut(15, status.cpsr.mode);
    *pc = pc.wrapping_add_signed(offset as i32);
}

fn branch_exchange(opcode: u32, cpu_regs: &mut Cpu, status: &mut Status) {
    assert!((opcode & 0xF) != 15, "BRANCH EXCHANGE is undefined if Rn == 15");

    let rn_index = opcode as u8 & 0xF;

    let rn = cpu_regs.get_register(rn_index, status.cpsr.mode);
    let pc = cpu_regs.get_register_mut(15, status.cpsr.mode);
    status.cpsr.t = (rn & 1) == 1;

    *pc = rn & 0xFFFFFFFE;
}

fn data_processing(opcode: u32, cpu_regs: &mut Cpu, status: &mut Status) {
    let change_cpsr = (opcode >> 20) & 1 == 1;
    let operation = (opcode >> 21) & 0b1111;

    // its a test one, but doesnt change cpsr, so psr transfer
    if operation >= 8 && operation <= 11 && !change_cpsr {
        psr_transfer(opcode, cpu_regs, status);
        return
    }
    
    // there are so many edge cases and weird procedures that its easier to 
    // have these predefined so they can be assigned whenever and exited
    let op2;
    let op2_carry;

    // means that the Operand2 is a Register with a shift applied
    if (opcode >> 25) & 1 == 0 {
        (op2, op2_carry) = get_shifted_value(cpu_regs, opcode & 0xFFF, status);
    } // means that its an Immediate value with a rotation applied to it 
    else {
        let imm = opcode & 0xFF;
        let shift_amount = (opcode >> 8) & 0xF;
        op2 = imm.rotate_right(shift_amount * 2);

        // i am just assuming that the carry bit functions in the same way
        // that the ROR carry bit works
        op2_carry = (op2 >> 31) & 1 != 0;
    }

    let op1_reg = (opcode >> 16) as u8 & 0xF;
    let src_index = (opcode >> 12) as u8 & 0xF;

    let op1 = cpu_regs.get_register(op1_reg, status.cpsr.mode);
    let src = cpu_regs.get_register_mut(src_index, status.cpsr.mode);

    let mut undo = false;
    let backup = *src;
    let (result, alu_carry) = match operation {
        0b0000 => (op1 & op2, false), // and
        0b0001 => (op1 ^ op2, false), // eor
        0b0010 => op1.overflowing_sub(op2), // sub
        0b0011 => op2.overflowing_sub(op1), // rsb
        0b0100 => op1.overflowing_add(op2), // add
        0b0101 => {
            let (inter_res, inter_of) = op1.overflowing_add(op2);
            let (end_res, end_of) = inter_res.overflowing_add(status.cpsr.c as u32);
            (end_res, inter_of | end_of)
        }, // adc
        0b0110 => {
            let (inter_res, inter_of) = op1.overflowing_sub(op2);
            let (end_res, end_of) = inter_res.overflowing_sub(!status.cpsr.c as u32);
            (end_res, inter_of | end_of)
        }, // sbc,
        0b0111 => {
            let (inter_res, inter_of) = op2.overflowing_sub(op1);
            let (end_res, end_of) = inter_res.overflowing_sub(!status.cpsr.c as u32);
            (end_res, inter_of | end_of)
        }, // rsc
        0b1000 => {
            undo = true; 
            (op1 & op2, false)
        }, // tst
        0b1001 => {
            undo = true; 
            (op1 ^ op2, false)
        }, // teq
        0b1010 => {
            undo = true;
            op1.overflowing_sub(op2)
        }, // cmp
        0b1011 => {
            undo = true; 
            op1.overflowing_add(op2)
        }, // cmn
        0b1100 => (op1 | op2, false), // orr
        0b1101 => (op2, false), // mov
        0b1110 => (op1 & !op2, false), // bic
        0b1111 => (!op2, false), // mvn
        _ => unreachable!()
    };
    *src = result;

    // both operations respect the S bit and R15 rule
    if !change_cpsr || op1_reg == 15 {
        return;
    }
    status.cpsr.z = *src == 0;
    status.cpsr.n = (*src >> 31) & 1 == 1;

    // the mathematical instructions
    if [0b0010, 0b0011, 0b0100, 0b0101, 0b0110, 0b0111, 0b1010, 0b1011].contains(&operation) {
        // it says that this should be ignored sometimes
        // but honestly i don't know when so i am doing it all the time
        status.cpsr.v = ((op1 ^ result) & (op2 ^ result)) >> 31 & 1 != 0; 
        status.cpsr.c = alu_carry;
    } else {
        status.cpsr.c = op2_carry;
    }

    if undo {
        *src = backup;
    }
}

fn psr_transfer(opcode: u32, cpu_regs: &mut Cpu, status: &mut Status) {
    // matching between the middle 10 bits
    match (opcode >> 12) & 0b11_1111_1111 {
        0b1010011111 => { // transfer register contents to PSR
            let used_cpsr = match (opcode >> 22) & 1 {
                0 => &status.cpsr,
                1 => status.get_spsr(),
                _ => unreachable!(),
            };

            let reg_index = opcode & 0b1111;
            assert!(reg_index != 15, "register 15 cannot be used as the source register");

            let reg = cpu_regs.get_register_mut(reg_index as u8, status.cpsr.mode);
            *reg = convert_cpsr_u32(used_cpsr);
        }
        0b1010001111 => {
            let operand_decider = (opcode >> 25) & 1 != 0;
            let operand = match operand_decider {
                false => {
                    let reg_index = opcode & 0b1111;
                    assert!(reg_index != 15, "source register cannot be register 15");
                    cpu_regs.get_register(reg_index as u8, status.cpsr.mode)
                }
                true => {
                    let imm = opcode & 0xFF;
                    let shift_amount = (opcode >> 8) & 0xF;

                    // this better be done the same way as the one before
                    imm.rotate_right(shift_amount * 2)
                }
            };
            
            let new_psr;
            match status.cpsr.mode {
                ProcessorMode::User => new_psr = convert_u32_cpsr_limited(operand),
                _ => new_psr = convert_u32_cpsr(operand),
            }

            match (opcode >> 25) & 1 != 0 {
                true => status.set_flags_spsr(new_psr),
                false => status.set_flags_cpsr(new_psr),
            }
        }
        _ => { // im just assuming all the other ones are MRS
            // just a quick test to make sure it is MRS, not a full check, just gives the program more confidence
            assert!((opcode >> 16) & 0b111111 == 0b001111, "not a MSR instruction");

            let dest_reg_index = (opcode >> 12) & 0xF;
            assert!(dest_reg_index != 15, "destination register cannot be register 15");
            let dest_reg = cpu_regs.get_register_mut(dest_reg_index as u8, status.cpsr.mode);

            let cpsr = match (opcode >> 22) & 1 != 0 {
                true => convert_cpsr_u32(status.get_spsr()),
                false => convert_cpsr_u32(&status.cpsr),
            };

            *dest_reg = cpsr;
        }
    }
}

fn multiply(opcode: u32, cpu_regs: &mut Cpu, status: &mut Status) {
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

fn multiply_long(opcode: u32, cpu_regs: &mut Cpu, status: &mut Status) {
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
fn software_interrupt(cpu_regs: &mut Cpu, status: &mut Status) {
    // spsr_svc gets the old cpsr transferred into it
    status.set_specific_spsr(status.cpsr, ProcessorMode::Supervisor);

    status.cpsr.mode = ProcessorMode::Supervisor;
    let pc = cpu_regs.get_register(15, status.cpsr.mode);
    let save_pc = cpu_regs.get_register_mut(14, status.cpsr.mode);
    *save_pc = pc;

    let change_pc = cpu_regs.get_register_mut(15, status.cpsr.mode);
    *change_pc = 0x08;
}

fn data_transfer(opcode: u32, cpu_regs: &mut Cpu, status: &Status, memory: &mut Memory) {
    let rd_index = (opcode >> 12) as u8 & 0xF;
    let rn_index = (opcode >> 16) as u8 & 0xF;

    let offset;
    if (opcode >> 25) & 1 != 0 {
        offset = get_shifted_value(cpu_regs, opcode & 0b0111_1111_1111, status).0;
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
            // load the value from memory
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
fn halfword_transfer(opcode: u32, cpu_regs: &mut Cpu, status: &Status, memory: &mut Memory) {
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
        0b00 => {
            unreachable!("i'm just going to say this is impossible, my decoding should deal with this")
        }
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

fn block_transfer(opcode: u32, cpu_regs: &mut Cpu, status: &mut Status, memory: &mut Memory) {
    let mut reg_list = opcode as u16;
    assert!(reg_list != 0, "reg list cannot be empty");

    let r15_in_list = (reg_list >> 15) & 1 == 1;

    let load_bit = (opcode >> 20) & 1 == 1;
    let pre_offset = (opcode >> 24) & 1 == 1;
    let add_offset = (opcode >> 23) & 1 == 1;

    let s_bit = (opcode >> 22) & 1 == 1;

    let mut saved_mode = status.cpsr.mode;
    let mut write_back_veto = false;
    if s_bit {
        write_back_veto = true;

        match (r15_in_list, load_bit) {
            (true, false) => {saved_mode = status.cpsr.mode ;status.cpsr.mode = ProcessorMode::User},
            (false, _) => {saved_mode = status.cpsr.mode; status.cpsr.mode = ProcessorMode::User},
            _ => write_back_veto = false,
        }
    }


    let rn_index = (opcode >> 16) as u8 & 0xF;
    assert!(rn_index != 15, "R15 cannot be the base register");

    let mut curr_address = cpu_regs.get_register(rn_index, status.cpsr.mode);
    match load_bit { 
        true => { // load from memory
            while reg_list.count_ones() != 0 {
                let next = 15 - reg_list.leading_zeros();

                if pre_offset {
                    match add_offset {
                        true => curr_address += 4,
                        false => curr_address -= 4,
                    }
                }

                let rnext = cpu_regs.get_register_mut(next as u8, status.cpsr.mode);
                let new_data = memory.read_u32(curr_address);

                *rnext = new_data;

                if !pre_offset {
                    match add_offset {
                        true => curr_address += 4,
                        false => curr_address -= 4,
                    }
                }

                // clearing the bit representing the newly completed write
                reg_list &= !(1 << next);
            }
        }
        false => { // store to memory
            while reg_list.count_ones() != 0 {
                let next = reg_list.trailing_zeros();

                if pre_offset {
                    match add_offset {
                        true => curr_address += 4,
                        false => curr_address -= 4,
                    }
                }

                let mut rnext = cpu_regs.get_register(next as u8, status.cpsr.mode);
                if next == 15 {
                    rnext += 4;
                }
                memory.write_u32(curr_address, rnext);

                if !pre_offset {
                    match add_offset {
                        true => curr_address += 4,
                        false => curr_address -= 4,
                    }
                }
                reg_list &= !(1 << next);
            }
        }
    }

    // dealing with the S bit fuckery, again
    if s_bit {
        match (r15_in_list, load_bit) {
            (true, true) => status.cpsr = *status.get_spsr(),
            _ => status.cpsr.mode = saved_mode,
        }
    }

    let write_back = (opcode >> 21) & 1 == 1;
    if write_back && !write_back_veto {
        let rn = cpu_regs.get_register_mut(rn_index, status.cpsr.mode);
        *rn = curr_address;
    }
}

fn single_swap(opcode: u32, cpu_regs: &mut Cpu, status: &Status, memory: &mut Memory) {
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