use pixels::Pixels;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;


pub struct Display {
    pub width: u32,
    pub height: u32,
    pub pixels: [[bool; WIDTH as usize]; HEIGHT as usize],
}

impl Display {
    pub fn new(width: u32, height: u32) -> Self {
        Display { width, height, pixels: [[false; WIDTH as usize]; HEIGHT as usize] }
    }
    pub fn clear(&mut self) {
        for row in self.pixels.iter_mut() {
            for pixel in row.iter_mut() {
                *pixel = false;
            }
        }
    }
    pub fn convert_to_buf(&self, render_buffer: &mut Pixels) {
        let pixels = render_buffer.frame_mut();
        for (y, row) in self.pixels.iter().enumerate() {
            for (x, &pixel) in row.iter().enumerate() {
                let index = (y * self.width as usize + x) * 4;
                let color = if pixel { [0xFF, 0xFF, 0xFF, 0xFF] } else { [0x00, 0x00, 0x00, 0xFF] };
                pixels[index..index + 4].copy_from_slice(&color);
            }
        }        
    }
}

