#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui_extras::RetainedImage;
use std::{
    env,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
};
#[cfg(windows)]
use std::error::Error;
#[cfg(windows)]
use winreg::{enums::HKEY_CURRENT_USER, RegKey};
use flate2::Compression;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;

const NXPF_MAGIC: &[u8; 4] = b"NXPF";
const TEMP_RESULT_PATH: &str = "temp.png";

// Flags byte structure
const FLAG_HAS_ALPHA: u8 = 0x01;
const FLAG_COMPRESSED: u8 = 0x02;

// Compression types
const COMPRESSION_NONE: u8 = 0x00;
const COMPRESSION_DEFLATE: u8 = 0x01;

fn png_to_nxpf(path: PathBuf, use_compression: bool) -> Result<(), std::io::Error> {
    let img = image::open(&path).expect("File not found!");
    let width = img.width();
    let height = img.height();

    // Convert to RGBA
    let rgba_img = img.to_rgba8();
    let mut pixel_data = Vec::new();

    // Collect all pixel bytes
    for pixel in rgba_img.pixels() {
        pixel_data.push(pixel[0]); // R
        pixel_data.push(pixel[1]); // G
        pixel_data.push(pixel[2]); // B
        pixel_data.push(pixel[3]); // A
    }

    if let Some(path_str) = path.to_str() {
        let output_path = path_str.replace(".png", ".nxpf");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&output_path)
            .expect("Couldn't create file");

        // Write magic number
        file.write_all(NXPF_MAGIC)?;

        // Write width (u32 little-endian)
        file.write_all(&width.to_le_bytes())?;

        // Write height (u32 little-endian)
        file.write_all(&height.to_le_bytes())?;

        // Write flags (has alpha = true)
        let flags = FLAG_HAS_ALPHA | if use_compression { FLAG_COMPRESSED } else { 0 };
        file.write_all(&[flags])?;

        // Write compression type
        let compression_type = if use_compression { COMPRESSION_DEFLATE } else { COMPRESSION_NONE };
        file.write_all(&[compression_type])?;

        // Write pixel data
        if use_compression {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&pixel_data)?;
            let compressed = encoder.finish()?;
            file.write_all(&compressed)?;
        } else {
            file.write_all(&pixel_data)?;
        }

        file.flush()?;
        println!("Successfully converted PNG to NXPF: {}", output_path);
        println!("Original size: {} bytes", pixel_data.len());
        
        Ok(())
    } else {
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Invalid path"))
    }
}

