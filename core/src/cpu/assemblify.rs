use crate::cpu::decode::{decode_arm, DecodedArm, DecodedThumb};
use super::decode::decode_thumb;

fn convert_cond_string(cond: u8) -> String {
    match cond {
        0b0000 => "eq",
        0b0001 => "ne",
        0b0010 => "cs",
        0b0011 => "cc",
        0b0100 => "mi",
        0b0101 => "pl",
        0b0110 => "vs",
        0b0111 => "vc",
        0b1000 => "hi",
        0b1001 => "ls",
        0b1010 => "ge",
        0b1011 => "lt",
        0b1100 => "gt",
        0b1101 => "le",
        0b1110 => "", // the "AL" suffix can be omitted
        0b1111 => "nv",
        _ => unreachable!(),
    }.to_string()
}

// i feel like it would just be really cool to have this feature
pub fn to_arm_assembly(opcode: u32) -> String {
    let instruction_type = decode_arm(opcode);
    let condition = convert_cond_string((opcode >> 28) as u8);

    use DecodedArm::*;
    // split into two to allow the condition to be added
    let (start, end) = match instruction_type {
        DataProcessing => data_processing_assembly(opcode),
        Multiply => multiply_assembly(opcode),
        MultiplyLong => multiply_long_assembly(opcode),
        SingleDataSwap => single_data_swap_assembly(opcode),
        BranchExchange => branch_exchange_assembly(opcode),
        HalfwordTransferReg => halfword_transfer_assembly(opcode),
        HalfwordTransferImm => halfword_transfer_assembly(opcode),
        SingleDataTransfer => single_data_transfer_assembly(opcode),
        Undefined => panic!("can't really print this out"),
        BlockDataTransfer => block_data_transfer_assembly(opcode),
        Branch => branch_assembly(opcode),
        CoprocDataTransfer => panic!("not hanlded"),
        CoprocDataOperation => panic!("not handled"),
        CoprocRegTransfer => panic!("not handled"),
        Swi => swi_assembly(opcode),
    };

    return format!("{start}{condition}{end}");
}

fn barrel_shifter_assembly(opcode: u32, is_immediate: bool) -> String {
    if is_immediate {
        let immediate_value = opcode & 0xFF;
        if immediate_value == 0 {
            return String::new();
        }

        let shift_amount = ((opcode >> 8) & 0xF) << 1;
        if shift_amount == 0 {
            return format!("0x{immediate_value:X}");
        }

        return format!("{immediate_value:X} ror 0x{shift_amount:X}");
    }

    let shift_type = match (opcode >> 5) & 0b11 {
        0b00 => "lsl",
        0b01 => "lsr",
        0b10 => "asr",
        0b11 => "ror",
        _ => unreachable!(),
    };

    let rm = opcode & 0xF;
    let shift_amount = match (opcode >> 4) & 1 == 1 {
        true => {
            let reg = (opcode >> 8) & 0xF;
            format!("r{reg}")
        }
        false => {
            let imm = (opcode >> 7) & 0x1F;
            format!("{imm}")
        }
    };

    return format!("r{rm} {shift_type} 0x{shift_amount}");
}
fn multiply_assembly(opcode: u32) -> (String, String) {
    let a_bit = (opcode >> 21) & 1 == 1;
    let start = match a_bit {
        true => "mla",
        false => "mul",
    }.to_string();

    let mut rest_of_line = String::new();
    if (opcode >> 20) & 1 == 1 {
        rest_of_line.push('s');
    }

    let rd = (opcode >> 16) & 0xF;
    let rm = opcode & 0xF;
    let rs = (opcode >> 8) & 0xF;
    let rn = (opcode >> 12) & 0xF;

    rest_of_line.push_str(&format!(" r{rd}, r{rm}, r{rs}"));
    if a_bit {
        rest_of_line.push_str(&format!(", r{rn}"));
    }

    return (start, rest_of_line);
 }
