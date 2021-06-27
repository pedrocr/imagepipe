use crate::opbasics::*;
use std::f32::consts::FRAC_PI_2;

// Crops that are less than 1 pixel in a million are treated as no-ops
// Transforms that need more than 1:million magnification are broken and are
// thus also treated as no-ops
static EPSILON: f32 = 1.0 / 1000000.0;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct OpRotateCrop {
  pub crop_top: f32,
  pub crop_right: f32,
  pub crop_bottom: f32,
  pub crop_left: f32,
  pub rotation: f32,
  input_ratio: f32,
  output_size: Option<(usize, usize)>,
}

impl OpRotateCrop {
  pub fn new(_img: &ImageSource) -> Self {
    Self::empty()
  }
  pub fn empty() -> Self {
    Self {
      crop_top: 0.0,
      crop_right: 0.0,
      crop_bottom: 0.0,
      crop_left: 0.0,
      rotation: 0.0,
      input_ratio: 1.0,
      output_size: None,
    }
  }
}

impl<'a> ImageOp<'a> for OpRotateCrop {
  fn name(&self) -> &str {"rotatecrop"}
  fn run(&self, _pipeline: &PipelineGlobals, buf: Arc<OpBuffer>) -> Arc<OpBuffer> {
    if self.noop() { return buf; }

    let x = ((buf.width as f32) * self.crop_left).floor();
    if x < 0.0 || x > buf.width as f32 {
      log::error!("Trying to crop left outside image");
      return buf;
    }
    let y = ((buf.height as f32) * self.crop_top).floor();
    if y < 0.0 || y > buf.height as f32 {
      log::error!("Trying to crop top outside image");
      return buf;
    }
    let (width, height) = self.calc_size(buf.width, buf.height, false);
    if (width, height) == (buf.width, buf.height) { return buf; }
    let (x, y) = (x as isize, y as isize);
    let newbuffer = buf.transform((x,y), (x+width as isize-1, y), (x, y+height as isize-1), width, height);
    Arc::new(newbuffer)
  }

  fn transform_forward(&mut self, width: usize, height: usize) -> (usize, usize) {
    if let Some(size) = self.output_size {
      // We're going forward after going reverse so we're commited to an output size
      size
    } else {
      self.input_ratio = width as f32 / height as f32;
      self.calc_size(width, height, false)
    }
  }

  fn transform_reverse(&mut self, width: usize, height: usize) -> (usize, usize) {
    // Save the output size we're now commited to
    self.output_size = Some((width, height));
    self.calc_size(width, height, true)
  }
}

impl OpRotateCrop {
  fn noop(&self) -> bool {
    self.rotation.abs() < EPSILON &&
    self.crop_top.abs() < EPSILON &&
    self.crop_right.abs() < EPSILON &&
    self.crop_bottom.abs() < EPSILON &&
    self.crop_left.abs() < EPSILON
  }

  fn calc_size(&self, owidth: usize, oheight: usize, reverse: bool) -> (usize, usize){
    if self.noop() { return (owidth, oheight); }

    let (width, height) = (owidth as f32, oheight as f32);

    let (width, height) = if reverse || self.rotation < EPSILON {
      (width, height)
    } else {
      let angle = FRAC_PI_2 * if self.rotation > 1.0 {1.0} else {self.rotation};
      let (sin, cos) = angle.sin_cos();
      (width*cos + height*sin, width*sin + height*cos)
    };

    let nwidth = {
      let ratio = 1.0 - self.crop_left - self.crop_right;
      let nwidth = if reverse {
        (width / ratio).round()
      } else {
        (width * ratio).round()
      };
      if ratio < EPSILON || nwidth < 1.0 {
        log::error!("Trying to crop width beyond limits");
        return (owidth, oheight);
      }
      nwidth
    };

    let nheight = {
      let ratio = 1.0 - self.crop_top - self.crop_bottom;
      let nheight = if reverse {
        (height / ratio).round()
      } else {
        (height * ratio).round()
      };
      if ratio < EPSILON || nheight < 1.0 {
        log::error!("Trying to crop height beyond limits");
        return (owidth, oheight);
      }
      nheight
    };

    let (nwidth, nheight) = if !reverse || self.rotation < EPSILON {
      (nwidth, nheight)
    } else {
      let angle = FRAC_PI_2 * if self.rotation > 1.0 {1.0} else {self.rotation};
      let (sin, cos) = angle.sin_cos();
      let width = (nheight / (sin + (cos/self.input_ratio))).round();
      let height = (width / self.input_ratio).round();
      (width, height)
    };

    (nwidth as usize, nheight as usize)
  }
}

