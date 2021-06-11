use crate::ops::*;
use crate::opbasics::*;

extern crate rawloader;
extern crate multicache;
use self::multicache::MultiCache;
extern crate serde;
extern crate serde_yaml;
use self::serde::{Serialize,Deserialize};

extern crate image;
use image::DynamicImage;

use std::fmt::Debug;
use std::sync::Arc;
use std::io::Write;
use std::path::Path;
use std::hash::{Hash, Hasher};
use std::time::Instant;

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

/// A RawImage processed into a full 16bit sRGB image with levels and gamma
///
/// The data is a Vec<u16> width width*height*3 elements, where each element is a value
/// between 0 and 65535 with the intensity of the color channel with gamma applied
#[derive(Debug, Clone)]
pub struct SRGBImage16 {
  pub width: usize,
  pub height: usize,
  pub data: Vec<u16>,
}

pub type PipelineCache = MultiCache<BufHash, OpBuffer>;
pub type OtherImage = DynamicImage;

#[derive(Debug, Clone)]
pub enum ImageSource {
  Raw(RawImage),
  Other(OtherImage),
}

impl ImageSource {
  fn width(&self) -> usize {
    match self {
      Self::Raw(raw) => raw.width,
      Self::Other(img) => img.width() as usize,
    }
  }

  fn height(&self) -> usize {
    match self {
      Self::Raw(raw) => raw.height,
      Self::Other(img) => img.height() as usize,
    }
  }
}

macro_rules! do_timing {
  ($name:expr, $body:expr) => {
    {
      let from_time = Instant::now();
      let ret = {
        $body
      };
      let duration = from_time.elapsed();
      info!("timing: {:>7} ms for |{}", duration.as_millis(), $name);
      ret
    }
  }
}

pub trait ImageOp<'a>: Debug+Serialize+Deserialize<'a> {
  fn name(&self) -> &str;
  fn run(&self, pipeline: &PipelineGlobals, buf: Arc<OpBuffer>) -> Arc<OpBuffer>;
  fn to_settings(&self) -> String {
    serde_yaml::to_string(self).unwrap()
  }
  fn hash(&self, hasher: &mut BufHasher) {
    // Hash the name first as a zero sized struct doesn't actually do any hashing
    hasher.write(self.name().as_bytes()).unwrap();
    hasher.from_serialize(self);
  }
  // What size is the output the operation creates given its input
  fn transform_forward(&self, width: usize, height: usize) -> (usize, usize) {
    (width, height)
  }
  // What size is the input the operation needs to create a given output
  fn transform_reverse(&self, width: usize, height: usize) -> (usize, usize) {
    (width, height)
  }
}

#[derive(Debug, Copy, Clone, Serialize)]
pub struct PipelineSettings {
  pub maxwidth: usize,
  pub maxheight: usize,
  pub demosaic_width: usize,
  pub demosaic_height: usize,
  pub linear: bool,
  pub use_fastpath: bool,
}

impl PipelineSettings {
  fn default() -> Self {
    Self {
      maxwidth: 0,
      maxheight: 0,
      demosaic_width: 0,
      demosaic_height: 0,
      linear: false,
      use_fastpath: true,
    }
  }
}

impl PipelineSettings{
  fn hash(&self, hasher: &mut BufHasher) {
    hasher.from_serialize(self);
  }
}