fn multiply_long_assembly(opcode: u32) -> (String, String) {
    let a_bit = (opcode >> 21) & 1 == 1;
    let u_bit = (opcode >> 22) & 1 == 1;

    let mut start = match u_bit {
        true => "s",
        false => "u"
    }.to_string();
    start.push_str(match a_bit {
        true => "mlal",
        false => "mull"
    });

    let s_bit = (opcode >> 20) & 1 == 1;
    let s = match s_bit {
        true => "S",
        false => "",
    }.to_string();

    let rdlo = (opcode >> 12) & 0xF;
    let rdhi = (opcode >> 16) & 0xF;
    let rm = opcode & 0xF;
    let rs = (opcode >> 8) & 0xF;

    let rest_of_line = format!("{s} r{rdlo}, r{rdhi}, r{rm}, r{rs}");
    return (start, rest_of_line);

}
fn single_data_swap_assembly(opcode: u32) -> (String, String) {
    let start = "swp".to_string();
    let mut rest_of_line = String::new();

    let b_bit = (opcode >> 22) & 1 == 1;
    if b_bit {
        rest_of_line.push('b');
    }

    let rd = (opcode >> 12) & 0xF;
    let rm = opcode & 0xF;
    let rn = (opcode >> 16) & 0xF;
    rest_of_line.push_str(&format!(" r{rd}, r{rm}, [r{rn}]"));

    return (start, rest_of_line);
}
fn branch_exchange_assembly(opcode: u32) -> (String, String) {
    let start = "bx".to_string();

    let rn = opcode & 0xF;
    let rest_of_line = format!(" r{rn}");

    return (start, rest_of_line);
}
fn halfword_transfer_assembly(opcode: u32) -> (String, String) {
    let l_bit = (opcode >> 20) & 1 == 1;
    let start = match l_bit {
        true => "ldr",
        false => "str"
    }.to_string();
    
    let sb_bits = (opcode >> 5) & 0b11; 
    let mut rest_of_line = match sb_bits {
        0b00 => unreachable!(),
        0b01 => "h",
        0b10 => "sb",
        0b11 => "sh",
        _ => unreachable!(),
    }.to_string();

    let rd = (opcode >> 12) & 0xF;
    let rn = (opcode >> 16) & 0xF;

    let i_bit = (opcode >> 22) & 1 == 1;
    let offset = match i_bit {
        true => {
            //imm
            let offset = ((opcode >> 4) & 0xF0) | (opcode & 0xF);
            if offset == 0 {
                "".to_string()
            } else {
                format!("{offset:X}")
            }
        }
        false => {
            //reg
            barrel_shifter_assembly(opcode, false)
        }
    };

    rest_of_line.push_str(&format!(" r{rd}, [r{rn}"));
    if !offset.is_empty() {
        rest_of_line.push_str(&format!(", 0x{offset}"));
    }
    rest_of_line.push(']');

    return (start, rest_of_line);
}
fn single_data_transfer_assembly(opcode: u32) -> (String, String) {
    let l_bit = (opcode >> 20) & 1 == 1;
    let start = match l_bit {
        true => "ldr",
        false => "stm"
    }.to_string();

    let mut rest_of_line = String::new();
    let b_bit = (opcode >> 22) & 1 == 1;
    let t_bit = (opcode >> 21) & 1 == 1;
    if b_bit {
        rest_of_line.push('b');
    }
    if t_bit {
        rest_of_line.push('t');
    }


    let rn = (opcode >> 16) & 0xF;
    let rd = (opcode >> 12) & 0xF;

    let i_bit = (opcode >> 25) & 1 == 1;
    let offset = match i_bit {
        false => {
            let offset = opcode & 0xFFF;
            format!("{offset:X}")
        }
        true => barrel_shifter_assembly(opcode, false),
    };

    rest_of_line.push_str(&format!(" r{rd}, [r{rn}, {offset}]"));
    return (start, rest_of_line)
}
fn block_data_transfer_assembly(opcode: u32) -> (String, String) {
    let l_bit = (opcode >> 20) & 1 == 1;
    let start = match l_bit {
        true => "ldm",
        false => "stm",
    }.to_string();

    let u_bit = (opcode >> 23) & 1 == 1;
    let p_bit = (opcode >> 24) & 1 == 1;

    let mut rest_of_line = match (u_bit, p_bit) {
        (false, false) => "da",
        (false, true) => "db",
        (true, false) => "ia",
        (true, true) => "ib",
    }.to_string();

    let rn = (opcode >> 16) & 0xF;
    let w_bit = (opcode >> 21) & 1 == 1;
    let w = match w_bit {
        true => "!",
        false => ""
    }.to_string();
    let s_bit = (opcode >> 22) & 1 == 1;
    let s = match s_bit {
        true => "^",
        false => "",
    }.to_string();

    let mut rlist = String::new();
    for i in 0..=15 {
        let exists = (opcode >> i) & 1 == 1;
        if !exists { continue; }

        rlist.push_str(&format!("r{i},"));
    }
    if !rlist.is_empty() {
        rlist.pop();
    }

    rest_of_line.push_str(&format!(" r{rn}{w}, {{{rlist}}}{s}"));
    return (start, rest_of_line);
}
fn branch_assembly(opcode: u32) -> (String, String) {
    let mut start = "b".to_string();
    let l_bit = (opcode >> 24) & 1 == 1;
    if l_bit {
        start.push('l');
    }

    let mut offset = (opcode & 0xFFFFFF) << 2;
    let s = match opcode >> 23 & 1 == 1 {
        true => {
            offset |= 0xFC000000;
            offset = (!offset).wrapping_add(1);
            "-"
        }  
        false => ""
    }.to_string();

    let rest_of_line = format!(" {s}0x{offset:X}");
    return (start, rest_of_line);
}
fn swi_assembly(opcode: u32) -> (String, String) {
    let start = "swi".to_string();
    let comment = opcode & 0xFFFFFF;
    let rest_of_line = format!("{comment:X}");

    return (start, rest_of_line);
}
fn psr_assembly(opcode: u32) -> (String, String) {
    let i_bit = (opcode >> 25) & 1 == 1;
    let op_bit = (opcode >> 21) & 1 == 1;

    let psr_string = match (opcode >> 22) & 1 == 1 {
        true => "spsr",
        false => "cpsr"
    }.to_string();
    let start = match op_bit {
        true => "msr ",
        false => "mrs ",
    }.to_string();

    let rest;
    match op_bit {
        true => { // MSR
            let f_flag = (opcode >> 19) & 1 == 1;
            let c_flag = (opcode >> 16) & 1 == 1;

            let op2 = match i_bit {
                true => barrel_shifter_assembly(opcode, true),
                false => format!("r{}",opcode & 0xF),
            }.to_string();

            let msr_psr = match (f_flag, c_flag) {
                (true, true) => psr_string,
                (true, false) => format!("{psr_string}_flg"),
                (false, true) => format!("{psr_string}_ctl"),
                (false, false) => format!("{psr_string}")
            };
            rest = format!("{msr_psr}, {op2}");
        }
        false => { // MRS
            let rd = (opcode >> 12) & 0xF;
            rest = format!("r{rd}, {psr_string}");
        }
    }
    return (start, rest);
}
fn data_processing_assembly(opcode: u32) -> (String, String) {
    let s_bit = (opcode >> 20) & 1 == 1;
    let inner_opcode = (opcode >> 21) & 0xF;

    if inner_opcode >= 0x8 && inner_opcode <= 0xB && !s_bit {
        return psr_assembly(opcode);
    }

    let name = match inner_opcode {
        0x0 => "and",
        0x1 => "eor",
        0x2 => "sub",
        0x3 => "rsb",
        0x4 => "add",
        0x5 => "adc",
        0x6 => "sbc",
        0x7 => "rsc",
        0x8 => "tst",
        0x9 => "teq",
        0xA => "cmp",
        0xB => "cmn",
        0xC => "orr",
        0xD => "mov",
        0xE => "bic",
        0xF => "mvn",
        _ => unreachable!(),
    }.to_string();
    let mut rest_of_line = String::new();

    let rn = (opcode >> 16) & 0xF;
    let rd = (opcode >> 12) & 0xF;
    let op2_assembly = barrel_shifter_assembly(opcode, (opcode >> 25) & 1 == 1);

    // its a testing opcode
    // so no result register is specified
    if inner_opcode >= 0x8 && inner_opcode <= 0xB {
        return (name, format!(" r{rn}, {op2_assembly}"));
    } 

    if opcode >> 20 & 1 == 1 {
        rest_of_line.push('s')
    }

    if inner_opcode == 0xD || inner_opcode == 0xF {
        rest_of_line.push_str(&format!(" r{rd}, {op2_assembly}"))
    } else {
        rest_of_line.push_str(&format!(" r{rd}, r{rn}, {op2_assembly}"));
    }

    return (name.to_string(), rest_of_line);
}