#[cfg(test)]
mod tests {
  use crate::buffer::OpBuffer;
  use super::*;

  fn setup() -> (Arc<OpBuffer>, OpRotateCrop, PipelineGlobals) {
    let mut buffer = OpBuffer::new(100, 100, 3, false);
    for (i, o) in buffer.data.chunks_exact_mut(1).enumerate() {
      o[0] = i as f32;
    }
    let op = OpRotateCrop::empty();
    (Arc::new(buffer), op, PipelineGlobals::mock(100, 100))
  }

  #[test]
  fn crop_top() {
    let (buffer, mut op, globals) = setup();
    op.crop_top = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 90);
    assert_eq!(newbuf.width, 100);
    assert_eq!(&newbuf.data[0], &buffer.data[100*10*3]);
  }

  #[test]
  fn crop_bottom() {
    let (buffer, mut op, globals) = setup();
    op.crop_bottom = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 90);
    assert_eq!(newbuf.width, 100);
    assert_eq!(&newbuf.data[0], &buffer.data[0]);
  }

  #[test]
  fn crop_vertical() {
    let (buffer, mut op, globals) = setup();
    op.crop_top = 0.1;
    op.crop_bottom = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 80);
    assert_eq!(newbuf.width, 100);
    assert_eq!(&newbuf.data[0], &buffer.data[100*10*3]);
  }

  #[test]
  fn crop_left() {
    let (buffer, mut op, globals) = setup();
    op.crop_left = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 100);
    assert_eq!(newbuf.width, 90);
    assert_eq!(&newbuf.data[0], &buffer.data[10*3]);
  }

  #[test]
  fn crop_right() {
    let (buffer, mut op, globals) = setup();
    op.crop_right = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 100);
    assert_eq!(newbuf.width, 90);
    assert_eq!(&newbuf.data[0], &buffer.data[0]);
  }

  #[test]
  fn crop_horizontal() {
    let (buffer, mut op, globals) = setup();
    op.crop_left = 0.1;
    op.crop_right = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 100);
    assert_eq!(newbuf.width, 80);
    assert_eq!(&newbuf.data[0], &buffer.data[10*3]);
  }

  #[test]
  fn crop_horizontal_and_vertical() {
    let (buffer, mut op, globals) = setup();
    op.crop_left = 0.1;
    op.crop_right = 0.1;
    op.crop_top = 0.1;
    op.crop_bottom = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 80);
    assert_eq!(newbuf.width, 80);
    assert_eq!(&newbuf.data[0], &buffer.data[100*10*3+10*3]);
  }

  #[test]
  fn roundtrip_transform() {
    let mut op = OpRotateCrop::empty();
    for dim in (0..10000).step_by(89) {
      for crop1 in (0..u16::MAX).step_by(97) {
        for crop2 in (0..u16::MAX).step_by(101) {
          op.crop_top = input16bit(crop1);
          op.crop_right = input16bit(crop1);
          op.crop_bottom = input16bit(crop2);
          op.crop_left = input16bit(crop2);
          let (width, height) = (dim as usize, dim as usize);
          // First reverse and then direct to make sure that if we promised we could
          // make a given output from a given input we then follow through exactly
          let intermediate = op.transform_reverse(width, height);
          let result = op.transform_forward(intermediate.0, intermediate.1);
          let from = (width, height);
          assert_eq!(result, from, "Got {:?}->{:?}->{:?} crops ({:.3}/{:.3}/{:.3}/{:.3})",
            from, intermediate, result, op.crop_top, op.crop_right, op.crop_bottom, op.crop_left);
        }
      }
    }
  }

  #[test]
  fn roundtrip_transform_rotation() {
    let mut op = OpRotateCrop::empty();
    for width in (0..10000).step_by(89) {
      for height in (0..10000).step_by(97) {
        for rotation in 0..u8::MAX {
          op.rotation = input8bit(rotation);
          let from = (width, height);
          let inter1 = op.transform_forward(from.0, from.1);
          let inter2 = op.transform_reverse(inter1.0, inter1.1);
          let result = op.transform_forward(inter2.0, inter2.1);
          assert_eq!(result, inter1, "Got {:?}->{:?}->{:?}->{:?} crops ({:.3}/{:.3}/{:.3}/{:.3})",
            from, inter1, inter2, result, op.crop_top, op.crop_right, op.crop_bottom, op.crop_left);
        }
      }
    }
  }
}