#[derive(Debug)]
pub struct PipelineGlobals {
  pub image: ImageSource,
  pub settings: PipelineSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOps {
  pub gofloat: gofloat::OpGoFloat,
  pub demosaic: demosaic::OpDemosaic,
  pub tolab: colorspaces::OpToLab,
  pub basecurve: curves::OpBaseCurve,
  pub fromlab: colorspaces::OpFromLab,
  pub gamma: gamma::OpGamma,
  pub transform: transform::OpTransform,
}

impl PipelineOps {
  fn new(img: &ImageSource) -> Self {
    Self {
      gofloat: gofloat::OpGoFloat::new(&img),
      demosaic: demosaic::OpDemosaic::new(&img),
      tolab: colorspaces::OpToLab::new(&img),
      basecurve: curves::OpBaseCurve::new(&img),
      fromlab: colorspaces::OpFromLab::new(&img),
      gamma: gamma::OpGamma::new(&img),
      transform: transform::OpTransform::new(&img),
    }
  }
}

impl PartialEq for PipelineOps {
  fn eq(&self, other: &Self) -> bool {
    let mut selfhasher = BufHasher::new();
    selfhasher.from_serialize(self);
    let mut otherhasher = BufHasher::new();
    otherhasher.from_serialize(other);
    selfhasher.result() == otherhasher.result()
  }
}
impl Eq for PipelineOps {}
impl Hash for PipelineOps {
  fn hash<H: Hasher>(&self, state: &mut H) {
    let mut selfhasher = BufHasher::new();
    selfhasher.from_serialize(self);
    selfhasher.result().hash(state);
  }
}

macro_rules! for_vals {
  ([$($val:expr),*] |$x:pat, $i:ident| $body:expr) => {
    let mut pos = 0;
    $({
      let $x = $val;
      pos += 1;
      let $i = pos-1;
      $body
    })*
  }
}

macro_rules! all_ops {
  ($ops:expr, |$x:pat, $i:ident| $body:expr) => {
    for_vals!([
      $ops.gofloat,
      $ops.demosaic,
      $ops.tolab,
      $ops.basecurve,
      $ops.fromlab,
      $ops.gamma,
      $ops.transform
    ] |$x, $i| {
      $body
    });
  }
}

macro_rules! all_ops_reverse {
  ($ops:expr, |$x:pat, $i:ident| $body:expr) => {
    for_vals!([
      $ops.transform,
      $ops.gamma,
      $ops.fromlab,
      $ops.basecurve,
      $ops.tolab,
      $ops.demosaic,
      $ops.gofloat
    ] |$x, $i| {
      $body
    });
  }
}

#[derive(Debug)]
pub struct Pipeline {
  pub globals: PipelineGlobals,
  pub ops: PipelineOps,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineSerialization {
  pub version: u32,
  pub filehash: String,
}

impl Pipeline {
  pub fn new_cache(size: usize) -> PipelineCache {
    MultiCache::new(size)
  }

  pub fn new_from_file<P: AsRef<Path>>(path: P) -> Result<Pipeline, String> {
    do_timing!("total new_from_file()", {
    if let Ok(img) = do_timing!("  rawloader", rawloader::decode_file(&path)) {
      Self::new_from_source(ImageSource::Raw(img))
    } else if let Ok(img) = do_timing!("  image::open", image::open(&path)) {
      Self::new_from_source(ImageSource::Other(img))
    } else {
      Err("imagepipe: Don't know how to decode image".to_string())
    }
    })
  }

  pub fn new_from_source(img: ImageSource) -> Result<Pipeline, String> {
    let ops = PipelineOps::new(&img);

    Ok(Pipeline {
      globals: PipelineGlobals {
        image: img,
        settings: PipelineSettings::default(),
      },
      ops,
    })
  }

  pub fn default_ops(&self) -> bool {
    self.ops == PipelineOps::new(&self.globals.image)
  }

  pub fn to_serial(&self) -> String {
    let serial = (PipelineSerialization {
      version: 0,
      filehash: "0".to_string(),
    }, &self.ops);

    serde_yaml::to_string(&serial).unwrap()
  }

  pub fn new_from_serial(img: ImageSource, serial: String) -> Pipeline {
    let serial: (PipelineSerialization, PipelineOps) = serde_yaml::from_str(&serial).unwrap();

    Pipeline {
      globals: PipelineGlobals {
        image: img,
        settings: PipelineSettings::default(),
      },
      ops: serial.1,
    }
  }

