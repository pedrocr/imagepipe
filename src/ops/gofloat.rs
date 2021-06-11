use crate::opbasics::*;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct OpGoFloat {
  pub crop_top: usize,
  pub crop_right: usize,
  pub crop_bottom: usize,
  pub crop_left: usize,
  pub is_cfa: bool,
  pub blacklevels: [f32;4],
  pub whitelevels: [f32;4],
}

fn from_int4(arr: [u16;4]) -> [f32;4] {
  [arr[0] as f32, arr[1] as f32, arr[2] as f32, arr[3] as f32]
}

impl OpGoFloat {
  pub fn new(img: &ImageSource) -> OpGoFloat {
    match img {
      ImageSource::Raw(img) => {
        // Calculate the resulting width/height and top-left corner after crops
        OpGoFloat{
          crop_top:    img.crops[0],
          crop_right:  img.crops[1],
          crop_bottom: img.crops[2],
          crop_left:   img.crops[3],
          is_cfa: img.cfa.is_valid(),
          blacklevels: from_int4(img.blacklevels),
          whitelevels: from_int4(img.whitelevels),
        }
      },
      ImageSource::Other(_) => {
        OpGoFloat{
          crop_top:    0,
          crop_right:  0,
          crop_bottom: 0,
          crop_left:   0,
          is_cfa: false,
          blacklevels: [0.0, 0.0, 0.0, 0.0], // Unused
          whitelevels: [0.0, 0.0, 0.0, 0.0], // Unused
        }
      }
    }
  }
}

impl<'a> ImageOp<'a> for OpGoFloat {
  fn name(&self) -> &str {"gofloat"}
  fn run(&self, pipeline: &PipelineGlobals, _buf: Arc<OpBuffer>) -> Arc<OpBuffer> {
    // FIXME: Doing all the transforms with lookup tables instead of f32 calcs
    //        on every pixel is much faster

    match &pipeline.image {
      ImageSource::Raw(img) => {
        self.run_raw(img)
      },
      ImageSource::Other(img) => {
        self.run_other(img)
      }
    }
  }

  fn transform_forward(&self, width: usize, height: usize) -> (usize, usize) {
    let (_, _, width, height) = self.size_image(width, height);
    (width, height)
  }

  // We don't transform_reverse as image sizing is relative to the scaling done
  // at the demosaic step
}

impl OpGoFloat {
  fn size_image(&self, owidth: usize, oheight: usize) -> (usize, usize, usize, usize) {
    // Calculate x/y/width/height making sure we get at least a 10x10 "image" to not trip up
    // reasonable assumptions in later ops
    let x = cmp::min(self.crop_left, owidth-10);
    let y = cmp::min(self.crop_top, oheight-10);
    let width = owidth - cmp::min(self.crop_left + self.crop_right, owidth-10);
    let height = oheight - cmp::min(self.crop_top + self.crop_bottom, oheight-10);
    (x, y, width, height)
  }

