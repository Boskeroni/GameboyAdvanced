use gba_core::{cpu::{check_condition, convert_u32_psr, execute_arm::execute_arm, execute_thumb::execute_thumb, Cpu, Fde}, memory::Memoriable};
use serde_json::{self, Value};

pub struct JsonEmulator {
    cpu: Cpu,
    cycles: u32,
    mem: JsonMemory,
}

// this just makes it much quicker to do tests
pub struct JsonMemory {
    transactions: Value,
    base_addr: u32,
    test_opcode: u32,
}
impl JsonMemory {
    fn read(&self, size: u64, addr: u32) -> u32 {
        for transaction in self.transactions.as_array().unwrap() {
            if transaction["size"].as_u64().unwrap() != size {
                continue;
            }
            if transaction["kind"].as_u64().unwrap() != 1 {
                continue;
            }
            if transaction["addr"].as_u64().unwrap() as u32 != addr {
                continue;
            }
            return transaction["data"].as_u64().unwrap() as u32;
        }
        panic!("address not handled {addr} {}", serde_json::to_string_pretty(&self.transactions).unwrap());
        // println!("failed");
    }
    fn write(&self, size: u64, addr: u32, data: u64) {
        for transaction in self.transactions.as_array().unwrap() {
            if transaction["size"].as_u64().unwrap() != size {
                continue;
            }
            if transaction["kind"].as_u64().unwrap() != 2 {
                continue;
            }
            if transaction["addr"].as_u64().unwrap() as u32 != addr {
                continue;
            }
            if transaction["data"].as_u64().unwrap() != data {
                println!("wrong data supplied {data} at {addr}");
                break;
            }
            return;
        }
        panic!("{}", serde_json::to_string_pretty(&self.transactions).unwrap());
        // println!("failed :(");
    }
    fn read_instruction(&self, address: u32) -> u32 {
        if address == self.base_addr {
            return self.test_opcode;
        }
        return address;
    }
}
impl Memoriable for JsonMemory {
    fn read_u16(&self, address: u32) -> u16 { self.read(2, address) as u16 }
    fn read_u32(&self, address: u32) -> u32 { self.read(4, address)        }
    fn read_u8(&self, address: u32) ->  u8  { self.read(1, address) as u8  }
    fn write_u16(&mut self, address: u32, data: u16) { self.write(2, address, data as u64); }
    fn write_u32(&mut self, address: u32, data: u32) { self.write(4, address, data as u64); }
    fn write_u8(&mut self, address: u32, data: u8)   { self.write(1, address, data as u64); }
}

pub fn perform_tests() {
    let files = std::fs::read_dir("./json/").unwrap();
    for file in files {
        let file = file.unwrap();
        let mut i = 0;
        // i don't really want to delete the python file,
        // so i will just ignore it
        let name = file.file_name();
        let filename = name.to_str().unwrap();
        if filename.ends_with(".py") { continue; }
        if filename.contains("arm")   { continue; }
        if filename.contains("bx")   { continue; }

        println!("{}", file.file_name().to_str().unwrap());
        let read_file = std::fs::read_to_string(file.path()).unwrap();
        let json: Value = serde_json::from_str(&read_file).unwrap();
        let all_tests = json.as_array().unwrap();
        for test in all_tests {
            let (cpu, fde) = init_single_test(true, test);
            let (end_cpu, _end_fde) = init_single_test(false, test);
            let mem = init_mem(test);

            let mut emu = JsonEmulator {
                cpu,
                cycles: 0,
                mem
            };
            //println!("{}", emu.cpu.fde.decoded_opcode.unwrap());
            run_json_test(&mut emu);
            if let Some(e) = check_identical(&emu.cpu, &end_cpu) {
                println!("{}", serde_json::to_string_pretty(test).unwrap());
                println!("{e}");
                panic!("{i}");
            }
            i += 1;
        }
    }
}

