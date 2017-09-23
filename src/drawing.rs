// Various utility functions to assist with drawing debug windows.
// All functions depend on a Piston::ImageBuffer as one of their inputs, and
// draw directly into the buffer.

use image::ImageBuffer;
use image::Rgba;
use std::collections::HashMap;

struct CharacterAttributes {
  x: u32,
  width: u32
}

struct BitmapFont {
  pub raw_buffer: ImageBuffer<Rgba<u8>, Vec<u8>>,
  pub char_attributes: HashMap<char, CharacterAttributes>,
}

pub fn draw_region(destination: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, source: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, dx: u32, dy: u32, sx: u32, sy: u32, width: u32, height: u32) 
  for x in 0 .. width {
    for y in 0 .. height {
      let pixel = source.get_pixel(sx + x, sy + y);
      // Very simple index-based alpha transparency
      if pixel[3] != 0 {
        destination.put_pixel(dx + x, dy + y, source.get_pixel(sx + x, sx + y));
      }
    }
  }
}

