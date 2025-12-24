use std::time::{Instant};

use crate::{cpu::CPU, display::Display, memory::Memory};
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::{PhysicalSize, Size};
use winit::event::{self, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{WindowBuilder};

pub struct Emulator {
    pub cpu: CPU,
    pub memory: Memory,
    pub display: Display,
    pub keys: [bool; 16], // Keypad state
}

const SCREEN_WIDTH: u32 = 64;
const SCREEN_HEIGHT: u32 = 32;

pub const FONT_ADDRESS: u16 = 0x050; // Address where fonts are stored in memory
pub const ROM_ADDRESS: u16 = 0x200; // Address where ROM is loaded in memory

const _CPU_FREQUENCY: u64 = 500; // CPU frequency in Hz


impl Emulator {
    pub fn new() -> Self {
        let memory: Memory = Memory::new();
        let display: Display = Display::new(SCREEN_WIDTH, SCREEN_HEIGHT);
        let cpu: CPU = CPU::new(ROM_ADDRESS as u16);

        Emulator { cpu, memory, display, keys: [false; 16] }
    }

    pub fn set_font(&mut self, font: [u8; 80]) {
        for (i, &byte) in font.iter().enumerate() {
            self.memory.data[FONT_ADDRESS as usize + i] = byte;
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if rom.len() + ROM_ADDRESS as usize > self.memory.data.len() {
            // Return an error
            println!("ROM size exceeds available memory");

            return Err(Box::from("ROM size exceeds available memory"));
        }
        for (i, &byte) in rom.iter().enumerate() {
            self.memory.data[ROM_ADDRESS as usize + i] = byte; // Load ROM starting at 0x200
        }
        Ok(())
    }

    pub fn get_font() -> [u8; 80]{
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
            0xF0, 0x80, 0xF0, 0x80, 0x80  // F
        ]
    }

    pub fn set_key(&mut self, key: u16, pressed: bool) {
        if key < 16 {
            self.keys[key as usize] = pressed;
        }
    }

    pub fn run(&mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

        let scale: u32 = 30;
        
        let window = WindowBuilder::new()
            .with_title("Chip-8 Emulator")
            .with_inner_size(Size::new(Size::Physical(PhysicalSize::new(SCREEN_WIDTH * scale, SCREEN_HEIGHT * scale))))
            .build(&event_loop)
            .unwrap();

        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let mut pixels: Pixels = pixels::Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture).unwrap();
        let frame_time = std::time::Duration::from_millis(1000 / 700); // Target 60 FPS


        let res = event_loop.run( |event: event::Event<()>, elwt: &winit::event_loop::EventLoopWindowTarget<()>| {
            elwt.set_control_flow(ControlFlow::WaitUntil(Instant::now() + frame_time));
            match event {
                event::Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                    let _ = pixels.resize_surface(size.width, size.height);
                }
                event::Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    println!("The close button was pressed; stopping");
                    elwt.exit();
                }
                event::Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                    // Frame loop: only redraw the display
                    self.cycle();

                    self.display.convert_to_buf(&mut pixels);
                    pixels.render().unwrap();
                    
                    window.request_redraw(); // Request the next frame
                }
                event::Event::WindowEvent { event: WindowEvent::KeyboardInput { event, .. }, .. } => { 
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Digit1) => self.set_key(0x1, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::Digit2) => self.set_key(0x2, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::Digit3) => self.set_key(0x3, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::Digit4) => self.set_key(0x4, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::KeyQ) => self.set_key(0x5, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::KeyW) => self.set_key(0x6, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::KeyE) => self.set_key(0x7, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::KeyR) => self.set_key(0x8, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::KeyA) => self.set_key(0x9, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::KeyS) => self.set_key(0xA, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::KeyD) => self.set_key(0xB, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::KeyF) => self.set_key(0xC, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::KeyZ) => self.set_key(0xD, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::KeyX) => self.set_key(0xE, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::KeyC) => self.set_key(0xF, event.state == event::ElementState::Pressed),
                        PhysicalKey::Code(KeyCode::Escape) => elwt.exit(),
                        _ => (),
                    }
                    
                }
                
                _ => (),
            }
        });
        if let Err(result) = res {
            println!("Error running event loop: {}", result);
            return;
        }

    }


    pub fn cycle(&mut self) {
        self.cpu.decode(&mut self.memory, &mut self.display, &self.keys);
    }
}