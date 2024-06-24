#![allow(unused)]

const PUSH_POP_MASK: u8 = 0b1111_0110;
const PUSH_POP_ID: u8 = 0b1011_0100;

const SWI_ID: u8 = 0b1101_1111;
const ADD_OFF_ID: u8 = 0b1011_0000;

/// these 2 can be used for both:
/// => "load/store with register offset",
/// => "load/store sign-extended byte/halfword"
const LOAD_REG_OFFSET_MASK: u8 = 0b1111_0010;
const LOAD_SIGN_MASK: u8 = 0b0101_0010;

enum DecodedThumb {
    MoveShiftedReg(u16),
    AddSubtract(u16), 
    AluImmediate(u16),
    AluOperation(u16),
    HiRegisterOperations(u16),
    PcRelativeLoad(u16),
    LoadWithOffset(u16),
    LoadSignExtended(u16),
    LoadWithImmediateOffset(u16),
    LoadHalfword(u16),
    SpRelativeLoad(u16),
    LoadAddress(u16),
    AddOffsetSp(u16), 
    PushPop(u16),
    MultipleLoadStore(u16), 
    ConditionalBranch(u16),
    Swi(u16),
    UnconditionalBranch(u16),
    LongBranchLink(u16),
}

fn decode_thumb(opcode: u16) -> DecodedThumb {
    use DecodedThumb::*;
    // the order in which i decode the instructions matters as it saves a lot of time
    let mut identifier = (opcode >> 8) as u8;
    match identifier {
        SWI_ID => return Swi(opcode),
        ADD_OFF_ID => return AddOffsetSp(opcode),
        _ => {},
    }
    if (identifier & PUSH_POP_MASK) == PUSH_POP_ID {
        return PushPop(opcode);
    }

    // since the 2nd to final bit can either be a 1/0 we account for both
    match (identifier & LOAD_REG_OFFSET_MASK) ^ LOAD_SIGN_MASK {
        0 => return LoadSignExtended(opcode),
        2 => return LoadWithOffset(opcode),
        _ => {},
    }

    // we can narrow down the identifiers
    // just makes it easier to work with
    identifier >>= 2;
    match identifier {
        0b010000 => return AluOperation(opcode),
        0b010001 => return HiRegisterOperations(opcode),
        _ => {},
    }

    identifier >>= 1;
    match identifier {
        0b00011 => return AddSubtract(opcode),
        0b01001 => return PcRelativeLoad(opcode),
        0b11100 => return UnconditionalBranch(opcode),
        _ => {},
    }

    identifier >>= 1;
    match identifier {
        0b1000 => return LoadHalfword(opcode),
        0b1001 => return SpRelativeLoad(opcode),
        0b1010 => return LoadAddress(opcode),
        0b1100 => return MultipleLoadStore(opcode),
        0b1101 => return ConditionalBranch(opcode),
        0b1111 => return LongBranchLink(opcode),
        _ => {}
    }

    identifier >>= 1;
    match identifier {
        0b000 => return MoveShiftedReg(opcode),
        0b001 => return AluImmediate(opcode),
        0b011 => return LoadWithImmediateOffset(opcode),
        _ => {}
    }
    unreachable!("this should never happen, possibly a mistake in the THUMB decoding");
}

pub enum DecodedArm {
    DataProcessing(u32),
    Multiply(u32),
    MultiplyLong(u32),
    SingleDataSwap(u32),
    BranchExchange(u32),
    HalfwordTransferReg(u32),
    HalfwordTransferImm(u32),
    SingleDataTransfer(u32),
    Undefined,
    BlockDataTransfer(u32),
    Branch(u32),
    CoprocDataTransfer(u32),
    CoprocDataOperation(u32),
    CoprocRegTransfer(u32),
    Swi,
}

const BRANCH_EXCHANGE_MASK: u32 = 0b0000_1111_1111_1111_1111_1111_1111_0000;
const BRANCH_EXCHANGE_VALUE: u32 = 0b0000_0001_0010_1111_1111_1111_0001_0000;

pub fn decode_arm(opcode: u32) -> DecodedArm {
    use DecodedArm::*;
    // removes the condition from the opcode. makes it easier to decode
    let unconditioned_opcode = opcode & 0b0000_1111_1111_1111_1111_1111_1111_1111;
    
    // these two just work differently
    if unconditioned_opcode & BRANCH_EXCHANGE_MASK == BRANCH_EXCHANGE_VALUE {
        return BranchExchange(opcode)
    }
    if unconditioned_opcode >> 26 == 0b01 {
        return SingleDataTransfer(opcode)
    }

    let identifier = unconditioned_opcode >> 25;
    match identifier {
        0b011 => return Undefined,
        0b100 => return BlockDataTransfer(opcode),
        0b101 => return Branch(opcode),
        0b110 => return CoprocDataTransfer(opcode),
        _ => {}
    }

    let identifier = unconditioned_opcode >> 24;
    match identifier {
        0b1111 => return Swi,
        0b1110 => {
            match unconditioned_opcode & 0b1000 != 0 {
                true => return CoprocRegTransfer(opcode),
                false => return CoprocDataOperation(opcode),
            }
        }
        _ => {}
    }

    // for this one the identifier becomes the bits 4-7
    let identifier = unconditioned_opcode >> 4 & 0b1111;
    if identifier == 0b1001 {
        match unconditioned_opcode >> 23 {
            0b00 => return Multiply(opcode),
            0b01 => return MultiplyLong(opcode),
            0b10 => return SingleDataSwap(opcode),
            _ => {}
        }
    }
    if identifier & 0b1001 == 0b1001 {
        match unconditioned_opcode >> 22 % 2 {
            0 => return HalfwordTransferReg(opcode),
            1 => return HalfwordTransferImm(opcode),
            _ => unreachable!(),
        }
    }

    // this last section isnt necessary but i am including it as a final check
    // could cause issues later if this didnt happen
    if unconditioned_opcode >> 26 & 0b11 == 0 {
        return DataProcessing(opcode);
    }
    unreachable!("this should never happen, possibly a mistake in the ARM decoding")
}