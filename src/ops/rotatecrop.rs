use crate::opbasics::*;

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
}

impl OpRotateCrop {
  pub fn new(_img: &ImageSource) -> Self {
    Self {
      crop_top: 0.0,
      crop_right: 0.0,
      crop_bottom: 0.0,
      crop_left: 0.0,
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
    let (width, height) = self.transform_forward(buf.width, buf.height);
    if (width, height) == (buf.width, buf.height) { return buf; }
    let (x, y) = (x as usize, y as usize);
    let mut newbuffer = OpBuffer::new(width, height, buf.colors, buf.monochrome);
    newbuffer.mutate_lines(&(|line: &mut [f32], row| {
      for (o, i) in line.chunks_exact_mut(buf.colors)
        .zip(buf.data[(buf.width*(row+y)+x)*buf.colors..].chunks_exact(buf.colors)) {
        o.copy_from_slice(i);
      }
    }));
    Arc::new(newbuffer)
  }

  fn transform_forward(&self, width: usize, height: usize) -> (usize, usize) {
    self.calc_size(width, height, false)
  }

  fn transform_reverse(&self, width: usize, height: usize) -> (usize, usize) {
    self.calc_size(width, height, true)
  }
}

impl OpRotateCrop {
  fn noop(&self) -> bool {
    self.crop_top.abs() < EPSILON &&
    self.crop_right.abs() < EPSILON &&
    self.crop_bottom.abs() < EPSILON &&
    self.crop_left.abs() < EPSILON
  }

  fn calc_size(&self, width: usize, height: usize, reverse: bool) -> (usize, usize){
    if self.noop() { return (width, height); }

    let nwidth = {
      let ratio = 1.0 - self.crop_left - self.crop_right;
      let nwidth = if reverse {
        (width as f32 / ratio).round()
      } else {
        (width as f32 * ratio).round()
      };
      if ratio < EPSILON || nwidth < 1.0 {
        log::error!("Trying to crop width beyond limits");
        return (width, height);
      }
      nwidth as usize
    };

    let nheight = {
      let ratio = 1.0 - self.crop_top - self.crop_bottom;
      let nheight = if reverse {
        (height as f32 / ratio).round()
      } else {
        (height as f32 * ratio).round()
      };
      if ratio < EPSILON || nheight < 1.0 {
        log::error!("Trying to crop height beyond limits");
        return (width, height);
      }
      nheight as usize
    };

    (nwidth, nheight)
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
    let op = OpRotateCrop {
      crop_top: 0.0,
      crop_right: 0.0,
      crop_bottom: 0.0,
      crop_left: 0.0,
    };
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
    let mut op = OpRotateCrop {
      crop_top: 0.0,
      crop_right: 0.0,
      crop_bottom: 0.0,
      crop_left: 0.0,
    };
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
}