pub fn to_thumb_assembly(opcode: u16) -> String {
    let instruction_type = decode_thumb(opcode);

    use DecodedThumb::*;
    match instruction_type {
        MoveShifted => move_shifted_assembly(opcode),
        AddSub => add_sub_assembly(opcode),
        AluImmediate => alu_immediate_assembly(opcode),
        AluOperation => alu_operation_assembly(opcode),
        HiRegister => hi_register_assembly(opcode),
        PcRelativeLoad => pc_relative_assembly(opcode),
        MemRegOffset => mem_reg_offset_assembly(opcode),
        MemSignExtended => mem_sign_assembly(opcode),
        MemImmOffset => mem_imm_offset_assembly(opcode),
        MemHalfword => mem_halfword_assembly(opcode),
        MemSpRelative => mem_sp_relative_assembly(opcode),
        LoadAddress => load_address_assembly(opcode),
        OffsetSp => offset_sp_assembly(opcode),
        PushPop => push_pop_assembly(opcode),
        MemMultiple => mem_multiple_assembly(opcode),
        CondBranch => cond_branch_assembly(opcode),
        Swi => thumb_swi_assembly(opcode),
        UncondBranch => uncond_branch_assembly(opcode),
        LongBranch => long_branch_assembly(opcode),
    }
}
fn move_shifted_assembly(opcode: u16) -> String {
    let op = (opcode >> 11) & 0b11;
    let instruction = match op {
        0b00 => "lsl",
        0b01 => "lsr",
        0b10 => "asr",
        _ => unreachable!(),
    };
    let rd = opcode & 0x7;
    let rs = (opcode >> 3) & 0x7;
    let offset = (opcode >> 6) & 0x1F;

    return format!("{instruction} r{rd}, r{rs}, 0x{offset:X}");
}
fn add_sub_assembly(opcode: u16) -> String {
    let i_bit = (opcode >> 10) & 1 == 1;
    let op = (opcode >> 9) & 1 == 1;
    let unformatted_value = (opcode >> 6) & 0x7;

    let instruction = match op {
        true => "sub",
        false => "add",
    }.to_string(); 

   let value = match i_bit {
        true => format!("0x{unformatted_value:X}"),
        false => format!("r{unformatted_value}"),
    };
    let rd = opcode & 0x7;
    let rs = (opcode >> 3) & 0x7;

    return format!("{instruction} r{rd}, r{rs}, {value}");
}
fn alu_immediate_assembly(opcode: u16) -> String {
    let op = (opcode >> 11) & 0x3;

    let instruction = match op {
        0b00 => "mov",
        0b01 => "cmp",
        0b10 => "add",
        0b11 => "sub",
        _ => unreachable!(),
    }.to_string();

    let rd = (opcode >> 8) & 0x7;
    let offset = opcode & 0xFF;

    return format!("{instruction} r{rd}, 0x{offset:X}");
}
fn alu_operation_assembly(opcode: u16) -> String {
    let op = (opcode >> 6) & 0xF;
    let rs = (opcode >> 3) & 0x7;
    let rd = opcode & 0x7;

    let instruction = match op {
        0b0000 => "and",
        0b0001 => "eor",
        0b0010 => "lsl",
        0b0011 => "lsr",
        0b0100 => "asr",
        0b0101 => "adc",
        0b0110 => "sbc",
        0b0111 => "ror",
        0b1000 => "tst",
        0b1001 => "neg",
        0b1010 => "cmp",
        0b1011 => "cmn",
        0b1100 => "orr",
        0b1101 => "mul",
        0b1110 => "bic",
        0b1111 => "mvn",
        _ => unreachable!(),
    }.to_string();

    return format!("{instruction} r{rd}, r{rs}");
}
fn hi_register_assembly(opcode: u16) -> String {
    let op = (opcode >> 8) & 0b11;
    let instruction = match op {
        0b00 => "add",
        0b01 => "cmp",
        0b10 => "mov",
        0b11 => "bx",
        _ => unreachable!(),
    }.to_string();

    let h1 = (opcode >> 7) & 1 == 1;
    let h2 = (opcode >> 6) & 1 == 1;

    let rd = (opcode & 0x7) + if h1 { 8 } else { 0 };
    let rs = (opcode >> 3 & 0x7) + if h2 { 8 } else { 0 };

    return format!("{instruction} r{rd}, r{rs}");
}
fn pc_relative_assembly(opcode: u16) -> String {
    let rd = (opcode >> 8) & 0x7;
    let offset = (opcode & 0xFF) << 2;

    return format!("ldr r{rd}, [pc, 0x{offset}]");
}
fn mem_reg_offset_assembly(opcode: u16) -> String {
    let b_bit = (opcode >> 10) & 1 == 1;
    let l_bit = (opcode >> 11) & 1 == 1;

    let ro = (opcode >> 6) & 0x7;
    let rb = (opcode >> 3) & 0x7;
    let rd = opcode & 0x7;

    let b = match b_bit {
        true => "b",
        false => ""
    }.to_string();
    let instruction = match l_bit {
        true => "ldr",
        false => "str",
    }.to_string();

    return format!("{instruction}{b} r{rd}, [r{rb}, r{ro}]");
}
fn mem_sign_assembly(opcode: u16) -> String {
    let ro = (opcode >> 6) & 0x7;
    let rb = (opcode >> 3) & 0x7;
    let rd = opcode & 0x7;

    let hs_bits = (opcode >> 10) & 0b11;
    let instruction = match hs_bits {
        0b00 => "strh",
        0b01 => "ldrh",
        0b10 => "ldsb",
        0b11 => "ldsh",
        _ => unreachable!(),
    }.to_string();

    return format!("{instruction} r{rd}, [r{rb}, r{ro}]");
}
fn mem_imm_offset_assembly(opcode: u16) -> String {
    let l_bit = (opcode >> 11) & 1 == 1;
    let b_bit = (opcode >> 12) & 1 == 1;

    let offset = (opcode >> 6) & 0x1F << if b_bit { 0 } else { 2 };
    let rb = (opcode >> 3) & 0x7;
    let rd = opcode & 0x7;

    let b = match b_bit {
        true => "b",
        false => "",
    }.to_string();
    let instruction = match l_bit {
        true => "ldr",
        false => "str"
    }.to_string();

    return format!("{instruction}{b} r{rd}, [r{rb}, 0x{offset:X}]");
}
fn mem_halfword_assembly(opcode: u16) -> String {
    let l_bit = (opcode >> 11) & 1 == 1;
    let offset = ((opcode >> 6) & 0x1F) << 1;

    let rb = (opcode >> 3) & 0x7;
    let rd = opcode & 0x7;

    let instruction = match l_bit {
        true => "strh",
        false => "ldrh"
    }.to_string();

    if offset == 0 {
        return format!("{instruction} r{rd}, [r{rb}]");
    }
    return format!("{instruction} r{rd}, [r{rb}, 0x{offset:X}]");
}
fn mem_sp_relative_assembly(opcode: u16) -> String {
    let l_bit = (opcode >> 11) & 1 == 1;
    let rd = (opcode >> 8) & 0x7;

    let word = opcode & 0xFF;
    let instruction = match l_bit {
        true => "ldr",
        false => "str",
    }.to_string();

    if word == 0 {
        return format!("{instruction} r{rd}, [SP]");
    }
    return format!("{instruction} r{rd}, [SP, 0x{word:X}]");
}
fn load_address_assembly(opcode: u16) -> String {
    let sp_bit = (opcode >> 11) & 1 == 1;
    let reg = match sp_bit {
        true => "sp",
        false => "pc",
    }.to_string();

    let rd = (opcode >> 8) & 0x7;
    let word = (opcode & 0xFF) << 2;

    return format!("add rd{rd}, {reg}, 0x{word:X}");
}
fn offset_sp_assembly(opcode: u16) -> String {
    let s_bit = (opcode >> 7) & 1 == 1;
    let sign = match s_bit {
        true => "-",
        false => ""
    }.to_string();

    let s_word = opcode & 0x7F;
    return format!("add sp, 0x{sign}{s_word:X}");
}
fn push_pop_assembly(opcode: u16) -> String {
    let l_bit = (opcode >> 11) & 1 == 1;

    let instruction = match l_bit {
        true => "pop",
        false => "push"
    }.to_string();

    let r_bit = (opcode >> 8) & 1 == 1;
    let mut rlist = String::new();

    for i in 0..8 {
        let exists = (opcode >> i) & 1 == 1;
        if !exists { continue; }

        rlist.push_str(&format!("r{i}"));
    }
    if r_bit {
        match l_bit {
            true => rlist.push_str("pc"),
            false => rlist.push_str("lr"),
        }
    }

    return format!("{instruction} {{{rlist}}}");
}
fn mem_multiple_assembly(opcode: u16) -> String {
    let l_bit = (opcode >> 11) & 1 == 1;
    let instruction = match l_bit {
        true => "ldmia",
        false => "stmia",
    }.to_string();

    let rb = (opcode >> 8) & 0x7;
    let mut rlist = String::new();
    for i in 0..8 {
        let exists = (opcode >> i) & 1 == 1;
        if !exists { continue; }

        rlist.push_str(&format!("r{i},"));
    }
    // remove the final comma and space
    if !rlist.is_empty() {
        rlist.pop();
    }

    return format!("{instruction} r{rb}!, {{{rlist}}}");
}
fn cond_branch_assembly(opcode: u16) -> String {
    let cond = convert_cond_string((opcode >> 8) as u8 & 0xF);
    let offset = (opcode & 0xFF) << 1;

    return format!("b{cond} 0x{offset:X}");
}
fn thumb_swi_assembly(opcode: u16) -> String {
    let value = opcode & 0xFF;
    return format!("swi {value:X}");
}
fn uncond_branch_assembly(opcode: u16) -> String {
    let offset = (opcode & 0x7FF) << 1;
    return format!("b {offset:X}");
}
fn long_branch_assembly(opcode: u16) -> String {
    let offset = (opcode & 0x7FF) << 1;
    return format!("bl {offset:X}");
}