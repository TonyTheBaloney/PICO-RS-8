mod memory;
mod display;
mod cpu;
mod emulator;

use std::error::Error;
use std::fs;

use crate::emulator::Emulator;

fn main() {
    let mut emulator: emulator::Emulator = emulator::Emulator::new();

    // Set the font in memory
    emulator.set_font(Emulator::get_font());

    // Load binary file
    let filename: &'static str = "6-keypad.ch8";
    let contents: Result<Vec<u8>, std::io::Error> = fs::read(filename);
    if contents.is_err() {
        println!("Error reading file: {}", contents.unwrap_err());
        return;
    }
    let rom: Vec<u8> = contents.unwrap();
    // Load ROM into memory
    let result: Result<(), Box<dyn Error>> = emulator.load_rom(rom.as_slice());
    if let Err(err) = result {
        println!("Error loading ROM: {}", err);
        return;
    }
    
    emulator.run();

}

