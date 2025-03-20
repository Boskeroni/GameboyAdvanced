#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecodedInstruction {
    Thumb(DecodedThumb),
    Arm(DecodedArm),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecodedThumb {
    MoveShifted,
    AddSub, 
    AluImmediate,
    AluOperation,
    HiRegister,
    PcRelativeLoad,
    MemRegOffset,
    MemSignExtended,
    MemImmOffset,
    MemHalfword,
    MemSpRelative,
    LoadAddress,
    OffsetSp, 
    PushPop,
    MemMultiple, 
    CondBranch,
    Swi,
    UncondBranch,
    LongBranch,
}
pub fn decode_thumb(opcode: u16) -> DecodedThumb {
    let mut identifier = (opcode >> 8) as u8;

    // these two opcodes require the first byte so easy to get out of the way
    match identifier {
        0b1101_1111 => return DecodedThumb::Swi,
        0b1011_0000 => return DecodedThumb::OffsetSp,
        _ => {},
    }
    
    // these opcodes have a 7 bit identifier
    // this code works i promise works
    identifier >>= 1;
    match identifier & 0b1111_001 {
        0b0101_000 => return DecodedThumb::MemRegOffset,
        0b0101_001 => return DecodedThumb::MemSignExtended,
        0b1011_000 => return DecodedThumb::PushPop,
        _ => {}
    }

    identifier >>= 1;
    match identifier {
        0b010000 => return DecodedThumb::AluOperation,
        0b010001 => return DecodedThumb::HiRegister,
        _ => {}
    }

    identifier >>= 1;
    match identifier {
        0b00011 => return DecodedThumb::AddSub,
        0b01001 => return DecodedThumb::PcRelativeLoad,
        0b11100 => return DecodedThumb::UncondBranch,
        _ => {}
    }

    identifier >>= 1;
    match identifier {
        0b1000 => return DecodedThumb::MemHalfword,
        0b1001 => return DecodedThumb::MemSpRelative,
        0b1010 => return DecodedThumb::LoadAddress,
        0b1100 => return DecodedThumb::MemMultiple,
        0b1101 => return DecodedThumb::CondBranch,
        0b1111 => return DecodedThumb::LongBranch,
        _ => {}
    }

    identifier >>= 1;
    match identifier {
        0b000 => return DecodedThumb::MoveShifted,
        0b001 => return DecodedThumb::AluImmediate,
        0b011 => return DecodedThumb::MemImmOffset,
        _ => {}
    }

    unreachable!("THUMB opcode provided is invalid: {opcode:X}");
}

const BRANCH_EXCHANGE_MASK: u32 = 0b0000_1111_1111_1111_1111_1111_1111_0000;
const BRANCH_EXCHANGE_VALUE: u32 = 0b0000_0001_0010_1111_1111_1111_0001_0000;
const UNDEFINED_MASK: u32 = 0b0000_1110_0000_0000_0000_0000_0001_0000;
const UNDEFINED_VALUE: u32 = 0b0000_0110_0000_0000_0000_0000_0001_0000;

const REMOVE_CONDITION_MASK: u32 = 0b0000_1111_1111_1111_1111_1111_1111_1111;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecodedArm {
    DataProcessing,
    Multiply,
    MultiplyLong,
    SingleDataSwap,
    BranchExchange,
    HalfwordTransferReg,
    HalfwordTransferImm,
    SingleDataTransfer,
    Undefined,
    BlockDataTransfer,
    Branch,
    CoprocDataTransfer,
    CoprocDataOperation,
    CoprocRegTransfer,
    Swi,
}

pub fn decode_arm(conditioned_opcode: u32) -> DecodedArm {
    let opcode = conditioned_opcode & REMOVE_CONDITION_MASK;

    // this is just a preliminary check
    if (opcode >> 25) == 0b001 {
        return DecodedArm::DataProcessing;
    }
    // these two just work differently
    if opcode & BRANCH_EXCHANGE_MASK == BRANCH_EXCHANGE_VALUE {
        return DecodedArm::BranchExchange
    }
    else if opcode & UNDEFINED_MASK == UNDEFINED_VALUE {
        return DecodedArm::Undefined;
    }
    else if opcode >> 26 == 0b01 {
        return DecodedArm::SingleDataTransfer
    }

    let identifier = opcode >> 25;
    match identifier {
        0b011 => unreachable!("this should already be handled"),
        0b100 => return DecodedArm::BlockDataTransfer,
        0b101 => return DecodedArm::Branch,
        0b110 => return DecodedArm::CoprocDataTransfer,
        _ => {}
    }

    let identifier = opcode >> 24;
    match identifier {
        0b1111 => return DecodedArm::Swi,
        0b1110 => {
            match opcode & 0b1000 != 0 {
                true => return DecodedArm::CoprocRegTransfer,
                false => return DecodedArm::CoprocDataOperation,
            }
        }
        _ => {}
    }

    if (opcode >> 4) & 0xF == 0b1001 {
        match (opcode >> 23) & 0b11 {
            0b00 => return DecodedArm::Multiply,
            0b01 => return DecodedArm::MultiplyLong,
            0b10 => return DecodedArm::SingleDataSwap,
            _ => unreachable!()
        }
    }        

    if (opcode >> 25) & 1 == 1 {
        return DecodedArm::DataProcessing;
    }

    let identifier = opcode >> 4;    
    if identifier & 0b1001 == 0b1001 {
        match opcode >> 22 & 1 == 1 {
            true => return DecodedArm::HalfwordTransferImm,
            false => return DecodedArm::HalfwordTransferReg,
        }
    }

    return DecodedArm::DataProcessing;
}