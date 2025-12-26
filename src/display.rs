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
}

