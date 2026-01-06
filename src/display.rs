const WIDTH: usize = 64;
const HEIGHT: usize = 32;
use crate::PixelBuffer;

pub struct Display {
    pub width: u32,
    pub height: u32,
    pub pixels: [[bool; WIDTH as usize]; HEIGHT as usize],
    pub pixel_buffer_sender: tokio::sync::mpsc::Sender<PixelBuffer>,
}

impl Display {
    pub fn new(
        width: u32,
        height: u32,
        pixel_buffer_sender: tokio::sync::mpsc::Sender<PixelBuffer>,
    ) -> Self {
        Display {
            width,
            height,
            pixels: [[false; WIDTH as usize]; HEIGHT as usize],
            pixel_buffer_sender,
        }
    }
    pub fn clear(&mut self) {
        for row in self.pixels.iter_mut() {
            for pixel in row.iter_mut() {
                *pixel = false;
            }
        }
    }

    pub fn draw_sprite(&mut self, x: usize, y: usize, n: usize, sprite: &[u8]) -> bool {
        let mut pixel_erased = false;
        for row in 0..n {
            if row > self.height as usize {
                break;
            }
            // Get the nth byte of sprite data counting from the memory address in the I register
            let sprite_byte: u8 = sprite[row];

            for col in 0..8 {
                // Check if the pixel is set at that col in the sprite byte
                let pixel: bool = ((sprite_byte >> (7 - col)) & 0x01) == 1;
                if col > self.width as usize {
                    break;
                }
                // Sprites are XORed onto the existing screen.
                let display_x: usize = (x + col) % self.width as usize;
                let display_y: usize = (y + row) % self.height as usize;
                // If this causes any pixels to be erased, VF is set to 1, otherwise it is set to 0.
                if self.pixels[display_y][display_x] && pixel {
                    pixel_erased = true;
                }
                // XOR the pixel
                self.pixels[display_y][display_x] ^= pixel;
            }
        }
        self.pixel_buffer_sender.blocking_send(PixelBuffer { pixels: self.pixels }).unwrap();
        pixel_erased
    }
}
