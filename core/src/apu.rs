enum ApuRegisters {
    Sound1CntL = 0x4000060,
    Sound1CntH = 0x4000062,
    Sound2CntX = 0x4000064,

    // tone
    Sound2CntL = 0x4000068,
    Sount2CntH = 0x400006C,

    // wave output
    Sound3CntL = 0x4000070,
    Sound3CntH = 0x4000072,
    Sound3CntX = 0x4000074,
    Sound3Ram0L = 0x4000090,
    Sound3Ram0H = 0x4000092,

    // noise
    Sound4CntL = 0x4000078,
    Sound4CntH = 0x400007C,

    // Sound Control Registers
    SoundCntL = 0x4000080,
    SoundCntH = 0x4000082,
    SountCntX = 0x4000084,
    SoundBias = 0x4000088,

    // DMA sound
    SoundChannel = 0x40000A0,
}

pub fn tick_apu() {
    
}