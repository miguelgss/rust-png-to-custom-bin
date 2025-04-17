use image::{GenericImageView, Rgba};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

const LIMIT_COLORS: u8 = 4;
// Convert PNGs to our custom format with just 2 bits per pixel (4 colors + transparency)
fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 3 {
        println!("Usage: {} <input.png> <output.bin>", args[0]);
        return;
    }
    
    let input_path = &args[1];
    let output_path = &args[2];
    
    match convert_png_to_custom(input_path, output_path) {
        Ok(size) => println!("Converted successfully! Output size: {} bytes", size),
        Err(e) => println!("Error: {}", e),
    }
}

fn convert_png_to_custom(input_path: &str, output_path: &str) -> Result<usize, Box<dyn std::error::Error>> {
    // Load the PNG image
    let img = image::open(&Path::new(input_path))?;
    let (width, height) = img.dimensions();
    
    // Create palette mapping - we'll only use the first 4 colors encountered
    let mut palette = HashMap::new();
    palette.insert(Rgba([0, 0, 0, 0]), 0u8); // Transparent is always 0
    let mut next_color_id = 1;
    
    // First pass: build our palette
    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            
            // Skip if already in palette
            if palette.contains_key(&pixel) {
                continue;
            }
            
            // Add to palette if we have space
            if next_color_id < LIMIT_COLORS {
                palette.insert(pixel, next_color_id);
                next_color_id += 1;
            } else {
                // More than 4 colors found! Return error or quantize
                return Err(format!("Image has more than 4 colors at position ({}, {})", x, y).into());
            }
        }
    }
    
    // Open output file
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);
    
    // Write simple header: width, height
    writer.write_all(&[width as u8, height as u8])?;
    
    // Write palette entries for reconstruction (except transparent)
    for color_id in 1..next_color_id {
        let color = palette.iter()
            .find(|(_, &id)| id == color_id)
            .map(|(rgba, _)| rgba)
            .unwrap();
        
        writer.write_all(&[color[0], color[1], color[2]])?;
    }
    
    // Write the pixel data, packing 4 pixels into each byte
    let mut current_byte = 0u8;
    let mut bit_position = 0;
    
    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let color_id = *palette.get(&pixel).unwrap_or(&0);
            
            // Pack this pixel into our current byte
            current_byte |= (color_id & 0b11) << bit_position;
            bit_position += 2;
            
            // When we've packed 4 pixels (8 bits), write the byte
            if bit_position == 8 {
                writer.write_all(&[current_byte])?;
                current_byte = 0;
                bit_position = 0;
            }
        }
    }
    
    // Write the final byte if needed
    if bit_position > 0 {
        writer.write_all(&[current_byte])?;
    }
    
    writer.flush()?;
    
    // Return the file size
    Ok(2 + (next_color_id as usize - 1) * 3 + ((width * height + 3) / 4) as usize)
}
