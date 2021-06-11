use imagepipe::{Pipeline, ImageSource};
use image::{ImageBuffer, DynamicImage};

fn roundtrip_8bit(fast: bool) {
  // Create a source with all possibilities of u8 (R,G,B) pixels 
  let mut image_data: Vec<u8> = Vec::with_capacity(256 * 256 * 256 * 3);
  for r in 0..=u8::MAX {
    for g in 0..=u8::MAX {
      for b in 0..=u8::MAX {
        image_data.push(r);
        image_data.push(g);
        image_data.push(b);
      }
    }
  }
  let image = ImageBuffer::from_raw(4096, 4096, image_data.clone()).unwrap();
  let source = ImageSource::Other(DynamicImage::ImageRgb8(image));
  let mut pipeline = Pipeline::new_from_source(source).unwrap();
  pipeline.globals.settings.use_fastpath = fast;
  let decoded = pipeline.output_8bit(None).unwrap();
  
  for (pixin, pixout) in image_data.chunks_exact(3).zip(decoded.data.chunks_exact(3)) {
    assert_eq!(pixout, pixin);
  }
}

#[test]
fn roundtrip_8bit_fastpath() {
  roundtrip_8bit(true);
}

#[test]
fn roundtrip_8bit_slowpath() {
  roundtrip_8bit(false);
}

fn roundtrip_16bit(fast: bool) {
  let mut start = (0,0,0);
  loop {
    // Create a source with a bunch of possibilities of u16 (R,G,B) pixels
    // We need to do this in blocks or we'd end up allocating huge buffers
    let mut image_data: Vec<u16> = vec![0; 256 * 256 * 256 * 3];
    let mut pos = 0;
    let mut newstart = None;
    'outer: for r in (start.0..=u16::MAX).step_by(89) {
      for g in (start.1..=u16::MAX).step_by(97) {
        for b in (start.2..=u16::MAX).step_by(101) {
          if pos >= image_data.len() {
            newstart = Some((r,g,b));
            break 'outer
          }
          image_data[pos+0] = r;
          image_data[pos+1] = g;
          image_data[pos+2] = b;
          pos += 3;
        }
      }
    }
    let image = ImageBuffer::from_raw(4096, 4096, image_data.clone()).unwrap();
    let source = ImageSource::Other(DynamicImage::ImageRgb16(image));
    let mut pipeline = Pipeline::new_from_source(source).unwrap();
    pipeline.globals.settings.use_fastpath = fast;
    let decoded = pipeline.output_16bit(None).unwrap();

    for (pixin, pixout) in image_data.chunks_exact(3).zip(decoded.data.chunks_exact(3)) {
      assert_eq!(pixout, pixin);
    }
    if let Some(newstart) = newstart {
      start = newstart;
    } else {
      break;
    }
  }
}

#[test]
fn roundtrip_16bit_fastpath() {
  roundtrip_16bit(true);
}

#[test]
fn roundtrip_16bit_slowpath() {
  roundtrip_16bit(false);
}