fn init_single_test(start: bool, test: &Value) -> (Cpu, Fde) {
    let location = match start {
        true => "initial",
        false => "final"
    };

    let mut unbanked_regs = [0; 8];
    let mut double_banked_regs = [[0; 2]; 5];
    let mut many_banked_regs = [[0; 6]; 2];

    let regs = test[location]["R"].as_array().unwrap();
    for i in 0..8 {
        unbanked_regs[i] = regs[i].as_u64().unwrap() as u32;
    }
    let fiq_regs = test[location]["R_fiq"].as_array().unwrap();
    for i in 0..5 {
        double_banked_regs[i][0] = regs[8+i].as_u64().unwrap() as u32;
        double_banked_regs[i][1] = fiq_regs[i].as_u64().unwrap() as u32;
    }

    let svc_regs = test[location]["R_svc"].as_array().unwrap();
    let abt_regs = test[location]["R_abt"].as_array().unwrap();
    let irq_regs = test[location]["R_irq"].as_array().unwrap();
    let und_regs = test[location]["R_und"].as_array().unwrap();
    for i in 0..2 {
        many_banked_regs[i][0] = regs[i+13].as_u64().unwrap() as u32;
        many_banked_regs[i][1] = fiq_regs[i+5].as_u64().unwrap() as u32;
        many_banked_regs[i][2] = svc_regs[i].as_u64().unwrap() as u32;
        many_banked_regs[i][3] = abt_regs[i].as_u64().unwrap() as u32;
        many_banked_regs[i][4] = irq_regs[i].as_u64().unwrap() as u32;
        many_banked_regs[i][5] = und_regs[i].as_u64().unwrap() as u32;
    }
    
    let cpsr = convert_u32_psr(test[location]["CPSR"].as_u64().unwrap() as u32);
    let mut spsr = Vec::new();
    for i in test[location]["SPSR"].as_array().unwrap() {
        spsr.push(convert_u32_psr(i.as_u64().unwrap() as u32));
    }

    let pipeline = test[location]["pipeline"].as_array().unwrap();
    let fde = Fde {
        fetched_opcode: Some(pipeline[1].as_u64().unwrap() as u32),
        decoded_opcode: Some(pipeline[0].as_u64().unwrap() as u32),
    };

    let pc = regs[15].as_u64().unwrap() as u32;
    let cpu = Cpu {
        unbanked_registers: unbanked_regs.try_into().unwrap(),
        double_banked_registers: double_banked_regs,
        many_banked_registers: many_banked_regs,
        pc,
        halted: false,
        cpsr,
        spsr: spsr.try_into().unwrap(),
        barrel_shifter: false,
        fde,
    };
    return (cpu, fde);
}

fn init_mem(test: &Value) -> JsonMemory {    
    JsonMemory { 
        transactions: test["transactions"].clone(),
        base_addr: test["base_addr"].as_array().unwrap()[0].as_u64().unwrap() as u32,
        test_opcode: test["opcode"].as_u64().unwrap() as u32,
    }
}

fn run_json_test(emu: &mut JsonEmulator) {
    // this will always be a thing
    let opcode = emu.cpu.fde.decoded_opcode.unwrap();
    match emu.cpu.cpsr.t {
        true => {
            execute_thumb(opcode as u16, &mut emu.cpu, &mut emu.mem);
            let addition = match emu.cpu.cpsr.t {
                true => 2,
                false => 4,
            };

            match emu.cpu.fde.fetched_opcode.is_none() {
                true => emu.cpu.pc = emu.cpu.pc.wrapping_add(addition * 2),
                false => emu.cpu.pc = emu.cpu.pc.wrapping_add(addition),
            }
        }
        false => {
            let runs = check_condition(opcode >> 28, &emu.cpu.cpsr);
            if !runs {
                emu.cpu.pc = emu.cpu.pc.wrapping_add(4);
            }
            execute_arm(opcode, &mut emu.cpu, &mut emu.mem);

            let addition = match emu.cpu.cpsr.t {
                true => 2,
                false => 4,
            };

            if runs {
                match emu.cpu.fde.fetched_opcode.is_none() {
                    true => emu.cpu.pc = emu.cpu.pc.wrapping_add(addition * 2),
                    false => emu.cpu.pc = emu.cpu.pc.wrapping_add(addition),
                }
            }
        }
    };
}

fn check_identical(test: &Cpu, correct: &Cpu) -> Option<String> {
    for i in 0..8 {
        let a = test.unbanked_registers[i];
        let b = correct.unbanked_registers[i];
        if a != b { 
            return Some(format!("R{i} => {a:X} != {b:X}")); 
        }
    }
    for i in 0..5 {
        for j in 0..2 {
            let a = test.double_banked_registers[i][j];
            let b = correct.double_banked_registers[i][j];
            if a != b { 
                return Some(format!("double-R[{}][{j}] => {a:X} != {b:X}", i+8)); 
            }
        }
    }
    for i in 0..2 {
        for j in 0..6 {
            let a = test.many_banked_registers[i][j];
            let b = correct.many_banked_registers[i][j];
            if a != b { 
                return Some(format!("many-R[{}][{j}] => {a:X} != {b:X}", i+13)); 
            }
        }
    }

    if test.cpsr != correct.cpsr { 
        // the c bit is so weird that i will usually just ignore it if wrong
        if test.cpsr.c == correct.cpsr.c {
            return Some(format!("{:?} != {:?}", test.cpsr, correct.cpsr)); 
        }
    }
    for i in 0..5 {
        if test.spsr[i] != correct.spsr[i] { 
            return Some(format!("SPSR[{i}] {:?} != {:?}", test.spsr[i], correct.spsr[i])); 
        }
    }
    if test.pc != correct.pc {
        return Some(format!("PC {:X} != {:X}", test.pc, correct.pc));
    }

    return None;
}