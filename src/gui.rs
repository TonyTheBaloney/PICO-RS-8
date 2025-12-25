use pixels::SurfaceTexture;
use winit::dpi::{PhysicalSize, Size};
use winit::event::{self, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{WindowBuilder};

const SCREEN_WIDTH: u32 = 64;
const SCREEN_HEIGHT: u32 = 32;
const DEFAULT_SCALE: u32 = 30;
const EXPECTED_FPS: u64 = 60;

pub struct GUI {
    width: u32,
    height: u32,
    event_loop: EventLoop<()>,
    window: winit::window::Window,
    pixels: [[bool; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize]
}



impl GUI {
    pub fn new() -> Self {
        let event_loop: EventLoop<()> = EventLoop::new().unwrap();

        
        let window: winit::window::Window = WindowBuilder::new()
            .with_title("Rust Chip8 Emulator")
            .with_inner_size(Size::Physical(PhysicalSize::new(
                SCREEN_WIDTH * DEFAULT_SCALE,
                SCREEN_HEIGHT * DEFAULT_SCALE,
            )))
            .build(&event_loop)
            .unwrap();

        let window_size: PhysicalSize<u32> = window.inner_size();

        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels: pixels::Pixels = pixels::Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture).unwrap();
        let frame_time = std::time::Duration::from_millis(1000 / EXPECTED_FPS); // Target 60 FPS
        
        

        GUI {
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            event_loop,
            window,
            pixels: [[false; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
        }
    }    
}