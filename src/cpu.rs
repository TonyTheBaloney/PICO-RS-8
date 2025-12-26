use std::sync::{Arc, Mutex};

use crate::memory::{Memory};
use crate::display::Display;
use crate::emulator::{FONT_ADDRESS};



pub struct CPU {
    pub pc: u16, // Program Counter
    pub sp: u8,  // Stack Pointer
    pub stack: [u16; 16], // Stack for subroutine calls
    pub v: [u8; 16], // General Purpose Registers (from 0 to F)
    pub i: u16, // Index Register
    pub delay_timer: Arc<Mutex<u8>>, // Delay Timer
    pub sound_timer: Arc<Mutex<u8>>, // Sound Timer
}

// In this mode, the CPU will set VX = VY when left and right shifting
const SHIFT_SET_MODE: bool = true;
// In this mode, the CPU will add VX to NNN in the BNNN instruction
const JUMP_VX_MODE: bool = false;

impl CPU {
    // Run a rom
    pub fn new(program_counter: u16) -> Self {
        let delay_timer: Arc<Mutex<u8>> = Arc::new(Mutex::new(0));
        let delay_timer_thread: Arc<Mutex<u8>> = Arc::clone(&delay_timer);
        let sound_timer: Arc<Mutex<u8>> = Arc::new(Mutex::new(0));
        let sound_timer_thread: Arc<Mutex<u8>> = Arc::clone(&sound_timer);

        std::thread::spawn(move || {
            loop {
                // Delay Timer Lock
                {
                    let mut delay_timer: std::sync::MutexGuard<'_, u8> = delay_timer_thread.lock().unwrap();
                    if *delay_timer > 0 {
                        *delay_timer -= 1;
                    }
                }
                // Sound Timer Lock
                {
                    let mut sound_timer: std::sync::MutexGuard<'_, u8> = sound_timer_thread.lock().unwrap();
                    if *sound_timer > 0 {
                        *sound_timer -= 1;
                    }
                }
                
                std::thread::sleep(std::time::Duration::from_millis(1000 / 60)); // Approximately 60Hz
            }
        });
        
        // Create a thread that seperately decrements the timers at 60Hz
        CPU {
            pc: program_counter,
            sp: 0,
            stack: [0; 16],
            v: [0; 16],
            i: 0,
            delay_timer: delay_timer,
            sound_timer: sound_timer,
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
        println!("Executing Opcode: {:04X} at PC: {:04X}", opcode, self.pc);
        match nibbles {
            // 00E0: Clear the display
            [0x0, 0x0, 0xE, 0x0] => {
                display.clear();
            }
            // 00EE: Return from a subroutine
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
            // 1NNN: Jump to location NNN
            [0x1, _, _, _] => {
                let address = ((nibbles[1] as u16) << 8) | ((nibbles[2] as u16) << 4) | nibbles[3] as u16;
                self.pc = address;
                // Do not increment PC here, as it is set directly
                self.pc -= 2;
            }
            // 2NNN: Call Subroutine at NNN
            [0x2, _, _, _] => {
                let address: u16 = ((nibbles[1] as u16) << 8) | ((nibbles[2] as u16) << 4) | nibbles[3] as u16;
                if self.sp < 15 {
                    self.stack[self.sp as usize] = self.pc;
                    self.sp += 1;
                    // Set PC to address, minus 2 to account for increment
                    self.pc = address - 2; 
                } else {
                    println!("Stack overflow: Cannot call subroutine");
                }
            }
            
            // 3XNN: Skip Next Instruction if VX == NN
            [0x3, _, _, _] => {
                let vx: usize = nibbles[1] as usize;
                let val_vx: u8 = self.v[vx];
                let nn: u8 = ((nibbles[2]) << 4) | nibbles[3];
                if vx < 16 && val_vx == nn {
                    self.pc += 2; // Skip next instruction
                }
            }
            // 4XNN: Skip Next Instruction if VX != NN
            [0x4, _, _, _] => {
                let vx: usize = nibbles[1] as usize;
                let val_vx: u8 = self.v[vx];
                let nn: u8 = ((nibbles[2]) << 4) | nibbles[3];
                if vx < 16 && val_vx != nn {
                    self.pc += 2; // Skip next instruction
                }
            }
            // 5XY0: Skip Next Instruction if VX == VY
            [0x5, _, _, 0x0] => {
                let vx: usize = nibbles[1] as usize;
                let vy: usize = nibbles[2] as usize;
                let val_vx: u8 = self.v[vx];
                let val_vy: u8 = self.v[vy];
                if vx < 16 && vy < 16 && val_vx == val_vy{
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
                let vx: usize = nibbles[1] as usize;
                let vy = nibbles[2] as usize;
                self.v[0xF] = 0;
                if vx < 16 && vy < 16 {
                    self.v[vx] |= self.v[vy];
                } else {
                    println!("Invalid register index: {} or {}", vx, vy);
                }
            }
            // 8XY2: Set VX to VX AND VY
            [0x8, _, _, 0x2] => {
                let vx: usize = nibbles[1] as usize;
                let vy: usize = nibbles[2] as usize;
                self.v[0xF] = 0;
                if vx < 16 && vy < 16 {
                    self.v[vx] &= self.v[vy];
                } else {
                    println!("Invalid register index: {} or {}", vx, vy);
                }
            }
            // 8XY3: Set VX to VX XOR VY
            [0x8, _, _, 0x3] => {
                let vx: usize = nibbles[1] as usize;
                let vy: usize = nibbles[2] as usize;
                self.v[0xF] = 0;
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
            // 9XY0: Skip Next Instruction if VX != VY
            [0x9, _, _, 0x0] => {
                let vx: usize = nibbles[1] as usize;
                let vy: usize = nibbles[2] as usize;
                let val_vx: u8 = self.v[vx];
                let val_vy: u8 = self.v[vy];
                if vx < 16 && vy < 16 && val_vx != val_vy {
                    self.pc += 2; // Skip next instruction
                }
            }

            // ANNN: Set I to address NNN
            [0xA, _, _, _] => {
                // Set index regier I to NNN
                self.i = ((nibbles[1] as u16) << 8) | ((nibbles[2] as u16) << 4) | nibbles[3] as u16;
            }

            // BNNN: Jump to location NNN + V0.
            [0xB, _, _, _] => {
                let nnn: u16 = ((nibbles[1]  as u16) << 8) | ((nibbles[2] as u16) << 4) | nibbles[3] as u16;
                
                if JUMP_VX_MODE {
                    let x: usize = nibbles[1] as usize;
                    let v_x: u16 = self.v[x] as u16;
                    self.pc = nnn + v_x - 2; // This adjusts for increment later    
                }else {
                    // Original CHIP-8 behavior
                    let v0: u8 = self.v[0];
                    self.pc = nnn + (v0 as u16) - 2; // This adjusts for increment later
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
            // DXYN: Draw Sprite
            // Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
            [0xD, _, _, _] => {
                // Draw sprite at Vx, Vy with height N
                let vx: usize = nibbles[1] as usize;
                let vy: usize = nibbles[2] as usize;
                let n: usize = nibbles[3] as usize;

                // Get the x and y coordinates on the screen
                let mut x: usize = self.v[vx] as usize;
                let mut y: usize = self.v[vy] as usize;

                // Bounds check. If the sprite is drawn outside the display, we ignore it
                if x >= display.width as usize || y >= display.height as usize {
                    x %= display.width as usize;
                    y %= display.height as usize;
                }

                // Set VF to 0
                self.v[0xF] = 0;
                for row in 0..n {
                    if row > display.height as usize {
                        break;
                    }
                    // Get the nth byte of sprite data counting from the memory address in the I register
                    let sprite_byte: u8 = memory.data[(self.i as usize + row) % memory.data.len()];

                    for col in 0..8 {
                        // Check if the pixel is set at that col in the sprite byte
                        let pixel: bool = ((sprite_byte >> (7 - col)) & 0x01) == 1;
                        if col > display.width as usize {
                            break;
                        }
                        // Sprites are XORed onto the existing screen.
                        let display_x: usize = (x + col) % display.width as usize;
                        let display_y: usize = (y + row) % display.height as usize;
                        // If this causes any pixels to be erased, VF is set to 1, otherwise it is set to 0.
                        if display.pixels[display_y][display_x] && pixel {
                            self.v[0xF] = 1; // If our sprite pixel is on, and the display pixel is on the XOR will cause an overwrite
                        }
                        // XOR the pixel
                        display.pixels[display_y][display_x] ^= pixel;
                    }
                    
                }
            }
            // EX9E Skip next instruction if key with the value of Vx is pressed.
            [0xE, _, 0x9, 0xE] => {
                let vx: usize = nibbles[1] as usize;
                let key: u8 = self.v[vx];
                if vx < 16 && keys[key as usize] {
                    self.pc += 2; // Skip next instruction
                }
            }
            // EXA1 Skip next instruction if key with the value of Vx is not pressed.
            [0xE, _, 0xA, 0x1] => {
                let vx = nibbles[1] as usize;
                let key = self.v[vx];
                if vx < 16 && !keys[key as usize] {
                    self.pc += 2; // Skip next instruction
                }
            }
            // FX07: Set Vx = delay timer value.
            [0xF, _, 0x0, 0x7] => {
                let vx = nibbles[1] as usize;
                if vx < 16 {
                    self.v[vx] = self.delay_timer.lock().unwrap().clone();
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // FX0A: Wait for a key press, store the value of the key in Vx
            [0xF, _, 0x0, 0xA] => {
                let mut key: Option<u8> = None;
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
                }else {
                    // Repeat this instruction until a key is pressed
                    self.pc -= 2;
                }

            }
            // FX15: Sets the delay timer to VX
            [0xF, _, 0x1, 0x5] => {
                let vx: usize = nibbles[1] as usize;
                if vx < 16 {
                    let mut delay_timer_thread: std::sync::MutexGuard<'_, u8> = self.delay_timer.lock().unwrap();
                    *delay_timer_thread = self.v[vx];
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // FX18: Sets the sound timer to VX
            [0xF, _, 0x1, 0x8] => {
                let vx: usize = nibbles[1] as usize;
                if vx < 16 {
                    let mut sound_timer_thread: std::sync::MutexGuard<'_, u8> = self.sound_timer.lock().unwrap();
                    *sound_timer_thread = self.v[vx];
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // FX1E: Adds VX to I
            [0xF, _, 0x1, 0xE] => {
                let vx: usize = nibbles[1] as usize;
                if vx < 16 {
                    self.i += self.v[vx] as u16;
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            
            // FX29: Load font Character
            [0xF, _, 0x2, 0x9] => {
                let vx: usize = nibbles[1] as usize;
                if vx < 16 {
                    // Set I to the address of the font character
                    self.i = FONT_ADDRESS as u16 + (self.v[vx] as u16);
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // FX33: Binary-Coded decimal conversion
            [0xF, _, 0x3, 0x3] => {
                let vx: usize = nibbles[1] as usize;
                let val: u8 = self.v[vx];
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
                let vx: usize = nibbles[1] as usize;
                if vx < 16 {
                    for i in 0..=vx {
                        memory.data[(self.i) as usize] = self.v[i];
                        self.i += 1;
                    }
                    // CHIP-8 Quirk: We do not reset I to its original value after operation
                } else {
                    println!("Invalid register index: {}", vx);
                }
            }
            // FX65: Read registers V0 to VX from memory starting at address I
            [0xF, _, 0x6, 0x5] => {
                let vx: usize = nibbles[1] as usize;
                if vx < 16 {
                    for i in 0..=vx {
                        self.v[i] = memory.data[(self.i) as usize];
                        self.i += 1;
                    }
                    // CHIP-8 Quirk: We do not reset I to its original value after operation
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

