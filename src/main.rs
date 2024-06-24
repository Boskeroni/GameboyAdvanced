mod processor;
mod decode;
mod memory;


fn main() {
    let cpu_regs = processor::CpuRegisters {
        pc: 0,
        unbanked_registers: [0, 0, 0, 0, 0, 0, 0 ,0],
        double_banked_registers: [0, 0,  0, 0,  0, 0,  0, 0,  0, 0],
        many_banked_registers: [0, 0, 0, 0, 0, 0,  0, 0, 0, 0, 0, 0],
    };
    let memory = memory::create_memory("golden_sun.gba");

    // the main loop of the program
    loop {
        
    }
}