use crate::memory::{Memory};
use crate::display::Display;
use crate::emulator::{FONT_ADDRESS};

pub struct CPU {
    pub pc: u16, // Program Counter
    pub sp: u8,  // Stack Pointer
    pub stack: [u16; 16], // Stack for subroutine calls
    pub v: [u8; 16], // General Purpose Registers (from 0 to F)
    pub i: u16, // Index Register
    pub delay_timer: u8, // Delay Timer
    pub sound_timer: u8, // Sound Timer
}

// In this mode, the CPU will set VX = VY when left and right shifting
const SHIFT_SET_MODE: bool = true;

impl CPU {
    // Run a rom
    pub fn new(program_counter: u16) -> Self {
        CPU {
            pc: program_counter,
            sp: 0,
            stack: [0; 16],
            v: [0; 16],
            i: 0,
            delay_timer: 0,
            sound_timer: 0,
        }
    }

    pub fn _set_program_counter(&mut self, pc: u16) {
        self.pc = pc;
    }

    pub fn decode(&mut self, memory: &mut Memory, display: &mut Display, keys: &[bool; 16]) {
        // Opcode is a 16 bit value with two bytes
        let opcode: u16 = (memory.data[self.pc as usize] as u16) << 8 | memory.data[self.pc as usize + 1] as u16;
        // There are 4 nibbles
        let nibbles: [u8; 4] = [
            (opcode >> 12) as u8, // First nibble
            (opcode >> 8 & 0x0F) as u8, // Second nibble
            (opcode >> 4 & 0x0F) as u8, // Third nibble
            (opcode & 0x0F) as u8, // Fourth nibble
        ];
        match nibbles {
            // Clear Screen 
            [0x0, 0x0, 0xE, 0x0] => {
                display.clear();
            }
            // Jump to address NNN
            [0x1, _, _, _] => {
                let address = ((nibbles[1] as u16) << 8) | ((nibbles[2] as u16) << 4) | nibbles[3] as u16;
                self.pc = address;
                // Do not increment PC here, as it is set directly
                self.pc -= 2;
            }
            // Call subroutine at NNN
            [0x2, _, _, _] => {
                let address: u16 = ((nibbles[1] as u16) << 8) | ((nibbles[2] as u16) << 4) | nibbles[3] as u16;
                if self.sp < 15 {
                    self.stack[self.sp as usize] = self.pc;
                    self.sp += 1;
                    // Set PC to address, minus 2 to account for increment
                    self.pc = address - 0x2; 
                } else {
                    println!("Stack overflow: Cannot call subroutine");
                }
            }
            // Return from subroutine
            [0x0, 0x0, 0xE, 0xE] => {
                if self.sp > 0 {
                    // Set PC to address at the top
                    self.pc = self.stack[self.sp as usize - 1];
                    // Pop stack pointer
                    self.sp -= 1;

                } else {
                    println!("Stack underflow: Cannot return from subroutine");
                }
            }
            // 3XNN: Skip Next Instruction if VX == NN
            [0x3, _, _, _] => {
                let vx = nibbles[1] as usize;
                let val_vx = self.v[vx];
                let nn = ((nibbles[2] as u16) << 4) | nibbles[3] as u16;
                if vx < 16 && val_vx == nn as u8 {
                    self.pc += 2; // Skip next instruction
                }
            }
            // 4XNN: Skip Next Instruction if VX != NN
            [0x4, _, _, _] => {
                let vx = nibbles[1] as usize;
                let val_vx = self.v[vx];
                let nn = ((nibbles[2] as u16) << 4) | nibbles[3] as u16;
                if vx < 16 && val_vx != nn as u8 {
                    self.pc += 2; // Skip next instruction
                }
            }
            // 5XY0: Skip Next Instruction if VX == VY
            [0x5, _, _, 0x0] => {
                let vx = nibbles[1] as usize;
                let vy = nibbles[2] as usize;
                let val_vx = self.v[vx];
                let val_vy = self.v[vy];
                if vx < 16 && vy < 16 && val_vx == val_vy{
                    self.pc += 2; // Skip next instruction
                }
            }
            // 9XY0: Skip Next Instruction if VX != VY
            [0x9, _, _, 0x0] => {
                let vx = nibbles[1] as usize;
                let vy = nibbles[2] as usize;
                let val_vx = self.v[vx];
                let val_vy = self.v[vy];
                if vx < 16 && vy < 16 && val_vx != val_vy {
                    self.pc += 2; // Skip next instruction
                }
            }
            // 6XNN: Set VX to NN
            [0x6, _, _, _] => {
                let vx = nibbles[1] as usize;
                let nn = ((nibbles[2] as u16) << 4) | nibbles[3] as u16;
                if vx < 16 {
                    self.v[vx] = nn as u8;
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // 7XNN: Add NN to VX
            [0x7, _, _, _] => {
                // Add NN to Vx
                let vx = nibbles[1] as usize;
                let nn = ((nibbles[2] as u16) << 4) | nibbles[3] as u16;
                if vx < 16 {
                    self.v[vx] = self.v[vx].wrapping_add(nn as u8);
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // Arithmetic
            // 8XY0: Set VX to VY
            [0x8, _, _, 0x0] => {
                let vx = nibbles[1] as usize;
                let vy = nibbles[2] as usize;
                if vx < 16 && vy < 16 {
                    self.v[vx] = self.v[vy];
                } else {
                    println!("Invalid register index: {} or {}", vx, vy);
                }
            }
            // 8XY1: Set VX to VX OR VY
            [0x8, _, _, 0x1] => {
                let vx = nibbles[1] as usize;
                let vy = nibbles[2] as usize;
                if vx < 16 && vy < 16 {
                    self.v[vx] |= self.v[vy];
                } else {
                    println!("Invalid register index: {} or {}", vx, vy);
                }
            }
            // 8XY2: Set VX to VX AND VY
            [0x8, _, _, 0x2] => {
                let vx = nibbles[1] as usize;
                let vy = nibbles[2] as usize;
                if vx < 16 && vy < 16 {
                    self.v[vx] &= self.v[vy];
                } else {
                    println!("Invalid register index: {} or {}", vx, vy);
                }
            }
            // 8XY3: Set VX to VX XOR VY
            [0x8, _, _, 0x3] => {
                let vx = nibbles[1] as usize;
                let vy = nibbles[2] as usize;
                if vx < 16 && vy < 16 {
                    self.v[vx] ^= self.v[vy];
                } else {
                    println!("Invalid register index: {} or {}", vx, vy);
                }
            }
            // 8XY4: Set VX to VX + VY, VF is set to carry
            [0x8, _, _, 0x4] => {
                let vx = nibbles[1] as usize;
                let vy = nibbles[2] as usize;
                if vx < 16 && vy < 16 {
                    let (result, carry) = self.v[vx].overflowing_add(self.v[vy]);
                    self.v[vx] = result;
                    self.v[0xF] = carry as u8;
                } else {
                    println!("Invalid register index: {} or {}", vx, vy);
                }
            }
            // 8XY5: Set VX to VX - VY, VF is set to NOT borrow
            [0x8, _, _, 0x5] => {
                let vx = nibbles[1] as usize;
                let vy = nibbles[2] as usize;
                if vx < 16 && vy < 16 {                    
                    let (result, borrow) = self.v[vx].overflowing_sub(self.v[vy]);
                    self.v[vx] = result;

                    self.v[0xF] = (!borrow) as u8; // Set VF to 1 if no borrow, 0 if borrow
                    
                } else {
                    println!("Invalid register index: {} or {}", vx, vy);
                }
            }
            // 8XY6: Shift VX right by 1, VF is set to the least significant bit of VX
            [0x8, _, _, 0x6] => {
                let vx: u8 = nibbles[1] as u8;
                let vy: u8 = nibbles[2] as u8;
                if vx < 16 {
                    if SHIFT_SET_MODE {
                        self.v[vx as usize] = self.v[vy as usize];
                    }
                    let bit: u8 = self.v[vx as usize] & 0x01; // Get the least significant bit
                    self.v[vx as usize] >>= 1; // Shift right
                    self.v[0xF] = bit; // Set VF to LSB of VX
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // 8XY7: Set VX to VY - VX
            [0x8, _, _, 0x7] => {
                let vx = nibbles[1] as usize;
                let vy = nibbles[2] as usize;
                if vx < 16 && vy < 16 {
                    let (result, borrow) = self.v[vy].overflowing_sub(self.v[vx]);
                    self.v[vx] = result;

                    self.v[0xF] = (!borrow) as u8; // Set VF to 1 if no borrow, 0 if borrow
                    
                } else {
                    println!("Invalid register index: {} or {}", vx, vy);
                }
            }
            [0x8, _, _, 0xE] => {
                // Shift VX left by 1, VF is set to the most significant bit of VX
                let vx: u8 = nibbles[1] as u8;
                let vy: u8 = nibbles[2] as u8;
                if vx < 16 {
                    if SHIFT_SET_MODE {
                        self.v[vx as usize] = self.v[vy as usize];
                    }
                    let bit: u8 = (self.v[vx as usize] & 0x80) >> 7; // Get the most significant bit
                    self.v[vx as usize] <<= 1; // Shift left
                    self.v[0xF] = bit; // Set VF to MSB of VX
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            
            // CXNN: Random
            [0xC, _, _, _] => {
                let vx = nibbles[1] as usize;
                let nn = ((nibbles[2] as u16) << 4) | nibbles[3] as u16;
                if vx < 16 {
                    // Generate a random number and AND it with NN
                    let random_byte = rand::random::<u8>();
                    self.v[vx] = random_byte & nn as u8;
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            [0xA, _, _, _] => {
                // Set index regier I to NNN
                self.i = ((nibbles[1] as u16) << 8) | ((nibbles[2] as u16) << 4) | nibbles[3] as u16;
            }
            [0xD, _, _, _] => {
                // Draw sprite at Vx, Vy with height N
                let vx = nibbles[1] as usize;
                let vy = nibbles[2] as usize;
                // Get the x and y coordinates
                let x = self.v[vx] as usize % display.width as usize;
                let y = self.v[vy] as usize % display.height as usize;
                // Set VF to 0
                self.v[0xF] = 0;
                let n = nibbles[3] as usize;
                for row in 0..n {
                    if row >= display.height as usize {
                        break;
                    }
                    // Get the nth byte of sprite data counting from the memory address in the I register
                    let sprite_byte: u8 = memory.data[(self.i as usize + row) % memory.data.len()];
                    for col in 0..8 {
                        // Check if the pixel is set
                        let pixel = (sprite_byte >> (7 - col)) & 0x01;
                        if col > display.width as usize {
                            break;
                        }
                        if pixel == 1 {
                            // Set the pixel on the display
                            display.pixels[y + row][x + col] = true;
                        }
                    }
                    
                }
            }
            // EX9E Skip if Key pressed
            [0xE, _, 0x9, 0xE] => {
                let vx = nibbles[1] as usize;
                let key = self.v[vx];
                if vx < 16 && keys[key as usize] {
                    self.pc += 2; // Skip next instruction
                }
            }
            // EXA1 Skip if key not pressed
            [0xE, _, 0xA, 0x1] => {
                let vx = nibbles[1] as usize;
                let key = self.v[vx];
                if vx < 16 && !keys[key as usize] {
                    self.pc += 2; // Skip next instruction
                }
            }
            // FX07: Set VX to the current value of delay timer
            [0xF, _, 0x0, 0x7] => {
                let vx = nibbles[1] as usize;
                if vx < 16 {
                    self.v[vx] = self.delay_timer;
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // FX15: Sets the delay timer to VX
            [0xF, _, 0x1, 0x5] => {
                let vx = nibbles[1] as usize;
                if vx < 16 {
                    self.delay_timer = self.v[vx];
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // FX18: Sets the sound timer to VX
            [0xF, _, 0x1, 0x8] => {
                let vx = nibbles[1] as usize;
                if vx < 16 {
                    self.sound_timer = self.v[vx];
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // FX1E: Adds VX to I
            [0xF, _, 0x1, 0xE] => {
                let vx = nibbles[1] as usize;
                if vx < 16 {
                    self.i += self.v[vx] as u16;
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // FX0A: Get Key
            [0xF, _, 0x0, 0xA] => {
                let mut key = None;
                for (i, &pressed) in keys.iter().enumerate() {
                    if pressed {
                        key = Some(i as u8);
                        break;
                    }
                }
                if key.is_some() {
                    // Set VX to the key pressed
                    let vx: usize = nibbles[1] as usize;
                    if vx < 16 {
                        self.v[vx] = key.unwrap();
                    } else {
                        println!("Invalid register index: {}", vx);
                    }
                }
            }
            // FX29: Load font Character
            [0xF, _, 0x2, 0x9] => {
                let vx = nibbles[1] as usize;
                if vx < 16 {
                    // Set I to the address of the font character
                    self.i = FONT_ADDRESS as u16 + (self.v[vx] as u16);
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // FX33: Binary-Coded decimal conversion
            [0xF, _, 0x3, 0x3] => {
                let vx = nibbles[1] as usize;
                let val = self.v[vx];
                if vx < 16 {
                    // Store the hundreds digit
                    memory.data[self.i as usize] = val / 100;
                    // Store the tens digit
                    memory.data[self.i as usize + 1] = (val / 10) % 10;
                    // Store the units digit
                    memory.data[self.i as usize + 2] = val % 10;
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // FX55: Store registers V0 to VX in memory starting at address I
            [0xF, _, 0x5, 0x5] => {
                let vx = nibbles[1] as usize;
                if vx < 16 {
                    for i in 0..=vx {
                        memory.data[(self.i + i as u16) as usize] = self.v[i];
                    }
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // FX65: Read registers V0 to VX from memory starting at address I
            [0xF, _, 0x6, 0x5] => {
                let vx = nibbles[1] as usize;
                if vx < 16 {
                    for i in 0..=vx {
                        self.v[i] = memory.data[(self.i + i as u16) as usize];
                    }
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            _ => {
                // Handle other opcodes
                println!("Unknown opcode: {:04X}", opcode);
            }
            
        }
        // Increment the program counter
        self.pc += 2;
    }
}

