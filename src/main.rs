mod cpu;
mod display;
mod emulator;
mod memory;

use std::{error::Error, path::PathBuf, thread};

use eframe::egui::{self};
use tokio::sync::mpsc;

use crate::emulator::Emulator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Rust Chip8 Emulator",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(Pico8Emulator::new()))
        }),
    )?;
    Ok(())
}

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

struct Pico8Emulator {
    selected_file: Option<String>,
    selected_font_file: Option<String>,
    requested_quit: bool,
    pixels: PixelBuffer,
    frame_buffer_receiver: mpsc::Receiver<PixelBuffer>,
    file_content_sender: mpsc::Sender<Vec<u8>>,
    font_file_content_sender: mpsc::Sender<Vec<u8>>,
    keys_sender: mpsc::Sender<[bool; 16]>,
    emulator_thread: thread::JoinHandle<()>,
}

impl Drop for Pico8Emulator {
    fn drop(&mut self) {
        if self.emulator_thread.is_finished() == false {
            // If the thread is still running, we should probably do something to stop it
            // For now, we'll just detach it
            self.emulator_thread.thread().unpark();
        } 
        eprintln!("Pico8Emulator DROPPED");
    }
}

impl Pico8Emulator {
    fn new(
    ) -> Self {
        let frame_buffer_channel: (mpsc::Sender<PixelBuffer>, mpsc::Receiver<PixelBuffer>) =
            mpsc::channel::<PixelBuffer>(1);
        let keys_channel: (mpsc::Sender<[bool; 16]>, mpsc::Receiver<[bool; 16]>) =
            mpsc::channel::<[bool; 16]>(1);
        let rom_content_channel: (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
            mpsc::channel::<Vec<u8>>(1);
        let font_content_channel: (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
            mpsc::channel::<Vec<u8>>(1);


        let mut emulator: emulator::Emulator = emulator::Emulator::new(emulator::EmulatorData {
            file_content: rom_content_channel.1,
            font_file_content: font_content_channel.1,
            frame_buffer_sender: frame_buffer_channel.0,
            keys: keys_channel.1,
        });
        emulator.set_font(Emulator::get_default_font());
        

        let emulator_thread: thread::JoinHandle<()> = thread::spawn(move || {
            loop {
                emulator.cycle();
                // Thread sleeping until we want to FPS sleep again
            }
        });

        Pico8Emulator {
            selected_file: None,
            selected_font_file: None,
            requested_quit: false,
            pixels: PixelBuffer::default(),
            frame_buffer_receiver: frame_buffer_channel.1,
            keys_sender: keys_channel.0,
            file_content_sender: rom_content_channel.0,
            font_file_content_sender: font_content_channel.0,
            emulator_thread: emulator_thread,
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
            self.emulator_thread.thread().unpark();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // This creates the Menu Bar on the top of the window
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        if let Some(file_path) = rfd::FileDialog::new().pick_file() {
                            self.rom_selected(file_path);
                            ctx.request_repaint();
                        }
                        ui.close();
                    }

                    if ui.button("Font File").clicked() {
                        if let Some(font_path) = rfd::FileDialog::new().pick_file() {
                            self.selected_font_file = Some(font_path.display().to_string());
                            // Read the font file content
                            let font_file_content = std::fs::read(&font_path).unwrap_or_default();
                            // Send the font file content to the emulator
                            let _ = self.font_file_content_sender.try_send(font_file_content);
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

                // Get keys
                let mut keys: [bool; 16] = [false; 16];
                for i in 0..16 {
                    let key_code = match i {
                        0x0 => egui::Key::Num0,
                        0x1 => egui::Key::Num1,
                        0x2 => egui::Key::Num2,
                        0x3 => egui::Key::Num3,
                        0x4 => egui::Key::Num4,
                        0x5 => egui::Key::Num5,
                        0x6 => egui::Key::Num6,
                        0x7 => egui::Key::Num7,
                        0x8 => egui::Key::Num8,
                        0x9 => egui::Key::Num9,
                        0xA => egui::Key::A,
                        0xB => egui::Key::B,
                        0xC => egui::Key::C,
                        0xD => egui::Key::D,
                        0xE => egui::Key::E,
                        0xF => egui::Key::F,
                        _ => continue,
                    };

                    if ui.input(|i| i.key_pressed(key_code)) {
                        keys[i] = true;
                    }else {
                        keys[i] = false;
                    }
                }
                let _ = self.keys_sender.try_send(keys);
            } else {
                ui.heading("Pico8 Emulator");

                if ui.button("Pick a file").clicked() {
                    if let Some(file_path) = rfd::FileDialog::new().pick_file() {
                        self.rom_selected(file_path);
                        ctx.request_repaint();
                    }
                }
            }
        });
        ctx.request_repaint();
    }
}

impl Pico8Emulator {
    pub fn rom_selected(&mut self, file_path: PathBuf) {
        self.selected_file = None;
        println!("Selected file: {}", file_path.display());
        let selected_file: String = file_path.display().to_string();
        

        // Read the file content
        let file_content: Vec<u8> = std::fs::read(&file_path).unwrap_or_default();
        // Send the file content to the emulator
        let err: Result<(), mpsc::error::TrySendError<Vec<u8>>> =
            self.file_content_sender.try_send(file_content);
        
        if err.is_err() {
            println!("Error sending file content to emulator");
        }else {
            self.selected_file = Some(selected_file);
        }
    }
}