fn nxpf_to_png(path: PathBuf) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(&path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    if buffer.len() < 14 {
        return Err("File too small".into());
    }

    // Read magic
    if &buffer[0..4] != NXPF_MAGIC {
        return Err("Invalid magic number".into());
    }

    // Read dimensions
    let width = u32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
    let height = u32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);

    // Read flags and compression
    let flags = buffer[12];
    let _compression_type = buffer[13];

    let _has_alpha = (flags & FLAG_HAS_ALPHA) != 0;
    let is_compressed = (flags & FLAG_COMPRESSED) != 0;

    // Extract pixel data
    let pixel_data = if is_compressed {
        let mut decoder = GzDecoder::new(&buffer[14..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        decompressed
    } else {
        buffer[14..].to_vec()
    };

    // Validate data size
    let expected_size = (width * height * 4) as usize;
    if pixel_data.len() != expected_size {
        return Err(format!("Invalid pixel data size: {} vs {}", pixel_data.len(), expected_size).into());
    }

    // Create image buffer
    let mut img_buffer = image::RgbaImage::new(width, height);
    for (i, chunk) in pixel_data.chunks(4).enumerate() {
        let x = (i as u32) % width;
        let y = (i as u32) / width;
        img_buffer.put_pixel(x, y, image::Rgba([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }

    // Save as PNG
    img_buffer.save(TEMP_RESULT_PATH)?;

    Ok((width, height))
}

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: nxpf [compile|view|register] <path>");
        eprintln!("  compile <path.png>  - Convert PNG to NXPF");
        eprintln!("  view <path.nxpf>    - View NXPF file");
        eprintln!("  register            - Register .nxpf file association on Windows (HKCU)");
        return Ok(());
    }

    // Allow registering file association: `nxpf register`
    if args[1] == "register" {
        #[cfg(windows)]
        {
            match register_file_association() {
                Ok(()) => println!("Successfully registered .nxpf file association for current user."),
                Err(e) => eprintln!("Registration failed: {}", e),
            }
        }

        #[cfg(not(windows))]
        {
            eprintln!("Registration is supported only on Windows.");
        }

        return Ok(());
    }

    let command = &args[1];

    if command == "compile" {
        if args.len() < 3 {
            panic!("Path not provided. Example: `cargo run compile image.png`");
        }

        let path: PathBuf = args[2].clone().into();
        let use_compression = args.contains(&"--compress".to_string());

        match png_to_nxpf(path, use_compression) {
            Ok(()) => println!("Conversion successful!"),
            Err(e) => eprintln!("Conversion failed: {}", e),
        }

        Ok(())
    } else {
        let file_path: PathBuf = command.into();

        match nxpf_to_png(file_path) {
            Ok((width, height)) => {
                let options = eframe::NativeOptions {
                    resizable: true,
                    // allow user to resize window; start at image size
                    initial_window_size: Some(egui::vec2(width as f32, height as f32)),
                    ..Default::default()
                };

                eframe::run_native(
                    "NXPF Viewer",
                    options,
                    Box::new(|_cc| Box::new(ImagePreview::default())),
                )
            }
            Err(e) => {
                eprintln!("Failed to load NXPF: {}", e);
                Ok(())
            }
        }
    }
}

struct ImagePreview {
    image: RetainedImage,
    zoom: f32,
}

impl Default for ImagePreview {
    fn default() -> Self {
        let image_data = std::fs::read(TEMP_RESULT_PATH).expect("Failed to read image file");
        let _ = fs::remove_file(TEMP_RESULT_PATH);

        Self {
            image: RetainedImage::from_image_bytes(TEMP_RESULT_PATH, &image_data).unwrap(),
            zoom: 1.0,
        }
    }
}

impl eframe::App for ImagePreview {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("-").clicked() {
                    self.zoom = (self.zoom / 1.1).clamp(0.1, 10.0);
                }
                ui.add(egui::Slider::new(&mut self.zoom, 0.1..=10.0).show_value(true));
                if ui.button("+").clicked() {
                    self.zoom = (self.zoom * 1.1).clamp(0.1, 10.0);
                }
                if ui.button("Fit").clicked() {
                    self.zoom = 1.0;
                }
            });

            let tex = self.image.texture_id(ctx);
            let size = self.image.size();
            let size_vec = egui::vec2(size[0] as f32, size[1] as f32);

            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                let scaled = size_vec * self.zoom;
                let image_resp = ui.add(egui::Image::new(tex, scaled));

                // Zoom with mouse wheel while hovering image
                if image_resp.hovered() {
                    let scroll = ctx.input(|i| i.scroll_delta).y;
                    if scroll != 0.0 {
                        // positive scroll -> zoom in
                        let factor = 1.0 + scroll * 0.1;
                        self.zoom = (self.zoom * factor).clamp(0.1, 10.0);
                    }
                }
            });
        });
    }
}

#[cfg(windows)]
fn register_file_association() -> Result<(), Box<dyn Error>> {
    // Use HKCU so admin rights are not required. Associates .nxpf files with this exe.
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // Set the file extension default value to a ProgID
    let (ext, _) = hkcu.create_subkey("Software\\Classes\\.nxpf")?;
    ext.set_value("", &"nxpf_file")?;

    // Create ProgID key and set display name
    let (prog, _) = hkcu.create_subkey("Software\\Classes\\nxpf_file")?;
    prog.set_value("", &"NXPF File")?;

    // Set the open command to this executable
    let exe_path = std::env::current_exe()?;
    let cmd = format!("\"{}\" \"%1\"", exe_path.display());
    let (shell, _) = prog.create_subkey("shell")?;
    let (open, _) = shell.create_subkey("open")?;
    let (command, _) = open.create_subkey("command")?;
    command.set_value("", &cmd)?;

    Ok(())
}
