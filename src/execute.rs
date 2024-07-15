use crate::cpu::{CpuRegisters, ProcessorMode};
use crate::cpu::status_registers::{check_arm_condition, convert_cpsr_u32, convert_u32_cpsr, StatusRegisters};
use crate::decode::{DecodedArm, DecodedThumb};

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
        SingleDataSwap => todo!(),
        BranchExchange => arm_branch_exchange(opcode, cpu_regs, status),
        HalfwordTransferReg => todo!(),
        HalfwordTransferImm => todo!(),
        SingleDataTransfer => arm_single_data_transfer(opcode, cpu_regs, status),
        Undefined => {},
        BlockDataTransfer => todo!(),
        Branch => arm_branch_link(opcode, cpu_regs, status),
        CoprocDataOperation => eprintln!("Coprocessor Data Operations arent handled for GBA"),
        CoprocDataTransfer => eprintln!("Coprocessor Data Transfers arent handled for GBA"),
        CoprocRegTransfer => eprintln!("Coprocessor Register Transfers arent handled for GBA"),
        Swi => arm_software_interrupt(cpu_regs, status),
    }
}

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

fn arm_branch_link(opcode: u32, cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
    let mut offset = (opcode & 0b1111_1111_1111_1111_1111_1111) as u32;
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
    status.cpsr.t = (opcode & 1) == 1;
    let rn = cpu_regs.get_register((opcode & 0b1111) as u8, status.cpsr.mode);
    let pc = cpu_regs.get_register_mut(15, status.cpsr.mode);

    *pc = rn
}

fn arm_single_data_transfer(opcode: u32, cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
    todo!();
}

fn arm_multiply_long(opcode: u32, cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
    let rs = cpu_regs.get_register(((opcode>>8) & 0b1111) as u8, status.cpsr.mode);
    let rm = cpu_regs.get_register((opcode & 0b1111) as u8, status.cpsr.mode);

    let rdlo_index = (opcode >> 12) & 0b1111;
    let rdhi_index = (opcode >> 16) & 0b1111;

    let mut result = match (opcode >> 22) & 1 {
        0 => (rs as u64) * (rm as u64),
        1 => ((rs as i64) * (rm as i64)) as u64,
        _ => unreachable!(),
    };
    if (opcode >> 21) & 1 == 1 {
        let accumulate = (cpu_regs.get_register(rdhi_index as u8, status.cpsr.mode) as u64) << 32
                         |cpu_regs.get_register(rdlo_index as u8, status.cpsr.mode) as u64;

        result += accumulate;
    }

    let rdhi = cpu_regs.get_register_mut(((opcode >> 16) & 0b1111) as u8, status.cpsr.mode);
    *rdhi = (result >> 32) as u32;

    let rdlo =  cpu_regs.get_register_mut(((opcode >> 12) & 0b1111) as u8, status.cpsr.mode);
    *rdlo = result as u32;

    if (opcode >> 20) & 1 == 1 {
        status.cpsr.n = (result >> 63) & 1 == 1;
        status.cpsr.z = result == 0;
    }
}

fn arm_multiply(opcode: u32, cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
    let rn = cpu_regs.get_register(((opcode>>12) &0b1111) as u8, status.cpsr.mode);
    let rs = cpu_regs.get_register(((opcode>>8) & 0b1111) as u8, status.cpsr.mode);
    let rm = cpu_regs.get_register((opcode & 0b1111) as u8, status.cpsr.mode);
    let rd = cpu_regs.get_register_mut(((opcode>>16)&0b1111) as u8, status.cpsr.mode);

    *rd = rm.wrapping_mul(rs);
    if (opcode >> 21) & 1 == 1 {
        *rd += rn;
    }

    if (opcode >> 20) & 1 == 1 {
        status.cpsr.z = *rd == 0;
        status.cpsr.n = (*rd >> 31) != 0;
    }
}

