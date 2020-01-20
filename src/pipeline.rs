use crate::ops::*;
use crate::opbasics::*;

extern crate rawloader;
extern crate multicache;
use self::multicache::MultiCache;
extern crate time;
extern crate serde;
extern crate serde_yaml;
use self::serde::{Serialize,Deserialize};

use std::fmt::Debug;
use std::sync::Arc;
use std::io::Write;
use std::path::Path;
use std::hash::{Hash, Hasher};

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

pub type PipelineCache = MultiCache<BufHash, OpBuffer>;

fn do_timing<O, F: FnMut() -> O>(name: &str, mut closure: F) -> O {
  let from_time = time::PrimitiveDateTime::now();
  let ret = closure();
  let to_time = time::PrimitiveDateTime::now();
  debug!("{} ms for '{}'", (to_time-from_time).whole_milliseconds(), name);

  ret
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
}

#[derive(Debug, Copy, Clone, Serialize)]
pub struct PipelineSettings {
  pub maxwidth: usize,
  pub maxheight: usize,
  pub linear: bool,
}

impl PipelineSettings{
  fn hash(&self, hasher: &mut BufHasher) {
    hasher.from_serialize(self);
  }
}

#[derive(Debug)]
pub struct PipelineGlobals {
  pub image: RawImage,
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

  pub fn new_from_file<P: AsRef<Path>>(path: P, maxwidth: usize, maxheight: usize, linear: bool) -> Result<Pipeline, String> {
    let img = rawloader::decode_file(path).map_err(|err| err.to_string())?;
    Self::new_from_rawimage(img, maxwidth, maxheight, linear)
  }

  pub fn new_from_rawimage(img: RawImage, maxwidth: usize, maxheight: usize, linear: bool) -> Result<Pipeline, String> {
    // Check if the image's orientation results in a rotation that
    // swaps the maximum width with the maximum height
    let (transpose, ..) = img.orientation.to_flips();
    let (maxwidth, maxheight) = if transpose {
      (maxheight, maxwidth)
    } else {
      (maxwidth, maxheight)
    };

    let ops = PipelineOps {
      gofloat: gofloat::OpGoFloat::new(&img),
      demosaic: demosaic::OpDemosaic::new(&img),
      tolab: colorspaces::OpToLab::new(&img),
      basecurve: curves::OpBaseCurve::new(&img),
      fromlab: colorspaces::OpFromLab::new(&img),
      gamma: gamma::OpGamma::new(&img),
      transform: transform::OpTransform::new(&img),
    };

    Ok(Pipeline {
      globals: PipelineGlobals {
        image: img,
        settings: PipelineSettings {maxwidth, maxheight, linear},
      },
      ops,
    })
  }

  pub fn to_serial(&self) -> String {
    let serial = (PipelineSerialization {
      version: 0,
      filehash: "0".to_string(),
    }, &self.ops);

    serde_yaml::to_string(&serial).unwrap()
  }

  pub fn new_from_serial(img: RawImage, maxwidth: usize, maxheight: usize, linear: bool, serial: String) -> Pipeline {
    let serial: (PipelineSerialization, PipelineOps) = serde_yaml::from_str(&serial).unwrap();

    Pipeline {
      globals: PipelineGlobals {
        image: img,
        settings: PipelineSettings {maxwidth, maxheight, linear},
      },
      ops: serial.1,
    }
  }

  pub fn run(&mut self, cache: Option<&PipelineCache>) -> Arc<OpBuffer> {
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
        bufin = do_timing(op.name(), ||op.run(&self.globals, bufin.clone()));
        if let Some(cache) = cache {
          cache.put_arc(ophashes[i], bufin.clone(), bufin.width*bufin.height*bufin.colors*4);
        }
      }
    });
    bufin
  }

  pub fn output_8bit(&mut self, cache: Option<&PipelineCache>) -> Result<SRGBImage, String> {
    let buffer = self.run(cache);
    let mut image = vec![0 as u8; buffer.width*buffer.height*3];
    for (o, i) in image.chunks_exact_mut(1).zip(buffer.data.iter()) {
      o[0] = (i*255.0).max(0.0).min(255.0) as u8;
    }

    Ok(SRGBImage{
      width: buffer.width,
      height: buffer.height,
      data: image,
    })
  }
}
