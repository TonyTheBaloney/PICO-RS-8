mod cpu;
mod display;
mod emulator;
mod memory;

use std::{error::Error, thread};

use eframe::egui::{self};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions::default();

    let frame_buffer_channel: (mpsc::Sender<PixelBuffer>, mpsc::Receiver<PixelBuffer>) = mpsc::channel::<PixelBuffer>(1);
    let keys_channel: (mpsc::Sender<[bool; 16]>, mpsc::Receiver<[bool; 16]>) = mpsc::channel::<[bool; 16]>(1);
    let rom_content_channel: (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) = mpsc::channel::<Vec<u8>>(1);
    let font_content_channel: (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) = mpsc::channel::<Vec<u8>>(1);

    let mut emulator: emulator::Emulator = emulator::Emulator::new(
        emulator::EmulatorData {
            file_content: rom_content_channel.1,
            font_file_content: font_content_channel.1,
            frame_buffer_sender: frame_buffer_channel.0,
            keys: keys_channel.1,
        }
    );

    thread::spawn(move || {
        // Load binary file
        emulator.emulator_data.file_content.blocking_recv().map(|rom_content| {
            // Load ROM into memory
            let _ = emulator.load_rom(rom_content.as_slice());
        });
        // Load font file
        emulator.emulator_data.font_file_content.blocking_recv().map(|font_content| {
            // Set font in memory
            emulator.set_font(font_content.as_slice().try_into().unwrap());
        });

        loop {
            emulator.cycle();
            // Sleep to control speed
            std::thread::sleep(std::time::Duration::from_micros(1200));
        }
    });

    eframe::run_native(
        "Rust Chip8 Emulator",
        options,
        Box::new(move |_cc| 
            Ok(Box::new(Pico8Emulator::new(frame_buffer_channel.1, keys_channel.0)))
        ),
    )?;
    Ok(())


    // // Load binary file
    // let filename: &'static str = "6-keypad.ch8";
    // let rom_content: Vec<u8> = fs::read(filename)?;
    // // Load ROM into memory
    // let _: () = emulator.load_rom(rom_content.as_slice())?;

    // emulator.run();
    // Ok(())
}

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

struct EmulatorData {
    file_content: Vec<u8>,
    font_file_content: Vec<u8>,
    frame_buffer_sender: mpsc::Sender<PixelBuffer>,
    keys: mpsc::Receiver<[bool; 16]>,
}

struct Pico8Emulator {
    selected_file: Option<String>,
    selected_font_file: Option<String>,
    requested_quit: bool,
    pixels: PixelBuffer,
    frame_buffer_receiver: mpsc::Receiver<PixelBuffer>,
    keys_sender: mpsc::Sender<[bool; 16]>,
}

impl Pico8Emulator {
    fn new(receiver: mpsc::Receiver<PixelBuffer>, keys_sender: mpsc::Sender<[bool; 16]>) -> Self {
        Pico8Emulator {
            selected_file: None,
            selected_font_file: None,
            requested_quit: false,
            pixels: PixelBuffer::default(),
            frame_buffer_receiver: receiver,
            keys_sender,
        }
    }
}

struct PixelBuffer {
    pixels: [[bool; WIDTH]; HEIGHT],
}

impl Default for PixelBuffer {
    fn default() -> Self {
        PixelBuffer {
            pixels: [[false; WIDTH]; HEIGHT],
        }
    }
}

impl PixelBuffer {
    pub fn clear(&mut self) {
        for row in self.pixels.iter_mut() {
            for pixel in row.iter_mut() {
                *pixel = false;
            }
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, value: bool) {
        if x < WIDTH && y < HEIGHT {
            self.pixels[y][x] = value;
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> bool {
        if x < WIDTH && y < HEIGHT {
            self.pixels[y][x]
        } else {
            false
        }
    }
}

impl eframe::App for Pico8Emulator {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        if self.requested_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // This creates the Menu Bar on the top of the window
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        if let Some(file_path) = rfd::FileDialog::new().pick_file() {
                            self.selected_file = Some(file_path.display().to_string());
                            ctx.request_repaint();
                        }
                        ui.close();
                    }

                    if ui.button("Font File").clicked() {
                        if let Some(font_path) = rfd::FileDialog::new().pick_file() {
                            self.selected_font_file = Some(font_path.display().to_string());
                            ctx.request_repaint();
                        }
                        ui.close();
                    }
                    if ui.button("Exit").clicked() {
                        // Close the application
                        self.requested_quit = true;
                    }
                });
            });
        });

        // This is the main screen
        egui::CentralPanel::default().show(ctx, |ui: &mut egui::Ui| {
            if let Some(_selected_file) = self.selected_file.as_ref() {
                // If we have a selected file, there's probbaly something to display
                // Render the latest frame
                while let Ok(frame_buffer) = self.frame_buffer_receiver.try_recv() {
                    self.pixels = frame_buffer;
                    ctx.request_repaint();
                }

                // Get pixels from the pixel buffer
                let pixels = &self.pixels.pixels;
                // Get the dimensions of the window
                let window_size = ui.available_size();

                // Scale the pixel buffer to fit the window while maintaining aspect ratio
                let scale_x = window_size.x / (WIDTH as f32);
                let scale_y = window_size.y / (HEIGHT as f32);
                let scale = scale_x.min(scale_y);

                // Create the Pixel Grid 
                let total_size =
                    egui::Vec2::new(pixels[0].len() as f32 * scale, pixels.len() as f32 * scale);
                let area = ui.allocate_space(total_size);
                let painter = ui.painter_at(area.1);
                painter.rect_filled(area.1, 0.0, egui::Color32::from_gray(10));

                // Draw pixels
                for y in 0..pixels.len() {
                    for x in 0..pixels[0].len() {
                        if pixels[y][x] {
                            let min = egui::Pos2::new(
                                area.1.min.x + x as f32 * scale,
                                area.1.min.y + y as f32 * scale,
                            );
                            let max = egui::Pos2::new(min.x + scale, min.y + scale);
                            painter.rect_filled(
                                egui::Rect::from_min_max(min, max),
                                0.0,
                                egui::Color32::WHITE,
                            );
                        }
                    }
                }
            } else {
                ui.heading("Pico8 Emulator");

                if ui.button("Pick a file").clicked() {
                    if let Some(file_path) = rfd::FileDialog::new().pick_file() {
                        self.selected_file = Some(file_path.display().to_string());
                        ctx.request_repaint();
                    }
                }
            }
        });
    }
}
