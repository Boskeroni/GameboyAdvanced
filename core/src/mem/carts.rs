use std::fs;
use crate::mem::{bus::MemoryRegion, split_memory_address};

trait Cartridge {
    // this would have been already checked to see which type of cart it is
    fn new(file: String) -> Self;
    
    // SRAM has a limitation where reading can only occur when code is being
    // executed in WRAM, not sure if this exists anywhere else
    fn read(&mut self, address: u32, from: MemoryRegion, is_8_bit: bool) -> Option<u8>;
    fn write(&mut self, address: u32, data: u8, is_8_bit: bool);
}

// this is handled the same as I am already doing it
// its just the SRAM that is different, given its name and all
struct Sram {
    rom: Vec<u8>,
    sram: [u8; 0x8000],
}
impl Cartridge for Sram {
    fn new(file: String) -> Self {
        let rom = fs::read(file).unwrap();

        Self {
            rom,
            sram: [0; 0x8000],
        }
    }

    fn read(&mut self, address: u32, from: MemoryRegion, is_8_bit: bool) -> Option<u8> {
        let (upp, low) = split_memory_address(address);
        if upp == 0xE {
            assert!(is_8_bit);
            if from != MemoryRegion::WramBoard && from != MemoryRegion::WramChip { return None; }
            // TODO: check if it needs to wrap
            return Some(self.sram[low]);
        }   
        
        assert!(upp >= 0x8 && upp <= 0xD);
        Some(self.rom[low])
    }
    fn write(&mut self, address: u32, data: u8, is_8_bit: bool) {
        assert!(is_8_bit);

        let (upp, low) = split_memory_address(address);
        assert!(upp == 0xE);
        self.sram[low] = data;
    }
}

const EEPROM_READ_END: usize = 68;
const EEPROM_WRITE_END: usize = 64;
enum EepRomState {
    TransferRequest,
    TransferData,
}
struct EepRom {
    rom: Vec<u8>,

    // since bit transfers are how this communicates, I will just store
    // everything here as booleans
    eeprom: [bool; 0x8000],
    is_8kb_eeprom: bool,
    is_32mb_rom: bool,
    address: u32,
    state: EepRomState,
    is_reading: bool,

    // when this reaches 68, then the transfer should be completed
    amount_completed: usize,
    previous_write: bool,
}
impl Cartridge for EepRom {
    fn new(file: String) -> Self {
        todo!();
    }
    fn read(&mut self, address: u32, _from: MemoryRegion, _is_8_bit: bool) -> Option<u8> {
        let reading_eeprom: bool;
        let (upp, low) = split_memory_address(address);

        match self.is_32mb_rom {
            true => reading_eeprom = upp % 2 == 1 && low >= 0xFFFF00,
            false => reading_eeprom = upp == 0xD,
        }

        if !reading_eeprom {
            return Some(self.rom[low]);
        }

        assert!(self.is_reading);
        
        let base_address = self.address as usize * 0x40;
        self.amount_completed += 1;
        if self.amount_completed <= 4 {
            return Some(0);
        }
        return Some(self.eeprom[base_address + (self.amount_completed - 4)] as u8)
    }
    fn write(&mut self, address: u32, data: u8, _is_8_bit: bool) {
        let writing_eeprom: bool;
        let (upp, low) = split_memory_address(address);
        
        match self.is_32mb_rom {
            true => writing_eeprom = upp % 2 == 1 && low >= 0xFFFF00,
            false => writing_eeprom = upp == 0xD, // 'can be accessed anywhere between 0xD000000..=0xDFFFFF'
        }

        if !writing_eeprom {
            // this might have to be a panic
            println!("EEPROM: attempted to write {data:X} to address {address:X}");
            return;
        }

        // this is all the EEPROM would be able to see normally
        let masked_data = data & 0x1;

        use EepRomState::*;
        match self.state {
            TransferRequest => {
                // for now just add each bit onto the address, deal with problems later
                self.address <<= 1;
                self.address |= masked_data as u32;

                // the only way that the unconverted address can be two is if
                // a read request was made
                if self.address == 0x2 {
                    self.is_reading = false;
                } else if self.address == 0x3 {
                    self.is_reading = true;
                }

                let checked_bit = match (self.is_reading, self.is_8kb_eeprom) {
                    (true, true) => 16,
                    (true, false) => 8,
                    (false, true) => 15,
                    (false, false) => 7,
                };

                let finished = match self.is_8kb_eeprom {
                    true => (self.address >> checked_bit) & 1 == 1,
                    false => (self.address >> checked_bit) & 1 == 1,
                };

                // the address has been fully recieved
                if finished {
                    self.state = EepRomState::TransferData;

                    // need to modify the data recieved to be the actual address
                    self.address >>= 1;
                    let top_bit = 31 - self.address.leading_zeros();
                    let mask = 0b11 << (top_bit - 1);
                    self.address &= mask;
                }
            }
            TransferData => {
                assert!(!self.is_reading);

                // 64 bit offset == 0x40
                let base_address = self.address as usize * 0x40;
                self.eeprom[base_address + self.amount_completed] = masked_data == 1;

                self.amount_completed += 1;
                if self.amount_completed == EEPROM_WRITE_END {
                    // transfer is finished
                    self.state = EepRomState::TransferRequest;
                }
            }
        }
    }
}
struct FlashRom;
impl Cartridge for FlashRom {
    fn new(file: String) -> Self {
        todo!();
    }
    fn read(&mut self, address: u32, from: MemoryRegion, is_8_bit: bool) -> Option<u8> {
        todo!();
    }
    fn write(&mut self, address: u32, data: u8, is_8_bit: bool) {
        todo!();
    }
}