  fn run_raw(&self, img: &RawImage) -> Arc<OpBuffer> {
    // Calculate the levels
    let mins = self.blacklevels;
    let ranges = self.whitelevels.iter().enumerate().map(|(i, &x)| {
      x - mins[i]
    }).collect::<Vec<f32>>();

    let owidth = img.width;
    let oheight = img.height;
    let (x, y, width, height) = self.size_image(owidth, oheight);

    Arc::new(match img.data {
      RawImageData::Integer(ref data) => {
        if img.cpp == 1 && !self.is_cfa {
          // We're in a monochrome image so turn it into RGB
          let mut out = OpBuffer::new(width, height, 4, true);
          out.mutate_lines(&(|line: &mut [f32], row| {
            for (o, i) in line.chunks_exact_mut(4).zip(data[owidth*(row+y)+x..].chunks_exact(1)) {
              let val = ((i[0] as f32 - mins[0]) / ranges[0]).min(1.0);
              o[0] = val;
              o[1] = val;
              o[2] = val;
              o[3] = 0.0;
            }
          }));
          out
        } else if img.cpp == 3 {
          // We're in an RGB image, turn it into four channel
          let mut out = OpBuffer::new(width, height, 4, false);
          out.mutate_lines(&(|line: &mut [f32], row| {
            for (o, i) in line.chunks_exact_mut(4).zip(data[(owidth*(row+y)+x)*3..].chunks_exact(3)) {
              o[0] = ((i[0] as f32 - mins[0]) / ranges[0]).min(1.0);
              o[1] = ((i[1] as f32 - mins[1]) / ranges[1]).min(1.0);
              o[2] = ((i[2] as f32 - mins[2]) / ranges[2]).min(1.0);
              o[3] = 0.0;
            }
          }));
          out
        } else {
          let mut out = OpBuffer::new(width, height, img.cpp, false);
          out.mutate_lines(&(|line: &mut [f32], row| {
            for (o, i) in line.chunks_exact_mut(1).zip(data[owidth*(row+y)+x..].chunks_exact(1)) {
              o[0] = ((i[0] as f32 - mins[0]) / ranges[0]).min(1.0);
            }
          }));
          out
        }
      },
      RawImageData::Float(ref data) => {
        if img.cpp == 1 && !self.is_cfa {
          // We're in a monochrome image so turn it into RGB
          let mut out = OpBuffer::new(width, height, 4, true);
          out.mutate_lines(&(|line: &mut [f32], row| {
            for (o, i) in line.chunks_exact_mut(4).zip(data[owidth*(row+y)+x..].chunks_exact(1)) {
              let val = ((i[0] as f32 - mins[0]) / ranges[0]).min(1.0);
              o[0] = val;
              o[1] = val;
              o[2] = val;
              o[3] = 0.0;
            }
          }));
          out
        } else if img.cpp == 3 {
          // We're in an RGB image, turn it into four channel
          let mut out = OpBuffer::new(width, height, 4, false);
          out.mutate_lines(&(|line: &mut [f32], row| {
            for (o, i) in line.chunks_exact_mut(4).zip(data[(owidth*(row+y)+x)*3..].chunks_exact(3)) {
              o[0] = ((i[0] as f32 - mins[0]) / ranges[0]).min(1.0);
              o[1] = ((i[1] as f32 - mins[1]) / ranges[1]).min(1.0);
              o[2] = ((i[2] as f32 - mins[2]) / ranges[2]).min(1.0);
              o[3] = 0.0;
            }
          }));
          out
        } else {
          let mut out = OpBuffer::new(width, height, img.cpp, false);
          out.mutate_lines(&(|line: &mut [f32], row| {
            for (o, i) in line.chunks_exact_mut(1).zip(data[owidth*(row+y)+x..].chunks_exact(1)) {
              o[0] = ((i[0] as f32 - mins[0]) / ranges[0]).min(1.0);
            }
          }));
          out
        }
      },
    })
  }

  fn run_other(&self, img: &OtherImage) -> Arc<OpBuffer> {
    let owidth = img.width() as usize;
    let oheight = img.height() as usize;
    let (x, y, width, height) = self.size_image(owidth, oheight);
    let mut out = OpBuffer::new(width, height, 4, false);
    let bits_per_channel = img.color().bits_per_pixel() / img.color().channel_count() as u16;

    if bits_per_channel == 8 {
      let data = img.to_rgb8().into_raw();
      out.mutate_lines(&(|line: &mut [f32], row| {
        for (o, i) in line.chunks_exact_mut(4).zip(data[(owidth*(row+y)+x)*3..].chunks_exact(3)) {
          o[0] = expand_srgb_gamma(input8bit(i[0]));
          o[1] = expand_srgb_gamma(input8bit(i[1]));
          o[2] = expand_srgb_gamma(input8bit(i[2]));
          o[3] = 0.0;
        }
      }));
    } else {
      let data = img.to_rgb16().into_raw();
      out.mutate_lines(&(|line: &mut [f32], row| {
        for (o, i) in line.chunks_exact_mut(4).zip(data[(owidth*(row+y)+x)*3..].chunks_exact(3)) {
          o[0] = input16bit(i[0]);
          o[1] = input16bit(i[1]);
          o[2] = input16bit(i[2]);
          o[3] = 0.0;
        }
      }));
    }

    Arc::new(out)
  }
}
