#[macro_use] extern crate serde_derive;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate rawloader;
extern crate image;

mod buffer;
mod hasher;
mod ops;
pub use ops::transform::Rotation;
mod opbasics;
mod pipeline;
pub use self::pipeline::*;
pub use self::ops::*;
pub mod color_conversions;

use std::path::Path;

pub fn simple_decode_8bit<P: AsRef<Path>>(img: P, maxwidth: usize, maxheight: usize) -> Result<SRGBImage, String> {
  Pipeline::new_from_file(&img, maxwidth, maxheight, false)?.output_8bit(None)
}
