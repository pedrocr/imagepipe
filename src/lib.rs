#[macro_use] extern crate serde_derive;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate rawloader;

mod buffer;
mod hasher;
mod ops;
pub use ops::transform::Rotation;
mod opbasics;
mod pipeline;
pub use self::pipeline::*;
pub use self::ops::*;

use std::path::Path;

pub fn simple_decode_8bit<P: AsRef<Path>>(img: P, maxwidth: usize, maxheight: usize) -> Result<SRGBImage, String> {
  if let Ok(mut pipe) = Pipeline::new_from_file(&img, maxwidth, maxheight, false) {
    if let Ok(img) = pipe.output_8bit(None) {
      return Ok(img)
    }
  }

  #[cfg(feature = "image")]
  {
    if let Ok(img) = image::open(&img) {
        let rgb = img.to_rgb();
        let width = rgb.width() as usize;
        let height = rgb.height() as usize;
        return Ok(SRGBImage {
            data: rgb.into_raw(),
            width: width,
            height: height,
        });
    }
  }

  Err("Don't know how to load this image".to_string())
}
