#[derive(Debug, Clone, Copy)]
pub enum DecodedInstruction {
    Thumb(DecodedThumb),
    Arm(DecodedArm),
}

#[derive(Debug, Clone, Copy)]
pub enum DecodedThumb {
    MoveShiftedReg,
    AddSubtract, 
    AluImmediate,
    AluOperation,
    HiRegisterOperations,
    PcRelativeLoad,
    LoadRegOffset,
    LoadSignExtended,
    LoadImmOffset,
    LoadHalfword,
    SpRelativeLoad,
    LoadAddress,
    AddOffsetSp, 
    PushPop,
    MultipleLoadStore, 
    ConditionalBranch,
    Swi,
    UnconditionalBranch,
    LongBranchLink,
}

#[derive(Debug, Clone, Copy)]
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

const SWI_ID: u8 = 0b1101_1111;
const ADD_OFF_ID: u8 = 0b1011_0000;
pub fn decode_thumb(opcode: u16) -> DecodedThumb {
    use DecodedThumb::*;

    let mut identifier = (opcode >> 8) as u8;

    // these two opcodes require the first byte so easy to get out of the way
    match identifier {
        SWI_ID => return Swi,
        ADD_OFF_ID => return AddOffsetSp,
        _ => {},
    }
    
    // these opcodes have a 7 bit identifier
    // this code works i promise works
    identifier >>= 1;
    match identifier & 0b1111_001 {
        0b0101_000 => return LoadRegOffset,
        0b0101_001 => return LoadSignExtended,
        0b1011_000 => return PushPop,
        _ => {}
    }

    identifier >>= 1;
    match identifier {
        0b010000 => return AluOperation,
        0b010001 => return HiRegisterOperations,
        _ => {}
    }

    identifier >>= 1;
    match identifier {
        0b00011 => return AddSubtract,
        0b01001 => return PcRelativeLoad,
        0b11100 => return UnconditionalBranch,
        _ => {}
    }

    identifier >>= 1;
    match identifier {
        0b1000 => return LoadHalfword,
        0b1001 => return SpRelativeLoad,
        0b1010 => return LoadAddress,
        0b1100 => return MultipleLoadStore,
        0b1101 => return ConditionalBranch,
        0b1111 => return LongBranchLink,
        _ => {}
    }

    identifier >>= 1;
    match identifier {
        0b000 => return MoveShiftedReg,
        0b001 => return AluImmediate,
        0b011 => return LoadImmOffset,
        _ => {}
    }

    unreachable!("THUMB opcode provided is invalid");
}

const BRANCH_EXCHANGE_MASK: u32 = 0b0000_1111_1111_1111_1111_1111_1111_0000;
const BRANCH_EXCHANGE_VALUE: u32 = 0b0000_0001_0010_1111_1111_1111_0001_0000;

const UNDEFINED_MASK: u32 = 0b0000_1110_0000_0000_0000_0000_0001_0000;
const UNDEFINED_VALUE: u32 = 0b0000_0110_0000_0000_0000_0000_0001_0000;

pub fn decode_arm(conditioned_opcode: u32) -> DecodedArm {
    use DecodedArm::*;

    // removes the condition from the opcode. makes it easier to decode
    let opcode = conditioned_opcode & 0b0000_1111_1111_1111_1111_1111_1111_1111;

    // these two just work differently
    if opcode & BRANCH_EXCHANGE_MASK == BRANCH_EXCHANGE_VALUE {
        return BranchExchange
    }
    if opcode >> 26 == 0b01 {
        if opcode & UNDEFINED_MASK == UNDEFINED_VALUE {
            return Undefined
        }
        return SingleDataTransfer
    }

    let identifier = opcode >> 25;
    match identifier {
        0b011 => unreachable!("this should already be handled"),
        0b100 => return BlockDataTransfer,
        0b101 => return Branch,
        0b110 => return CoprocDataTransfer,
        _ => {}
    }

    let identifier = opcode >> 24;
    match identifier {
        0b1111 => return Swi,
        0b1110 => {
            match opcode & 0b1000 != 0 {
                true => return CoprocRegTransfer,
                false => return CoprocDataOperation,
            }
        }
        _ => {}
    }

    if (opcode >> 4) & 0xF == 0b1001 {
        match (opcode >> 23) & 0b11 {
            0b00 => return Multiply,
            0b01 => return MultiplyLong,
            0b10 => return SingleDataSwap,
            _ => unreachable!()
        }
    }        

    // this is so incredibly dumb
    if (opcode >> 25) & 1 == 1 {
        return DataProcessing;
    }

    let identifier = opcode >> 4;    
    if identifier & 0b1001 == 0b1001 {
        match opcode >> 22 & 1 == 1 {
            true => return HalfwordTransferImm,
            false => return HalfwordTransferReg,
        }
    }

    return DataProcessing;
}