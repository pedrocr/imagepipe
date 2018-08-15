#[macro_use] extern crate serde_derive;
#[macro_use] extern crate lazy_static;
extern crate rawloader;
extern crate image;

mod buffer;
use self::buffer::*;
mod hasher;
mod ops;
mod opbasics;
mod pipeline;
pub use self::pipeline::*;
pub use self::ops::*;

use std::sync::Arc;
use std::path::Path;

fn simple_decode_full<P: AsRef<Path>>(img: P, maxwidth: usize, maxheight: usize, linear: bool) -> Result<OpBuffer, String> {
  let img = try!(rawloader::decode_file(img).map_err(|err| err.to_string()));

  let buf = {
    let mut pipeline = Pipeline::new(&img, maxwidth, maxheight, linear);
    // FIXME: turn these into tests
    //
    // --- Check if serialization roundtrips
    // let serial = pipeline.to_serial();
    // println!("Settings are: {}", serial);
    // pipeline = Pipeline::new_from_serial(img, maxwidth, maxheight, linear, serial);
    //
    // --- Check that the pipeline caches buffers and does not recalculate
    // pipeline.run();
    pipeline.run()
  };

  // Since we've kept the pipeline to ourselves unwraping always works
  Ok(Arc::try_unwrap(buf).unwrap())
}


/// A RawImage processed into a full 8bit sRGB image with levels and gamma
///
/// The data is a Vec<u8> width width*height*3 elements, where each element is a value
/// between 0 and 255 with the intensity of the color channel with gamma applied
#[derive(Debug, Clone)]
pub struct SRGBImage {
  pub width: usize,
  pub height: usize,
  pub data: Vec<u8>,
}

pub fn simple_decode<P: AsRef<Path>>(img: P, maxwidth: usize, maxheight: usize) -> Result<OpBuffer, String> {
  simple_decode_full(img, maxwidth, maxheight, false)
}

pub fn simple_decode_linear<P: AsRef<Path>>(img: P, maxwidth: usize, maxheight: usize) -> Result<OpBuffer, String> {
  simple_decode_full(img, maxwidth, maxheight, true)
}

pub fn simple_decode_8bit<P: AsRef<Path>>(img: P, maxwidth: usize, maxheight: usize) -> Result<SRGBImage, String> {
  if let Ok(buffer) = simple_decode(&img, maxwidth, maxheight) {
    let mut image = vec![0 as u8; buffer.width*buffer.height*3];
    for (o, i) in image.chunks_mut(1).zip(buffer.data.iter()) {
      o[0] = (i*255.0).max(0.0).min(255.0) as u8;
    }

    return Ok(SRGBImage{
      width: buffer.width,
      height: buffer.height,
      data: image,
    })
  }

  if let Ok(img) = image::open(&img) {
    let rgb = img.to_rgb();
    let width = rgb.width() as usize;
    let height = rgb.height() as usize;
    return Ok(SRGBImage {
      data: rgb.into_raw(),
      width: width,
      height: height,
    })
  }

  Err("Don't know how to load this image".to_string())
}
