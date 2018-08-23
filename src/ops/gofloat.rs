use opbasics::*;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct OpGoFloat {
  pub crop_top: usize,
  pub crop_right: usize,
  pub crop_bottom: usize,
  pub crop_left: usize,
  pub is_cfa: bool,
}

impl OpGoFloat {
  pub fn new(img: &RawImage) -> OpGoFloat {
    // Calculate the resulting width/height and top-left corner after crops
    OpGoFloat{
      crop_top:    img.crops[0],
      crop_right:  img.crops[1],
      crop_bottom: img.crops[2],
      crop_left:   img.crops[3],
      is_cfa: img.cfa.is_valid(),
    }
  }
}

impl<'a> ImageOp<'a> for OpGoFloat {
  fn name(&self) -> &str {"gofloat"}
  fn run(&self, pipeline: &PipelineGlobals, _buf: Arc<OpBuffer>) -> Arc<OpBuffer> {
    let img = &pipeline.image;

    // Calculate x/y/width/height making sure we get at least a 10x10 "image" to not trip up
    // reasonable assumptions in later ops
    let x = cmp::min(self.crop_left, img.width-10);
    let y = cmp::min(self.crop_top, img.height-10);
    let width = img.width - cmp::min(self.crop_left + self.crop_right, img.width-10);
    let height = img.height - cmp::min(self.crop_top + self.crop_bottom, img.height-10);

    Arc::new(match img.data {
      RawImageData::Integer(ref data) => {
        if img.cpp == 1 && !self.is_cfa {
          // We're in a monochrome image so turn it into RGB
          let mut out = OpBuffer::new(width, height, 4, true);
          out.mutate_lines(&(|line: &mut [f32], row| {
            for (o, i) in line.chunks_mut(4).zip(data[img.width*(row+y)+x..].chunks(1)) {
              o[0] = i[0] as f32;
              o[1] = i[0] as f32;
              o[2] = i[0] as f32;
              o[3] = 0.0;
            }
          }));
          out
        } else if img.cpp == 3 {
          // We're in an RGB image, turn it into four channel
          let mut out = OpBuffer::new(width, height, 4, false);
          out.mutate_lines(&(|line: &mut [f32], row| {
            for (o, i) in line.chunks_mut(4).zip(data[(img.width*(row+y)+x)*3..].chunks(3)) {
              o[0] = i[0] as f32;
              o[1] = i[1] as f32;
              o[2] = i[2] as f32;
              o[3] = 0.0;
            }
          }));
          out
        } else {
          let mut out = OpBuffer::new(width, height, img.cpp, false);
          out.mutate_lines(&(|line: &mut [f32], row| {
            for (o, i) in line.chunks_mut(1).zip(data[img.width*(row+y)+x..].chunks(1)) {
              o[0] = i[0] as f32;
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
            for (o, i) in line.chunks_mut(4).zip(data[img.width*(row+y)+x..].chunks(1)) {
              o[0] = i[0];
              o[1] = i[0];
              o[2] = i[0];
              o[3] = 0.0;
            }
          }));
          out
        } else if img.cpp == 3 {
          // We're in an RGB image, turn it into four channel
          let mut out = OpBuffer::new(width, height, 4, false);
          out.mutate_lines(&(|line: &mut [f32], row| {
            for (o, i) in line.chunks_mut(4).zip(data[(img.width*(row+y)+x)*3..].chunks(3)) {
              o[0] = i[0];
              o[1] = i[1];
              o[2] = i[2];
              o[3] = 0.0;
            }
          }));
          out
        } else {
          let mut out = OpBuffer::new(width, height, img.cpp, false);
          out.mutate_lines(&(|line: &mut [f32], row| {
            for (o, i) in line.chunks_mut(1).zip(data[img.width*(row+y)+x..].chunks(1)) {
              o[0] = i[0];
            }
          }));
          out
        }
      },
    })
  }
}
