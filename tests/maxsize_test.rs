use imagepipe::{Pipeline, ImageSource, Rotation};
use image::{RgbImage, DynamicImage};

fn create_pipeline() -> Pipeline {
  let source = ImageSource::Other(DynamicImage::ImageRgb8(RgbImage::new(128, 64)));
  Pipeline::new_from_source(source).unwrap()
}

fn assert_width(pipeline: &mut Pipeline, width: usize, height: usize) {
  let decoded = pipeline.output_8bit(None).unwrap();
  pipeline.globals.settings.use_fastpath = true;
  assert_eq!(decoded.width, width);
  assert_eq!(decoded.height, height);

  let decoded = pipeline.output_8bit(None).unwrap();
  pipeline.globals.settings.use_fastpath = false;
  assert_eq!(decoded.width, width);
  assert_eq!(decoded.height, height);

  let decoded = pipeline.output_16bit(None).unwrap();
  pipeline.globals.settings.use_fastpath = true;
  assert_eq!(decoded.width, width);
  assert_eq!(decoded.height, height);

  let decoded = pipeline.output_16bit(None).unwrap();
  pipeline.globals.settings.use_fastpath = false;
  assert_eq!(decoded.width, width);
  assert_eq!(decoded.height, height);
}

#[test]
fn default_same_size() {
  let mut pipeline = create_pipeline();
  assert_width(&mut pipeline, 128, 64);
}

#[test]
fn no_upscaling() {
  let mut pipeline = create_pipeline();
  pipeline.globals.settings.maxwidth = 256;
  pipeline.globals.settings.maxwidth = 128;
  assert_width(&mut pipeline, 128, 64);
}

#[test]
fn downscale_keeps_ratio() {
  let mut pipeline = create_pipeline();
  pipeline.globals.settings.maxwidth = 64;
  assert_width(&mut pipeline, 64, 32);
}

#[test]
fn rotation_works() {
  let mut pipeline = create_pipeline();
  pipeline.globals.settings.maxwidth = 64;
  pipeline.ops.transform.rotation = Rotation::Rotate90;
  assert_width(&mut pipeline, 64, 128);

  let mut pipeline = create_pipeline();
  pipeline.globals.settings.maxwidth = 32;
  pipeline.ops.transform.rotation = Rotation::Rotate90;
  assert_width(&mut pipeline, 32, 64);

  let mut pipeline = create_pipeline();
  pipeline.globals.settings.maxwidth = 256;
  pipeline.ops.transform.rotation = Rotation::Rotate90;
  assert_width(&mut pipeline, 64, 128);
}
