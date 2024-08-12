pub mod execute_arm;
pub mod execute_thumb;
pub mod registers;

pub mod decode {
    pub enum DecodedThumb {
        MoveShiftedReg,
        AddSubtract, 
        AluImmediate,
        AluOperation,
        HiRegisterOperations,
        PcRelativeLoad,
        LoadWithOffset,
        LoadSignExtended,
        LoadWithImmediateOffset,
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
    
    
    const PUSH_POP_MASK: u8 = 0b1111_0110;
    const PUSH_POP_ID: u8 = 0b1011_0100;
    
    const SWI_ID: u8 = 0b1101_1111;
    const ADD_OFF_ID: u8 = 0b1011_0000;
    
    /// these 2 can be used for both:
    /// => "load/store with register offset",
    /// => "load/store sign-extended byte/halfword"
    const LOAD_REG_OFFSET_MASK: u8 = 0b1111_0010;
    const LOAD_SIGN_MASK: u8 = 0b0101_0010;
    
    pub fn decode_thumb(opcode: u16) -> DecodedThumb {
        use DecodedThumb::*;
        // the order in which i decode the instructions matters as it saves a lot of time
        let mut identifier = (opcode >> 8) as u8;
        match identifier {
            SWI_ID => return Swi,
            ADD_OFF_ID => return AddOffsetSp,
            _ => {},
        }
        if (identifier & PUSH_POP_MASK) == PUSH_POP_ID {
            return PushPop;
        }
    
        // since the 2nd to final bit can either be a 1/0 we account for both
        match (identifier & LOAD_REG_OFFSET_MASK) ^ LOAD_SIGN_MASK {
            0 => return LoadSignExtended,
            2 => return LoadWithOffset,
            _ => {},
        }
    
        // we can narrow down the identifiers
        // just makes it easier to work with
        identifier >>= 2;
        match identifier {
            0b010000 => return AluOperation,
            0b010001 => return HiRegisterOperations,
            _ => {},
        }
    
        identifier >>= 1;
        match identifier {
            0b00011 => return AddSubtract,
            0b01001 => return PcRelativeLoad,
            0b11100 => return UnconditionalBranch,
            _ => {},
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
            0b011 => return LoadWithImmediateOffset,
            _ => {}
        }
        unreachable!("this should never happen, possibly a mistake in the THUMB decoding");
    }
    
    const BRANCH_EXCHANGE_MASK: u32 = 0b0000_1111_1111_1111_1111_1111_1111_0000;
    const BRANCH_EXCHANGE_VALUE: u32 = 0b0000_0001_0010_1111_1111_1111_0001_0000;
    
    pub fn decode_arm(opcode: u32) -> DecodedArm {
        use DecodedArm::*;
        // removes the condition from the opcode. makes it easier to decode
        let unconditioned_opcode = opcode & 0b0000_1111_1111_1111_1111_1111_1111_1111;
        
        // these two just work differently
        if unconditioned_opcode & BRANCH_EXCHANGE_MASK == BRANCH_EXCHANGE_VALUE {
            return BranchExchange
        }
        if unconditioned_opcode >> 26 == 0b01 {
            return SingleDataTransfer
        }
    
        let identifier = unconditioned_opcode >> 25;
        match identifier {
            0b011 => return Undefined,
            0b100 => return BlockDataTransfer,
            0b101 => return Branch,
            0b110 => return CoprocDataTransfer,
            _ => {}
        }
    
        let identifier = unconditioned_opcode >> 24;
        match identifier {
            0b1111 => return Swi,
            0b1110 => {
                match unconditioned_opcode & 0b1000 != 0 {
                    true => return CoprocRegTransfer,
                    false => return CoprocDataOperation,
                }
            }
            _ => {}
        }
    
        // for this one the identifier becomes the bits 4-7
        let identifier = unconditioned_opcode >> 4 & 0b1111;
        if identifier == 0b1001 {
            match unconditioned_opcode >> 23 {
                0b00 => return Multiply,
                0b01 => return MultiplyLong,
                0b10 => return SingleDataSwap,
                _ => {}
            }
        }
        if identifier & 0b1001 == 0b1001 {
            match unconditioned_opcode >> 22 % 2 {
                0 => return HalfwordTransferReg,
                1 => return HalfwordTransferImm,
                _ => unreachable!(),
            }
        }
    
        // this last section isnt necessary but i am including it as a final check
        // could cause issues later if this didnt happen
        if unconditioned_opcode >> 26 & 0b11 == 0 {
            return DataProcessing;
        }
        unreachable!("this should never happen, possibly a mistake in the ARM decoding")
    }
}
