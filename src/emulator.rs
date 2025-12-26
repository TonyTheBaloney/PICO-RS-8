use std::time::Instant;

use crate::PixelBuffer;
use crate::{cpu::CPU, display::Display, memory::Memory};
use pixels::{Pixels, SurfaceTexture};
use tokio::sync::mpsc;
use winit::dpi::{PhysicalSize, Size};
use winit::event::{self, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowBuilder;

pub struct EmulatorData {
    pub file_content: mpsc::Receiver<Vec<u8>>,
    pub font_file_content: mpsc::Receiver<Vec<u8>>,
    pub frame_buffer_sender: mpsc::Sender<PixelBuffer>,
    pub keys: mpsc::Receiver<[bool; 16]>,
}

pub struct Emulator {
    pub cpu: CPU,
    pub memory: Memory,
    pub display: Display,
    pub keys: [bool; 16], // Keypad state
    pub emulator_data: EmulatorData,
    is_rom_loaded: bool,
}

const SCREEN_WIDTH: u32 = 64;
const SCREEN_HEIGHT: u32 = 32;

pub const FONT_ADDRESS: u16 = 0x050; // Address where fonts are stored in memory
pub const ROM_ADDRESS: u16 = 0x200; // Address where ROM is loaded in memory

const _CPU_FREQUENCY: u64 = 500; // CPU frequency in Hz

impl Emulator {
    pub fn new(emulator_data: EmulatorData) -> Self {
        let memory: Memory = Memory::new();
        let display: Display = Display::new(SCREEN_WIDTH, SCREEN_HEIGHT);
        let cpu: CPU = CPU::new(ROM_ADDRESS as u16);

        Emulator {
            cpu,
            memory,
            display,
            keys: [false; 16],
            emulator_data,
            is_rom_loaded: false,
        }
    }

    pub fn reset(&mut self) {
        self.cpu = CPU::new(ROM_ADDRESS as u16);
        self.memory.clear();
        self.display.clear();
        self.keys = [false; 16];
        self.is_rom_loaded = false;
    }

    pub fn set_font(&mut self, font: [u8; 80]) {
        for (i, &byte) in font.iter().enumerate() {
            self.memory.data[FONT_ADDRESS as usize + i] = byte;
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        self.is_rom_loaded = false;
        if rom.len() + ROM_ADDRESS as usize > self.memory.data.len() {
            // Return an error
            println!("ROM size exceeds available memory");

            return Err(Box::from("ROM size exceeds available memory"));
        }

        self.reset();

        for (i, &byte) in rom.iter().enumerate() {
            self.memory.data[ROM_ADDRESS as usize + i] = byte; // Load ROM starting at 0x200
        }
        println!("ROM loaded successfully, size: {} bytes", rom.len());
        self.is_rom_loaded = true;
        Ok(())
    }

    

    pub fn get_default_font() -> [u8; 80] {
        [
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ]
    }

    pub fn set_key(&mut self, key: u16, pressed: bool) {
        if key < 16 {
            self.keys[key as usize] = pressed;
        }
    }

    pub fn run(&mut self) {
        let cycle_duration = std::time::Duration::from_micros(2_000); // 500 Hz
        let mut last_cycle_time = Instant::now();

        loop {
            let now = Instant::now();
            if now.duration_since(last_cycle_time) >= cycle_duration {
                self.cycle();
                last_cycle_time = now;
            }
        }
    }

    pub fn cycle(&mut self) {
        if let Ok(rom_content) = self.emulator_data.file_content.try_recv() {
            let _ = self.load_rom(rom_content.as_slice());
        }

        if let Ok(font_content) = self.emulator_data.font_file_content.try_recv() {
            self.set_font(font_content.as_slice().try_into().unwrap());
        }

        if self.is_rom_loaded {
            if let Ok(keys) = self.emulator_data.keys.try_recv() {
                self.keys = keys;
            }

            self.cpu
                .decode(&mut self.memory, &mut self.display, &self.keys);

            let _: Result<(), mpsc::error::TrySendError<PixelBuffer>> = self
                .emulator_data
                .frame_buffer_sender
                .try_send(PixelBuffer {
                    pixels: self.display.pixels,
                });
        }
    }
}