fn arm_data_processing(opcode: u32, cpu_regs: &mut CpuRegisters, status: &mut StatusRegisters) {
    let change_cpsr = (opcode >> 20) & 1 == 1;
    let operation = (opcode >> 21) & 0b1111;

    // its a test one, but doesnt change cpsr, so psr transfer
    if operation >= 8 && operation <= 11 && !change_cpsr {
        psr_transfer(opcode, status, cpu_regs);
        return
    }
    
    let op2 = match (opcode >> 25) & 1 {
        0 => { // operand2 is a register with shift
            let r = cpu_regs.get_register((opcode as u8) & 0b1111, status.cpsr.mode);
            let shift_id = (opcode >> 4) & 0b1111_1111;

            let shift_amount = match shift_id & 1 {
                0 => shift_id >> 7,
                1 => cpu_regs.get_register((shift_id>>8) as u8, status.cpsr.mode) & 0b1111_1111,
                _ => unreachable!(),
            };

            match (shift_id >> 1) & 0b11 {
                0b00 => r << shift_amount,
                0b01 => r >> shift_amount,
                0b10 => {
                    let padding = (1 << 31) as i32;
                    if r >> 31 == 1 {
                        (r >> shift_amount) | (padding >> shift_amount) as u32
                    } else {
                        r >> shift_amount
                    }
                }, // arithmetic shift
                0b11 => r.rotate_right(shift_amount),
                _ => unreachable!()
            }
        }
        1 => {
            let imm = opcode & 0b1111_1111;
            let rotate = (opcode >> 8) & 0b1111;
            imm.rotate_right(rotate >> 1)
        },
        _ => unreachable!()
    };

    let op1 = cpu_regs.get_register(((opcode>>16) as u8) & 0b1111, status.cpsr.mode);
    let src = cpu_regs.get_register_mut(((opcode >> 12) as u8) & 0b1111, status.cpsr.mode);

    let mut undo = false;
    let backup = *src;
    match operation {
        0b0000 => *src = op1 & op2, // and
        0b0001 => *src = op1 ^ op2, // eor
        0b0010 => *src = op1 - op2, // sub
        0b0011 => *src = op2 - op1, // rsb
        0b0100 => *src = op1 + op2, // add
        0b0101 => *src = op1 + op2 + status.cpsr.c as u32, // adc
        0b0110 => *src = op1 - op2 + status.cpsr.c as u32 - 1, // sbc,
        0b0111 => *src = op2 - op1 + status.cpsr.c as u32 - 1, // rsc
        0b1000 => {*src = op1 & op2; undo = true}, // tst
        0b1001 => {*src = op1 ^ op2; undo = true}, // teq
        0b1010 => {*src = op1 - op2; undo = true}, // cmp
        0b1011 => {*src = op1 + op2; undo = true}, // cmn
        0b1100 => *src = op1 | op2, // orr
        0b1101 => *src = op2, // mov
        0b1110 => *src = op1 & !op2, // bic
        0b1111 => *src = !op2, // mvn
        _ => unreachable!()
    }

    if !change_cpsr {
        return;
    }

    status.cpsr.z = *src == 0;
    status.cpsr.n = (*src >> 31) != 0;

    // the logical operations
    // 0b0000, 0b0001, 0b1000, 0b1001, 0b1100, 0b1101, 0b1110, 0b1111
    if [0b0000, 0b0001, 0b1000, 0b1001, 0b1100, 0b1101, 0b1110, 0b1111].contains(&operation) {
        // idk what to do
    } else {
        // idk what to do
    }

    if undo {
        *src = backup;
    }
}

fn psr_transfer(opcode: u32, status: &mut StatusRegisters, cpu_regs: &mut CpuRegisters) {
    match (opcode >> 12) & 0b11_1111_1111 {
        0b1010011111 => {
            let used_cpsr = match (opcode >> 22) & 1 {
                0 => &status.cpsr,
                1 => status.get_spsr(),
                _ => unreachable!(),
            };

            let reg = cpu_regs.get_register_mut(((opcode>>12)&0b111) as u8, status.cpsr.mode);
            *reg = convert_cpsr_u32(used_cpsr);
        }
        0b1010001111 => {
            let reg = cpu_regs.get_register((opcode & 0b111) as u8, status.cpsr.mode);

            match (opcode >> 22) & 1 {
                0 => status.cpsr = convert_u32_cpsr(reg),
                1 => status.set_spsr(convert_u32_cpsr(reg)),
                _ => unreachable!()
            }
        }
        _ => { // im just assuming all the other ones are MRS
            let data = match (opcode >> 25) & 1 {
                0 => {
                    cpu_regs.get_register((opcode & 0b1111) as u8, status.cpsr.mode)
                }
                1 => {
                    let imm = opcode & 0b1111_1111;
                    let rotate = (opcode >> 8) & 0b1111;
                    imm.rotate_right(rotate >> 2)
                }
                _ => unreachable!(),
            };
            match (opcode >> 22) & 1 {
                0 => status.set_flags_cpsr(convert_u32_cpsr(data)),
                1 => status.set_flags_spsr(convert_u32_cpsr(data)),
                _ => unreachable!(),
            }
        }
    }
}


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
        MoveShiftedReg => thumb_move_shifted(opcode, cpu_regs),
        AddSubtract => thumb_add_sub(opcode, cpu_regs),
        AluImmediate => thumb_alu_imm(opcode, cpu_regs),
        _ => todo!(),
    }
    todo!("still need to implement the status registers changing for this");
}

fn thumb_move_shifted(opcode: u16, cpu_regs: &mut CpuRegisters) {
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
        0b0111 =>
    }
}