  pub fn run(&mut self, cache: Option<&PipelineCache>) -> Arc<OpBuffer> {
    do_timing!("  total pipeline", {
    // Calculate what size of image we should scale down to at the demosaic stage
    let mut width = self.globals.image.width();
    let mut height = self.globals.image.height();
    all_ops!(self.ops, |ref op, _i| {
      let (w, h) = op.transform_forward(width, height);
      width = w;
      height = h;
    });
    log::debug!("Maximum possible image size is {}x{}", width, height);
    let maxwidth = self.globals.settings.maxwidth;
    let maxheight = self.globals.settings.maxheight;
    let (_, mut width, mut height) =
      crate::scaling::calculate_scaling(width, height, maxwidth, maxheight);
    all_ops_reverse!(self.ops, |ref op, _i| {
      let (w, h) = op.transform_reverse(width, height);
      width = w;
      height = h;
    });
    log::debug!("Final image size is {}x{}", width, height);
    self.globals.settings.demosaic_width = width;
    self.globals.settings.demosaic_height = height;

    // Generate all the hashes for the operations
    let mut hasher = BufHasher::new();
    let mut ophashes = Vec::new();
    let mut startpos = 0;
    // Hash the base settings that are potentially used by all operations
    self.globals.settings.hash(&mut hasher);
    // Start with a dummy buffer as gofloat doesn't use it
    let mut bufin = Arc::new(OpBuffer::default());
    // Find the hashes of all ops
    all_ops!(self.ops, |ref op, i| {
      op.hash(&mut hasher);
      let result = hasher.result();
      ophashes.push(result);

      // Set the latest op for which we already have the calculated buffer
      if let Some(cache) = cache {
        if let Some(buffer) = cache.get(&result) {
          bufin = buffer;
          startpos = i+1;
        }
      }
    });

    // Do the operations, starting for the last we have a cached buffer for
    all_ops!(self.ops, |ref op, i| {
      if i >= startpos {
        let opstr = "    ".to_string() + op.name();
        bufin = do_timing!(&opstr, op.run(&self.globals, bufin.clone()));
        if let Some(cache) = cache {
          cache.put_arc(ophashes[i], bufin.clone(), bufin.width*bufin.height*bufin.colors*4);
        }
      }
    });
    bufin
    })
  }

  pub fn output_8bit(&mut self, cache: Option<&PipelineCache>) -> Result<SRGBImage, String> {
    // If the image is raster and we haven't changed it yet there's no need to go
    // through the whole pipeline. Just go straight to 8bit using the image
    // crate and resize if needed
    if let ImageSource::Other(ref image) = self.globals.image {
      if self.globals.settings.use_fastpath && self.default_ops() {
        return Ok(do_timing!("total output_8bit_fastpath()", {
        let rgb = image.to_rgb8();
        let (width, height) = (rgb.width() as usize, rgb.height() as usize);
        let out = SRGBImage{
          width,
          height,
          data: rgb.into_raw(),
        };
        let (_, nwidth, nheight) = crate::scaling::calculate_scaling(
          out.width, out.height,
          self.globals.settings.maxwidth, self.globals.settings.maxheight
        );
        if nwidth != out.width || nheight != out.height {
          crate::scaling::scale_down_srgb(&out, nwidth, nheight)
        } else {
          out
        }
        }))
      }
    }

    do_timing!("total output_8bit()", {
    self.globals.settings.linear = false;
    let buffer = self.run(cache);

    let image = do_timing!("  8 bit conversion", {
      let mut image = vec![0 as u8; buffer.width*buffer.height*3];
      for (o, i) in image.chunks_exact_mut(1).zip(buffer.data.iter()) {
        o[0] = output8bit(*i);
      }
      image
    });

    Ok(SRGBImage{
      width: buffer.width,
      height: buffer.height,
      data: image,
    })
    })
  }

  pub fn output_16bit(&mut self, cache: Option<&PipelineCache>) -> Result<SRGBImage16, String> {
    // If the image is raster and we haven't changed it yet there's no need to go
    // through the whole pipeline. Just go straight to 16bit using the image
    // crate and resize if needed
    if let ImageSource::Other(ref image) = self.globals.image {
      if self.globals.settings.use_fastpath && self.default_ops() {
        return Ok(do_timing!("total output_16bit_fastpath()", {
        let rgb = image.to_rgb16();
        let (width, height) = (rgb.width() as usize, rgb.height() as usize);
        let out = SRGBImage16{
          width,
          height,
          data: rgb.into_raw(),
        };
        let (_, nwidth, nheight) = crate::scaling::calculate_scaling(
          out.width, out.height,
          self.globals.settings.maxwidth, self.globals.settings.maxheight
        );
        if nwidth != out.width || nheight != out.height {
          crate::scaling::scale_down_srgb16(&out, nwidth, nheight)
        } else {
          out
        }
        }))
      }
    }

    do_timing!("total output_16bit()", {
    self.globals.settings.linear = true;
    let buffer = self.run(cache);

    let image = do_timing!("  8 bit conversion", {
      let mut image = vec![0 as u16; buffer.width*buffer.height*3];
      for (o, i) in image.chunks_exact_mut(1).zip(buffer.data.iter()) {
        o[0] = output16bit(*i);
      }
      image
    });

    Ok(SRGBImage16{
      width: buffer.width,
      height: buffer.height,
      data: image,
    })
    })
  }
}
