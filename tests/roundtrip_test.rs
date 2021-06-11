use imagepipe::{Pipeline, ImageSource};
use image::{ImageBuffer, DynamicImage};

use num_traits::ops::saturating::Saturating;
use std::fmt::Debug;
use std::cmp::PartialOrd;
#[inline(always)]
fn assert_offby<T>(to: &[T], from: &[T], offdown: T, offup: T)
  where T: Saturating+Debug+PartialOrd+Copy {
  let condition =
    to[0] <= from[0].saturating_add(offup) && to[0] >= from[0].saturating_sub(offdown) &&
    to[1] <= from[1].saturating_add(offup) && to[1] >= from[1].saturating_sub(offdown) &&
    to[2] <= from[2].saturating_add(offup) && to[2] >= from[2].saturating_sub(offdown);
  if !condition {
    eprintln!("Got {:?} instead of {:?}", to, from);
  }
  assert!(condition)
}

fn pipeline_8bit() -> (Vec<u8>, Pipeline) {
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
  let pipeline = Pipeline::new_from_source(source, 0, 0, false).unwrap();

  (image_data, pipeline)
}

#[test]
fn roundtrip_8bit_fastpath() {
  let (image_data, mut pipeline) = pipeline_8bit();

  pipeline.globals.settings.use_fastpath = true;
  let decoded = pipeline.output_8bit(None).unwrap();
  
  for (pixin, pixout) in image_data.chunks_exact(3).zip(decoded.data.chunks_exact(3)) {
    assert_eq!(pixout, pixin);
  }
}

// FIXME: The total pipeline is rountripping 8bit off-by-one, even though the
//        pipeline as a single function roundtrips exactly in color_conversions
#[test]
fn roundtrip_8bit_slowpath() {
  let (image_data, mut pipeline) = pipeline_8bit();

  pipeline.globals.settings.use_fastpath = false;
  let decoded = pipeline.output_8bit(None).unwrap();
  
  for (pixin, pixout) in image_data.chunks_exact(3).zip(decoded.data.chunks_exact(3)) {
    assert_offby(pixout, pixin, 1, 0);
  }
}
