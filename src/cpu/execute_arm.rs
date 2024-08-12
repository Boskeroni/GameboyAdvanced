use crate::cpu::registers::{*, status_registers::*};
use crate::cpu::decode::DecodedArm;

pub fn execute_arm(
    opcode: u32, 
    decoded_arm: DecodedArm,
    cpu_regs: &mut CpuRegisters,
    status: &mut StatusRegisters
) {
    // first check if we even have to do it
    let condition = opcode >> 28;
    if !check_arm_condition(condition as u8, &status.cpsr) {
        return;
    }

    use DecodedArm::*;
    match decoded_arm {
        DataProcessing => arm_data_processing(opcode, cpu_regs, status),
        Multiply => arm_multiply(opcode, cpu_regs, status),
        MultiplyLong => arm_multiply_long(opcode, cpu_regs, status),
        SingleDataSwap => todo!("memory needs to be implemented for this"),
        BranchExchange => arm_branch_exchange(opcode, cpu_regs, status),
        HalfwordTransferReg => todo!("memory needs to be implemented for this"),
        HalfwordTransferImm => todo!("memory needs to be implemented for this"),
        SingleDataTransfer => todo!("memory needs to be implemented for this"),
        Undefined => {},
        BlockDataTransfer => todo!("memory needs to be implemented for this"),
        Branch => arm_branch_link(opcode, cpu_regs, status),
        CoprocDataOperation => eprintln!("Coprocessor Data Operations arent handled for GBA"),
        CoprocDataTransfer => eprintln!("Coprocessor Data Transfers arent handled for GBA"),
        CoprocRegTransfer => eprintln!("Coprocessor Register Transfers arent handled for GBA"),
        Swi => arm_software_interrupt(cpu_regs, status),
    }
}

fn arm_branch_link(opcode: u32, cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
    // the bottom 24-bits
    let mut offset = (opcode & 0xFFFFFF) as u32; 
    offset <<= 2;
    if (opcode >> 23) & 1 == 1 {
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

fn arm_branch_exchange(opcode: u32, cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
    assert!((opcode & 0xF) != 15, "BRANCH EXCHANGE is undefined if Rn == 15");

    status.cpsr.t = (opcode & 1) == 1;
    let rn = cpu_regs.get_register((opcode & 0b1111) as u8, status.cpsr.mode);
    let pc = cpu_regs.get_register_mut(15, status.cpsr.mode);

    *pc = rn
}

fn arm_data_processing(opcode: u32, cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
    /// there are so many goddamn fucking edge cases that require me to
    /// leave / exit early that i decided to create its own function for convenience
    fn decide_op2_with_carry(shift_type: u32, mut shift_amount: u32, reg: u32, old_carry: bool) -> (u32, bool) {
        match shift_type {
            0b00 => {
                // the carry bit stays the same if the shift instruction is LSL #0
                // the shift amount can be greater than 32 if its from a register
                if shift_amount > 32 {
                    return (0, false)
                }
                let res = reg << shift_amount;
                if shift_amount == 0 {
                    return (res, old_carry)
                }
                // this line may look wrong but the math does check out
                return (res, (reg >> (32 - shift_amount)) & 1 != 0)
            } // Logical left shift
            0b01 => {
                if shift_amount > 32 {
                    return (0, false)
                }
                // LSR #0 automatically becomes LSL #0 so it doesnt need an edge case
                if shift_amount == 0 {
                    shift_amount = 32;
                }
                return (reg >> shift_amount, (reg >> (shift_amount - 1) & 1) != 0)
            } // logical right shift
            0b10 => {
                let padding = (reg & 0x80000000) as i32;
                if shift_amount > 32 {
                    return ((padding >> 31) as u32, padding != 0)
                }
                if shift_amount == 0 {
                    shift_amount = 32;
                }
                return ((reg >> shift_amount) | (padding >> shift_amount) as u32, reg >> (shift_amount - 1) & 1 != 0)
            } // Arithmetic shift left
            0b11 => {
                // when it is ROR #0 it means RRX
                if shift_amount == 0 {
                    return ((reg >> 1) | (old_carry as u32) << 31, reg & 1 != 0)
                }
                let res = reg.rotate_right(shift_amount);
                return (res, (res >> 31) & 1 != 0)
            },
            _ => unreachable!(),
        };
    }

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
        let rm_index = opcode & 0xF;
        let rm = cpu_regs.get_register(rm_index as u8, status.cpsr.mode);

        let shift_amount = match (opcode >> 4) & 1 {
            0 => (opcode >> 7) & 0x1F, // the simple case :)
            1 => {
                let rs = (opcode >> 8) as u8 & 0xF;
                assert!(rs != 15, "Rs cannot equal 15 in this case");
                cpu_regs.get_register(rs, status.cpsr.mode) & 0xFF
            },
            _ => unreachable!(),
        };

        let shift_type = (opcode >> 5) & 0b11;
        (op2, op2_carry) = decide_op2_with_carry(shift_type, shift_amount, rm, status.cpsr.c);
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
    let op1 = cpu_regs.get_register(op1_reg, status.cpsr.mode);
    let src = cpu_regs.get_register_mut(((opcode >> 12) as u8) & 0xF, status.cpsr.mode);

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
    if !change_cpsr || op1_reg != 15 {
        return;
    }

    status.cpsr.z = *src == 0;
    status.cpsr.n = (*src >> 31) != 0;

    // the mathematical instructions
    if [0b0010, 0b0011, 0b0100, 0b0101, 0b0110, 0b0111, 0b1010, 0b1011].contains(&operation) {
        // it says that this should be ignored sometimes
        // but honestly i don't know when so i am doing it all the time
        status.cpsr.v = ((*src >> 31) != 0) & ((!backup >> 31) != 0); 
        status.cpsr.c = alu_carry;
    } else {
        status.cpsr.c = op2_carry;
    }

    if undo {
        *src = backup;
    }
}

fn psr_transfer(opcode: u32, cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
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

            let new_psr = convert_u32_cpsr(operand);
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

fn arm_multiply(opcode: u32, cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
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

fn arm_multiply_long(opcode: u32, cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
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
fn arm_software_interrupt(cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
    // spsr_svc gets the old cpsr transferred into it
    status.set_specific_spsr(status.cpsr, ProcessorMode::Supervisor);

    status.cpsr.mode = ProcessorMode::Supervisor;
    let pc = cpu_regs.get_register(15, status.cpsr.mode);
    let save_pc = cpu_regs.get_register_mut(14, status.cpsr.mode);
    *save_pc = pc;

    let change_pc = cpu_regs.get_register_mut(15, status.cpsr.mode);
    *change_pc = 0x08;
}
