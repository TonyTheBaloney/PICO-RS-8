
const RAM_SIZE: usize = 4 * 1024; // 4 KB

pub struct Memory {
    pub data: [u8; RAM_SIZE],
}
impl Memory {
    pub fn new() -> Self {
        Memory {
            data: [0; RAM_SIZE],
        }
    }
    pub fn clear(&mut self) {
        for byte in self.data.iter_mut() {
            *byte = 0;
        }
    }
}