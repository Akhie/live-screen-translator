use tauri::Manager;
use scrap::{Capturer, Display};
use image::{ImageBuffer, RgbaImage, ImageOutputFormat};
use std::io::Cursor;
use std::thread::sleep;
use std::time::Duration;

/// Capture a rectangular area of the primary screen and return PNG bytes
#[tauri::command]
fn capture_area(x: u32, y: u32, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let display = Display::primary().map_err(|e| e.to_string())?;
    let mut capturer = Capturer::new(display).map_err(|e| e.to_string())?;

    let screen_width = capturer.width() as u32;
    let screen_height = capturer.height() as u32;

    if x + width > screen_width || y + height > screen_height {
        return Err("Selection area is out of screen bounds".into());
    }

    // Capture a frame
    let frame = loop {
        match capturer.frame() {
            Ok(buf) => break buf.to_vec(),
            Err(_) => {
                // In scrap 0.5, any error just means retry
                sleep(Duration::from_millis(10));
                continue;
            }
        }
    };

    // BGRA -> RGBA
    let mut rgba: Vec<u8> = Vec::with_capacity((screen_width * screen_height * 4) as usize);
    for chunk in frame.chunks(4) {
        rgba.push(chunk[2]); // R
        rgba.push(chunk[1]); // G
        rgba.push(chunk[0]); // B
        rgba.push(chunk[3]); // A
    }

    // Crop selected rectangle manually
    let mut cropped_buf = Vec::with_capacity((width * height * 4) as usize);
    for row in y..(y + height) {
        let start = (row * screen_width + x) * 4;
        let end = start + width * 4;
        cropped_buf.extend_from_slice(&rgba[start as usize..end as usize]);
    }

    let img: RgbaImage = ImageBuffer::from_vec(width, height, cropped_buf)
        .ok_or("Failed to create image buffer")?;

    // Encode as PNG
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), ImageOutputFormat::Png)
        .map_err(|e| e.to_string())?;

    Ok(bytes)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![capture_area])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}